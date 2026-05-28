//! Native Maven dependency resolver for jbx.
//!
//! Resolves `//DEPS` coordinates against Maven repositories without requiring
//! Coursier on PATH. Implements a fixpoint resolution algorithm similar to
//! Coursier: fetch POMs, extract transitive deps, reconcile version conflicts,
//! repeat until stable.

use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

// ─── Data Model ───────────────────────────────────────────────────────────────

/// Maven module identifier (groupId:artifactId).
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Module {
    pub org: String,
    pub name: String,
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.org, self.name)
    }
}

/// Sanitize a string for safe use as a filesystem path segment.
/// Replaces path separators and `..` to prevent directory traversal.
fn sanitize_path_segment(s: &str) -> String {
    s.replace("..", "_dotdot_").replace(['/', '\\', '\0'], "_")
}

/// Maven dependency scope.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Scope {
    Compile,
    Runtime,
    Provided,
    Test,
    System,
    Import,
}

impl Scope {
    pub fn parse_scope(s: &str) -> Scope {
        match s {
            "runtime" => Scope::Runtime,
            "provided" => Scope::Provided,
            "test" => Scope::Test,
            "system" => Scope::System,
            "import" => Scope::Import,
            _ => Scope::Compile, // default per Maven spec
        }
    }

    /// Whether this scope should be included when resolving the runtime classpath.
    pub fn is_in_classpath(&self) -> bool {
        matches!(self, Scope::Compile | Scope::Runtime)
    }
}

/// Exclusion rule — matches against a module's (org, name).
/// `"*"` acts as wildcard.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Exclusion {
    pub org: String,
    pub name: String,
}

impl Exclusion {
    pub fn matches(&self, module: &Module) -> bool {
        (self.org == "*" || self.org == module.org)
            && (self.name == "*" || self.name == module.name)
    }
}

/// A dependency declaration.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub module: Module,
    pub version: String,
    pub scope: Scope,
    pub optional: bool,
    pub exclusions: HashSet<Exclusion>,
    pub classifier: Option<String>,
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.module, self.version)
    }
}

/// A parsed POM project.
#[derive(Debug, Clone)]
pub struct Project {
    pub module: Module,
    pub version: String,
    pub packaging: String,
    pub parent: Option<(Module, String)>, // (parent module, parent version)
    pub dependencies: Vec<Dependency>,
    pub dependency_management: Vec<Dependency>,
    pub properties: HashMap<String, String>,
    pub relocation: Option<Dependency>,
}

// ─── POM Parser ───────────────────────────────────────────────────────────────

/// Parse a POM XML string into a Project.
pub fn parse_pom(xml: &str) -> Result<Project> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut path: Vec<String> = Vec::new();

    // Accumulators
    let mut group_id = String::new();
    let mut artifact_id = String::new();
    let mut version = String::new();
    let mut packaging = String::new();
    let mut parent_group_id = String::new();
    let mut parent_artifact_id = String::new();
    let mut parent_version = String::new();
    let mut deps: Vec<Dependency> = Vec::new();
    let mut dep_mgmt: Vec<Dependency> = Vec::new();
    let mut properties: HashMap<String, String> = HashMap::new();
    let mut relocation_group_id = String::new();
    let mut relocation_artifact_id = String::new();
    let mut relocation_version = String::new();

    // State machine for parsing
    let mut in_dep = false;
    let mut in_dep_mgmt = false;
    let mut in_relocation = false;
    let mut dep_group_id = String::new();
    let mut dep_artifact_id = String::new();
    let mut dep_version = String::new();
    let mut dep_scope = String::new();
    let mut dep_optional = false;
    let mut dep_exclusions: HashSet<Exclusion> = HashSet::new();
    let mut in_exclusion = false;
    let mut excl_group_id = String::new();
    let mut excl_artifact_id = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                path.push(tag.clone());

                match tag.as_str() {
                    // parent parsed via path matching
                    "dependency" => {
                        // Only capture dependencies under <dependencies> or
                        // <dependencyManagement>, not plugin deps under
                        // <build>/<plugins>/<plugin>
                        let in_project_deps = (path.iter().any(|p| p == "dependencies")
                            || path.iter().any(|p| p == "dependencyManagement"))
                            && !path
                                .iter()
                                .any(|p| p == "plugins" || p == "plugin" || p == "build");
                        if in_project_deps {
                            in_dep = true;
                            dep_group_id.clear();
                            dep_artifact_id.clear();
                            dep_version.clear();
                            dep_scope.clear();
                            dep_optional = false;
                            dep_exclusions.clear();
                        }
                    }
                    "dependencyManagement" => in_dep_mgmt = true,
                    // exclusions parsed via path matching
                    "exclusion" => {
                        in_exclusion = true;
                        excl_group_id.clear();
                        excl_artifact_id.clear();
                    }
                    "relocation" => in_relocation = true,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    // /parent end
                    "dependency" => {
                        if in_dep && !dep_group_id.is_empty() && !dep_artifact_id.is_empty() {
                            let dep = Dependency {
                                module: Module {
                                    org: dep_group_id.clone(),
                                    name: dep_artifact_id.clone(),
                                },
                                version: dep_version.clone(),
                                scope: if dep_scope.is_empty() {
                                    Scope::Compile
                                } else {
                                    Scope::parse_scope(&dep_scope)
                                },
                                optional: dep_optional,
                                exclusions: dep_exclusions.clone(),
                                classifier: None,
                            };
                            if in_dep_mgmt {
                                dep_mgmt.push(dep);
                            } else {
                                deps.push(dep);
                            }
                        }
                        in_dep = false;
                    }
                    "dependencyManagement" => in_dep_mgmt = false,
                    // /exclusions end
                    "exclusion" => {
                        if in_exclusion {
                            dep_exclusions.insert(Exclusion {
                                org: excl_group_id.clone(),
                                name: excl_artifact_id.clone(),
                            });
                        }
                        in_exclusion = false;
                    }
                    "relocation" => in_relocation = false,
                    _ => {}
                }

                path.pop();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }
                let full_path = path.join("/");

                match full_path.as_str() {
                    // Project coordinates
                    "project/groupId" => group_id = text,
                    "project/artifactId" => artifact_id = text,
                    "project/version" => version = text,
                    "project/packaging" => packaging = text,
                    // Parent
                    "project/parent/groupId" => parent_group_id = text,
                    "project/parent/artifactId" => parent_artifact_id = text,
                    "project/parent/version" => parent_version = text,
                    // Dependency fields
                    p if in_dep && p.ends_with("/groupId") && !in_exclusion => {
                        dep_group_id = text;
                    }
                    p if in_dep && p.ends_with("/artifactId") && !in_exclusion => {
                        dep_artifact_id = text;
                    }
                    p if in_dep && p.ends_with("/version") && !in_exclusion => {
                        dep_version = text;
                    }
                    p if in_dep && p.ends_with("/scope") => dep_scope = text,
                    p if in_dep && p.ends_with("/optional") => {
                        dep_optional = text == "true";
                    }
                    // Exclusion fields
                    p if in_exclusion && p.ends_with("/groupId") => excl_group_id = text,
                    p if in_exclusion && p.ends_with("/artifactId") => excl_artifact_id = text,
                    // Relocation
                    p if in_relocation && p.ends_with("/groupId") => relocation_group_id = text,
                    p if in_relocation && p.ends_with("/artifactId") => {
                        relocation_artifact_id = text;
                    }
                    p if in_relocation && p.ends_with("/version") => relocation_version = text,
                    // Properties — capture project/properties/* entries
                    p if p.starts_with("project/properties/") => {
                        let key = p.strip_prefix("project/properties/").unwrap();
                        properties.insert(key.to_string(), text);
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(anyhow!(
                    "XML parse error at {}: {e}",
                    reader.error_position()
                ))
            }
            _ => {}
        }
    }

    // Inject intrinsic Maven project properties
    if !group_id.is_empty() {
        properties.insert("project.groupId".to_string(), group_id.clone());
        properties.insert("project.version".to_string(), version.clone());
        properties.insert("groupId".to_string(), group_id.clone());
        properties.insert("version".to_string(), version.clone());
    }
    if !artifact_id.is_empty() {
        properties.insert("project.artifactId".to_string(), artifact_id.clone());
        properties.insert("artifactId".to_string(), artifact_id.clone());
    }
    if !parent_group_id.is_empty() {
        properties.insert(
            "project.parent.groupId".to_string(),
            parent_group_id.clone(),
        );
    }
    if !parent_artifact_id.is_empty() {
        properties.insert(
            "project.parent.artifactId".to_string(),
            parent_artifact_id.clone(),
        );
    }
    if !parent_version.is_empty() {
        properties.insert("project.parent.version".to_string(), parent_version.clone());
    }

    // Apply property substitution
    let substitute = |s: &str, props: &HashMap<String, String>| -> String {
        let mut result = s.to_string();
        // Repeat until stable to handle chained properties
        // (e.g. ${jackson.version} → ${jackson.core.version} → 2.17.0)
        // Cap iterations to prevent infinite loops from circular refs
        for _ in 0..10 {
            let mut changed = false;
            for (key, value) in props {
                let pattern = format!("${{{key}}}");
                if result.contains(&pattern) {
                    result = result.replace(&pattern, value);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        result
    };

    // Substitute properties in dependencies
    for dep in &mut deps {
        dep.module.org = substitute(&dep.module.org, &properties);
        dep.module.name = substitute(&dep.module.name, &properties);
        dep.version = substitute(&dep.version, &properties);
    }
    for dep in &mut dep_mgmt {
        dep.module.org = substitute(&dep.module.org, &properties);
        dep.module.name = substitute(&dep.module.name, &properties);
        dep.version = substitute(&dep.version, &properties);
    }

    let parent = if !parent_group_id.is_empty() && !parent_version.is_empty() {
        Some((
            Module {
                org: parent_group_id,
                name: parent_artifact_id,
            },
            parent_version,
        ))
    } else {
        None
    };

    let relocation = if !relocation_group_id.is_empty() {
        Some(Dependency {
            module: Module {
                org: relocation_group_id,
                name: relocation_artifact_id,
            },
            version: relocation_version,
            scope: Scope::Compile,
            optional: false,
            exclusions: HashSet::new(),
            classifier: None,
        })
    } else {
        None
    };

    if packaging.is_empty() {
        packaging = "jar".to_string();
    }

    Ok(Project {
        module: Module {
            org: group_id,
            name: artifact_id,
        },
        version,
        packaging,
        parent,
        dependencies: deps,
        dependency_management: dep_mgmt,
        properties,
        relocation,
    })
}

// ─── Maven Repository Client ─────────────────────────────────────────────────

/// A Maven repository.
#[derive(Debug, Clone)]
pub struct Repository {
    pub id: String,
    pub url: String,
}

impl Repository {
    pub fn central() -> Repository {
        Repository {
            id: "central".to_string(),
            url: "https://repo1.maven.org/maven2".to_string(),
        }
    }

    /// Build the URL for a module's Maven metadata.
    pub fn metadata_url(&self, module: &Module) -> String {
        let group_path = module.org.replace('.', "/");
        format!(
            "{}/{}/{}/maven-metadata.xml",
            self.url.trim_end_matches('/'),
            group_path,
            module.name
        )
    }

    /// Build the URL for a module's POM.
    pub fn pom_url(&self, module: &Module, version: &str) -> String {
        let group_path = module.org.replace('.', "/");
        format!(
            "{}/{}/{}/{}/{}-{}.pom",
            self.url.trim_end_matches('/'),
            group_path,
            module.name,
            version,
            module.name,
            version
        )
    }

    /// Build the URL for a module's JAR, optionally with a classifier.
    pub fn jar_url(&self, module: &Module, version: &str, classifier: Option<&str>) -> String {
        let group_path = module.org.replace('.', "/");
        let jar_filename = match classifier {
            Some(c) => format!("{}-{}-{}.jar", module.name, version, c),
            None => format!("{}-{}.jar", module.name, version),
        };
        format!(
            "{}/{}/{}/{}/{}",
            self.url.trim_end_matches('/'),
            group_path,
            module.name,
            version,
            jar_filename
        )
    }
}

/// Fetch Maven metadata from repositories, trying each in order.
pub fn fetch_maven_metadata(module: &Module, repos: &[Repository]) -> Result<MavenMetadata> {
    for repo in repos {
        let url = repo.metadata_url(module);
        match ureq::get(&url).call() {
            Ok(response) => {
                let body = response
                    .into_string()
                    .context("failed to read Maven metadata body")?;
                return parse_maven_metadata(&body)
                    .context(format!("failed to parse Maven metadata for {module}"));
            }
            Err(_) => continue,
        }
    }
    Err(anyhow!(
        "Maven metadata for {module} not found in any repository"
    ))
}

#[derive(Debug, Clone, Default)]
pub struct MavenMetadata {
    pub latest: Option<String>,
    pub release: Option<String>,
    pub versions: Vec<String>,
}

fn parse_maven_metadata(xml: &str) -> Result<MavenMetadata> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut metadata = MavenMetadata::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                path.push(String::from_utf8_lossy(e.name().as_ref()).to_string());
            }
            Ok(Event::End(_)) => {
                path.pop();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }
                match path.join("/").as_str() {
                    "metadata/versioning/latest" => metadata.latest = Some(text),
                    "metadata/versioning/release" => metadata.release = Some(text),
                    "metadata/versioning/versions/version" => metadata.versions.push(text),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(anyhow!(
                    "XML parse error at {}: {e}",
                    reader.error_position()
                ))
            }
            _ => {}
        }
    }

    Ok(metadata)
}

pub fn resolve_latest_version(module: &Module, repos: &[Repository]) -> Result<String> {
    let metadata = fetch_maven_metadata(module, repos)?;
    metadata
        .release
        .or(metadata.latest)
        .or_else(|| highest_version(&metadata.versions))
        .ok_or_else(|| anyhow!("Maven metadata for {module} does not list any versions"))
}

fn is_version_range(version: &str) -> bool {
    (version.starts_with('[') || version.starts_with('('))
        && (version.ends_with(']') || version.ends_with(')'))
}

fn resolve_version_spec(module: &Module, version: &str, repos: &[Repository]) -> Result<String> {
    if is_version_range(version) {
        let metadata = fetch_maven_metadata(module, repos)?;
        select_version_from_range(version, &metadata.versions)
            .ok_or_else(|| anyhow!("no version of {module} matches range {version}"))
    } else {
        Ok(version.to_string())
    }
}

fn select_version_from_range(range: &str, versions: &[String]) -> Option<String> {
    let include_lower = range.starts_with('[');
    let include_upper = range.ends_with(']');
    let body = range.strip_prefix(['[', '('])?.strip_suffix([']', ')'])?;

    // Maven also allows exact soft ranges like [1.2.3]. Treat them as exact.
    let (lower, upper) = match body.split_once(',') {
        Some((lower, upper)) => (empty_to_none(lower.trim()), empty_to_none(upper.trim())),
        None => (empty_to_none(body.trim()), empty_to_none(body.trim())),
    };

    versions
        .iter()
        .filter(|version| {
            let lower_ok = lower.is_none_or(|lower| {
                if include_lower {
                    compare_versions(version, lower) != std::cmp::Ordering::Less
                } else {
                    compare_versions(version, lower) == std::cmp::Ordering::Greater
                }
            });
            let upper_ok = upper.is_none_or(|upper| {
                if include_upper {
                    compare_versions(version, upper) != std::cmp::Ordering::Greater
                } else {
                    compare_versions(version, upper) == std::cmp::Ordering::Less
                }
            });
            lower_ok && upper_ok
        })
        .cloned()
        .reduce(|a, b| pick_higher_version(&a, &b))
}

fn empty_to_none(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Fetch a POM from repositories, trying each in order.
pub fn fetch_pom(module: &Module, version: &str, repos: &[Repository]) -> Result<Project> {
    for repo in repos {
        let url = repo.pom_url(module, version);
        match ureq::get(&url).call() {
            Ok(response) => {
                let body = response.into_string().context("failed to read POM body")?;
                return parse_pom(&body)
                    .context(format!("failed to parse POM for {module}:{version}"));
            }
            Err(_) => continue,
        }
    }
    Err(anyhow!(
        "POM for {module}:{version} not found in any repository"
    ))
}

// ─── Resolution Algorithm ────────────────────────────────────────────────────

/// Resolved artifact: a module at a specific version with optional classifier.
#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub module: Module,
    pub version: String,
    pub classifier: Option<String>,
}

impl fmt::Display for ResolvedArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.module, self.version)
    }
}

/// Resolve Maven coordinates to their transitive dependency artifacts (metadata only).
///
/// `coordinates` are Maven coordinates like `org.slf4j:slf4j-api:2.0.13`.
/// Returns resolved artifact metadata (groupId, artifactId, version, classifier).
/// Does NOT download JARs — use `resolve_classpath` for that.
pub fn resolve(
    coordinates: &[String],
    repos: &[Repository],
    _cache_dir: &Path,
) -> Result<Vec<ResolvedArtifact>> {
    // Parse root coordinates
    let mut root_deps: Vec<Dependency> = Vec::new();
    for coord in coordinates {
        let dep = parse_coordinate(coord)?;
        root_deps.push(dep);
    }

    if root_deps.is_empty() {
        return Ok(Vec::new());
    }

    // Resolution state
    let mut project_cache: HashMap<(Module, String), Project> = HashMap::new();
    let mut resolved: HashMap<Module, (String, Option<String>)> = HashMap::new(); // module → (version, classifier)
    let mut queue: Vec<Dependency> = root_deps.clone();
    let mut iterations = 0;
    let max_iterations = 2000; // safety limit

    while !queue.is_empty() && iterations < max_iterations {
        iterations += 1;
        let mut next_queue: Vec<Dependency> = Vec::new();

        for dep in &queue {
            // Check exclusions from parent deps — skip if excluded
            // (handled at extraction time, but double-check here)

            // Version reconciliation: if we already resolved this module, use that version
            let requested_version = if let Some((chosen, _)) = resolved.get(&dep.module) {
                chosen.clone()
            } else {
                resolve_version_spec(&dep.module, &dep.version, repos)?
            };
            let version = requested_version;

            // Skip if already fully processed
            let cache_key = (dep.module.clone(), version.clone());
            if project_cache.contains_key(&cache_key) {
                // Already fetched; just extract transitive deps below
            } else {
                // Fetch POM
                let project = fetch_pom(&dep.module, &version, repos)?;

                // Resolve parent chain first
                let effective = resolve_parent_chain(&project, repos, &mut project_cache)?;

                // Handle relocation: if this project has a relocation, remove
                // the old module and track the relocation target instead
                if let Some(ref reloc) = effective.relocation {
                    resolved.remove(&dep.module);
                    resolved.insert(reloc.module.clone(), (reloc.version.clone(), None));
                    let reloc_key = (reloc.module.clone(), reloc.version.clone());
                    if !project_cache.contains_key(&reloc_key) {
                        // Fetch the relocated project in the next iteration
                        next_queue.push(reloc.clone());
                    }
                    // Don't cache the old artifact — it's relocated
                    continue;
                }

                project_cache.insert(cache_key.clone(), effective);
            }

            // Record the chosen version + classifier
            resolved.insert(dep.module.clone(), (version, dep.classifier.clone()));
        }

        // Extract transitive deps from all newly-fetched projects
        for dep in &queue {
            let (version, _) = resolved
                .get(&dep.module)
                .cloned()
                .unwrap_or_else(|| (dep.version.clone(), dep.classifier.clone()));
            let cache_key = (dep.module.clone(), version.clone());
            if let Some(project) = project_cache.get(&cache_key) {
                let transitive = extract_dependencies(project, dep);
                for t in transitive {
                    // Version reconciliation: if already resolved, use higher version
                    let effective_version = match resolved.get(&t.module) {
                        Some((existing, _)) => {
                            let requested = resolve_version_spec(&t.module, &t.version, repos)?;
                            let higher = pick_higher_version(existing, &requested);
                            resolved
                                .insert(t.module.clone(), (higher.clone(), t.classifier.clone()));
                            higher
                        }
                        None => {
                            let requested = resolve_version_spec(&t.module, &t.version, repos)?;
                            resolved.insert(
                                t.module.clone(),
                                (requested.clone(), t.classifier.clone()),
                            );
                            requested
                        }
                    };

                    // Only queue if not already processed
                    let t_key = (t.module.clone(), effective_version.clone());
                    if !project_cache.contains_key(&t_key) {
                        next_queue.push(Dependency {
                            module: t.module.clone(),
                            version: effective_version,
                            scope: t.scope,
                            optional: t.optional,
                            exclusions: t.exclusions,
                            classifier: t.classifier.clone(),
                        });
                    }
                }
            }
        }

        queue = next_queue;
    }

    if iterations >= max_iterations {
        return Err(anyhow!(
            "resolution did not converge after {max_iterations} iterations"
        ));
    }

    // Build the final artifact list
    let mut artifacts: Vec<ResolvedArtifact> = resolved
        .iter()
        .map(|(module, (version, classifier))| ResolvedArtifact {
            module: module.clone(),
            version: version.clone(),
            classifier: classifier.clone(),
        })
        .collect();

    // Sort for determinism
    artifacts.sort_by(|a, b| {
        a.module
            .org
            .cmp(&b.module.org)
            .then_with(|| a.module.name.cmp(&b.module.name))
    });

    // Return as resolved artifacts (metadata only, no JAR download)
    Ok(artifacts)
}

/// Build a classpath from resolved artifacts.
pub fn resolve_classpath(
    coordinates: &[String],
    repos: &[Repository],
    cache_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let artifacts = resolve(coordinates, repos, cache_dir)?;

    // Download all JARs (resolve is now metadata-only)
    fs::create_dir_all(cache_dir).context("failed to create cache directory")?;
    for artifact in &artifacts {
        if let Err(e) = download_jar(artifact, repos, cache_dir) {
            // POM-only artifacts (BOMs, parent POMs) won't have JARs — skip silently
            // We can't tell from here if it's POM-only, so just warn
            eprintln!("warning: could not download JAR for {artifact}: {e:#}");
        }
    }

    let mut paths: Vec<PathBuf> = Vec::new();
    for artifact in &artifacts {
        let group_path = sanitize_path_segment(&artifact.module.org).replace('.', "/");
        let artifact_name = sanitize_path_segment(&artifact.module.name);
        let version = sanitize_path_segment(&artifact.version);
        let jar_name = match &artifact.classifier {
            Some(c) => format!(
                "{}-{}-{}.jar",
                artifact_name,
                version,
                sanitize_path_segment(c)
            ),
            None => format!("{}-{}.jar", artifact_name, version),
        };
        let jar_path = cache_dir
            .join(&group_path)
            .join(&artifact_name)
            .join(&jar_name);
        if jar_path.exists() {
            paths.push(jar_path);
        }
    }

    Ok(paths)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Parse a Maven coordinate string like `org.slf4j:slf4j-api:2.0.13`.
pub fn parse_coordinate(coord: &str) -> Result<Dependency> {
    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 {
        return Err(anyhow!(
            "invalid Maven coordinate '{coord}' (expected groupId:artifactId[:classifier]:version)"
        ));
    }

    // With 4+ segments: group:artifact[:classifier]:version
    // Last segment is always version. If 4 parts, part[2] is classifier.
    let (org, name, classifier, version) = if parts.len() >= 4 {
        (
            parts[0].to_string(),
            parts[1].to_string(),
            Some(parts[2].to_string()),
            parts[3].to_string(),
        )
    } else {
        (
            parts[0].to_string(),
            parts[1].to_string(),
            None,
            parts[2].to_string(),
        )
    };

    Ok(Dependency {
        module: Module { org, name },
        version,
        scope: Scope::Compile,
        optional: false,
        exclusions: HashSet::new(),
        classifier,
    })
}

/// Extract transitively-reachable dependencies from a project,
/// applying scope filtering, dependency management, exclusions, and optionals.
fn extract_dependencies(project: &Project, from_dep: &Dependency) -> Vec<Dependency> {
    let mut deps: Vec<Dependency> = Vec::new();

    for dep in &project.dependencies {
        // Scope filter: only compile + runtime
        if !dep.scope.is_in_classpath() {
            continue;
        }

        // Skip optional deps (Coursier's defaultFilter)
        if dep.optional {
            continue;
        }

        // Apply dependencyManagement: fill missing version
        let mut effective = dep.clone();
        if effective.version.is_empty() {
            if let Some(managed) = project
                .dependency_management
                .iter()
                .find(|m| m.module == effective.module)
            {
                effective.version = managed.version.clone();
                if effective.scope == Scope::Compile && managed.scope != Scope::Compile {
                    effective.scope = managed.scope;
                }
            }
        }
        if effective.version.is_empty() {
            // Still no version — skip this dep
            continue;
        }

        // Apply exclusions from the parent dep
        if from_dep
            .exclusions
            .iter()
            .any(|ex| ex.matches(&effective.module))
        {
            continue;
        }

        // Propagate exclusions
        effective.exclusions.extend(from_dep.exclusions.clone());

        deps.push(effective);
    }

    deps
}

/// Resolve the parent POM chain, merging inherited properties and dependencyManagement.
fn resolve_parent_chain(
    project: &Project,
    repos: &[Repository],
    cache: &mut HashMap<(Module, String), Project>,
) -> Result<Project> {
    resolve_parent_chain_inner(project, repos, cache, &mut HashSet::new(), 0)
}

fn resolve_parent_chain_inner(
    project: &Project,
    repos: &[Repository],
    cache: &mut HashMap<(Module, String), Project>,
    seen: &mut HashSet<(Module, String)>,
    depth: usize,
) -> Result<Project> {
    const MAX_DEPTH: usize = 50;
    if depth > MAX_DEPTH {
        return Err(anyhow!(
            "parent POM chain exceeds {MAX_DEPTH} levels — possible circular reference"
        ));
    }

    let mut effective = project.clone();

    if let Some((ref parent_module, ref parent_version)) = project.parent {
        let cache_key = (parent_module.clone(), parent_version.clone());

        // Cycle detection: if we've already seen this parent, break the loop
        if seen.contains(&cache_key) {
            return Err(anyhow!(
                "circular parent POM reference detected: {}:{}",
                parent_module,
                parent_version
            ));
        }
        seen.insert(cache_key.clone());

        let parent = if let Some(p) = cache.get(&cache_key) {
            p.clone()
        } else {
            let p = fetch_pom(parent_module, parent_version, repos)?;
            let resolved = resolve_parent_chain_inner(&p, repos, cache, seen, depth + 1)?;
            cache.insert(cache_key.clone(), resolved.clone());
            resolved
        };

        // Inherit groupId if empty
        if effective.module.org.is_empty() {
            effective.module.org = parent.module.org.clone();
        }
        // Inherit version if empty
        if effective.version.is_empty() {
            effective.version = parent.version.clone();
        }
        // Merge properties (child overrides parent)
        for (key, value) in &parent.properties {
            effective
                .properties
                .entry(key.clone())
                .or_insert(value.clone());
        }
        // Merge dependencyManagement (child takes precedence, parent fills gaps)
        for managed in &parent.dependency_management {
            if !effective
                .dependency_management
                .iter()
                .any(|m| m.module == managed.module)
            {
                effective.dependency_management.push(managed.clone());
            }
        }
    }

    // Handle BOM imports in dependencyManagement (scope=import)
    let mut bom_imports: Vec<Dependency> = Vec::new();
    for dep in &effective.dependency_management {
        if dep.scope == Scope::Import {
            bom_imports.push(dep.clone());
        }
    }
    for bom in &bom_imports {
        let cache_key = (bom.module.clone(), bom.version.clone());
        let bom_project = if let Some(p) = cache.get(&cache_key) {
            p.clone()
        } else {
            match fetch_pom(&bom.module, &bom.version, repos) {
                Ok(p) => {
                    let resolved = resolve_parent_chain(&p, repos, cache)?;
                    cache.insert(cache_key.clone(), resolved.clone());
                    resolved
                }
                Err(_) => continue, // Skip BOMs we can't fetch
            }
        };
        for managed in &bom_project.dependency_management {
            if !effective
                .dependency_management
                .iter()
                .any(|m| m.module == managed.module)
            {
                effective.dependency_management.push(managed.clone());
            }
        }
    }

    // Inject intrinsic project properties after inheritance resolution
    if !effective.module.org.is_empty() {
        effective
            .properties
            .insert("project.groupId".to_string(), effective.module.org.clone());
        effective
            .properties
            .insert("groupId".to_string(), effective.module.org.clone());
    }
    if !effective.version.is_empty() {
        effective
            .properties
            .insert("project.version".to_string(), effective.version.clone());
        effective
            .properties
            .insert("version".to_string(), effective.version.clone());
    }
    if !effective.module.name.is_empty() {
        effective.properties.insert(
            "project.artifactId".to_string(),
            effective.module.name.clone(),
        );
        effective
            .properties
            .insert("artifactId".to_string(), effective.module.name.clone());
    }

    // Re-apply property substitution with merged properties
    let substitute = |s: &str, props: &HashMap<String, String>| -> String {
        let mut result = s.to_string();
        // Repeat until stable to handle chained properties
        // Cap iterations to prevent infinite loops from circular refs
        for _ in 0..10 {
            let mut changed = false;
            for (key, value) in props {
                let pattern = format!("${{{key}}}");
                if result.contains(&pattern) {
                    result = result.replace(&pattern, value);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        result
    };

    for dep in &mut effective.dependencies {
        dep.module.org = substitute(&dep.module.org, &effective.properties);
        dep.module.name = substitute(&dep.module.name, &effective.properties);
        dep.version = substitute(&dep.version, &effective.properties);
    }
    for dep in &mut effective.dependency_management {
        dep.module.org = substitute(&dep.module.org, &effective.properties);
        dep.module.name = substitute(&dep.module.name, &effective.properties);
        dep.version = substitute(&dep.version, &effective.properties);
    }

    Ok(effective)
}

fn highest_version(versions: &[String]) -> Option<String> {
    versions
        .iter()
        .cloned()
        .reduce(|a, b| pick_higher_version(&a, &b))
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    if a == b {
        return std::cmp::Ordering::Equal;
    }
    if pick_higher_version(a, b) == a {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Less
    }
}

/// Pick the higher of two Maven version strings.
/// Simplified: compare segment-by-segment (numeric where possible).
fn pick_higher_version(a: &str, b: &str) -> String {
    if a == b {
        return a.to_string();
    }
    let pa = parse_version_parts(a);
    let pb = parse_version_parts(b);
    for (sa, sb) in pa.iter().zip(pb.iter()) {
        let oa = qualifier_order(sa);
        let ob = qualifier_order(sb);
        if oa != ob {
            // Higher qualifier order = more mature/release → wins
            if oa > ob {
                return a.to_string();
            } else {
                return b.to_string();
            }
        }
        // Same qualifier type — numeric or lexicographic comparison
        match (sa.parse::<u64>(), sb.parse::<u64>()) {
            (Ok(na), Ok(nb)) => {
                if na > nb {
                    return a.to_string();
                } else if nb > na {
                    return b.to_string();
                }
            }
            _ => {
                if sa > sb {
                    return a.to_string();
                } else if sb > sa {
                    return b.to_string();
                }
            }
        }
    }
    // If all compared parts are equal, check for trailing qualifier segments.
    // The version with extra pre-release parts (SNAPSHOT, alpha, etc.) should LOSE
    // to the version without them. E.g. "2.0.0" > "2.0.0-SNAPSHOT".
    let a_tail = pa.get(pb.len()).map(|s| qualifier_order(s));
    let b_tail = pb.get(pa.len()).map(|s| qualifier_order(s));
    match (a_tail, b_tail) {
        // a has extra parts, b doesn't — a wins only if extra part is release-like
        (Some(qa), None) => {
            if qa >= 10 {
                a.to_string()
            } else {
                b.to_string()
            }
        }
        // b has extra parts, a doesn't — b wins only if extra part is release-like
        (None, Some(qb)) => {
            if qb >= 10 {
                b.to_string()
            } else {
                a.to_string()
            }
        }
        // Both have extra parts (equal length, already compared above) or neither
        _ => {
            if pa.len() >= pb.len() {
                a.to_string()
            } else {
                b.to_string()
            }
        }
    }
}

/// Maven qualifier ordering: lower = more mature.
/// SNAPSHOT < alpha/beta < milestone < RC < release (no qualifier).
/// See: https://docs.oracle.com/middleware/1212/core/MAVEN/maven_version.htm
fn qualifier_order(part: &str) -> u32 {
    let lower = part.to_lowercase();
    if lower == "snapshot" || lower.ends_with("-snapshot") {
        0
    } else if lower.starts_with("alpha") || lower == "a" {
        1
    } else if lower.starts_with("beta") || lower == "b" {
        2
    } else if lower.starts_with("milestone") || lower == "m" {
        3
    } else if lower.starts_with("rc") || lower.starts_with("cr") {
        4
    } else {
        10 // release / numeric
    }
}

fn parse_version_parts(v: &str) -> Vec<String> {
    v.split(['.', '-']).map(|s| s.to_string()).collect()
}

/// Download a JAR to the cache directory.
/// Probes Maven local repo and Gradle cache before downloading.
fn download_jar(
    artifact: &ResolvedArtifact,
    repos: &[Repository],
    cache_dir: &Path,
) -> Result<PathBuf> {
    // Use groupId path segments to avoid collisions between artifacts
    // with the same artifactId+version from different groups.
    // Sanitize all coordinate-derived path segments to prevent traversal.
    let group_path = sanitize_path_segment(&artifact.module.org).replace('.', "/");
    let artifact_name = sanitize_path_segment(&artifact.module.name);
    let version = sanitize_path_segment(&artifact.version);
    let jar_name = match &artifact.classifier {
        Some(c) => format!(
            "{}-{}-{}.jar",
            artifact_name,
            version,
            sanitize_path_segment(c)
        ),
        None => format!("{}-{}.jar", artifact_name, version),
    };
    let jar_path = cache_dir
        .join(&group_path)
        .join(&artifact_name)
        .join(&jar_name);

    // 1. Check jbx's own cache
    if jar_path.exists() {
        return Ok(jar_path);
    }

    // 2. Probe existing tool caches (Maven, Gradle, Coursier)
    if let Some(cached) = probe_local_caches(&artifact.module, &artifact.version, &jar_name) {
        // Symlink from jbx cache to the existing file
        if let Some(parent) = jar_path.parent() {
            fs::create_dir_all(parent)?;
        }
        symlink_or_copy(&cached, &jar_path)?;
        return Ok(jar_path);
    }

    // 3. Download from remote repositories
    for repo in repos {
        let url = repo.jar_url(
            &artifact.module,
            &artifact.version,
            artifact.classifier.as_deref(),
        );
        match ureq::get(&url).call() {
            Ok(response) => {
                if let Some(parent) = jar_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut body = response.into_reader();
                let mut file = std::fs::File::create(&jar_path)
                    .context(format!("failed to create {jar_path:?}"))?;
                std::io::copy(&mut body, &mut file)?;

                // Verify SHA1 checksum
                if let Err(e) = verify_sha1(&url, &jar_path) {
                    // Delete corrupted file and try next repo
                    let _ = fs::remove_file(&jar_path);
                    eprintln!("warning: SHA1 verification failed for {artifact}: {e}");
                    continue;
                }

                return Ok(jar_path);
            }
            Err(_) => continue,
        }
    }

    Err(anyhow!("JAR for {artifact} not found in any repository"))
}

/// Verify the SHA1 checksum of a downloaded JAR against the published `.sha1` file.
fn verify_sha1(jar_url: &str, jar_path: &Path) -> Result<()> {
    use sha1::Digest;
    let sha1_url = format!("{jar_url}.sha1");
    let expected = match ureq::get(&sha1_url).call() {
        Ok(response) => {
            let mut text = String::new();
            response.into_reader().read_to_string(&mut text)?;
            // SHA1 files may contain "hash  filename" — take only the hash
            text.split_whitespace()
                .next()
                .ok_or_else(|| anyhow!("empty SHA1 response for {sha1_url}"))?
                .to_string()
        }
        Err(_) => {
            // No SHA1 file available — skip verification (some repos don't publish them)
            return Ok(());
        }
    };

    let mut hasher = sha1::Sha1::new();
    let mut file = std::fs::File::open(jar_path)?;
    std::io::copy(&mut file, &mut hasher)?;
    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        anyhow::bail!(
            "SHA1 mismatch: expected {expected}, got {actual} for {}",
            jar_path.display()
        );
    }
    Ok(())
}

/// Create a symlink on Unix, or copy the file on non-Unix platforms.
fn symlink_or_copy(src: &Path, dst: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
            .context(format!("failed to symlink {:?} → {:?}", src, dst))?;
    }
    #[cfg(not(unix))]
    {
        fs::copy(src, dst).context(format!("failed to copy {:?} → {:?}", src, dst))?;
    }
    Ok(())
}

/// Probe Maven local repo, Gradle cache, and Coursier cache for an existing JAR.
/// Returns the first matching path found, or None.
pub fn probe_local_caches(module: &Module, version: &str, jar_name: &str) -> Option<PathBuf> {
    dirs::home_dir().and_then(|home| probe_local_caches_with_home(module, version, jar_name, &home))
}

/// Same as [`probe_local_caches`] but with an explicit home directory (testable).
pub fn probe_local_caches_with_home(
    module: &Module,
    version: &str,
    jar_name: &str,
    home: &Path,
) -> Option<PathBuf> {
    // Maven: ~/.m2/repository/{group_path}/{artifactId}/{version}/{jar_name}
    let group_path = module.org.replace('.', "/");
    let maven_path = home
        .join(".m2/repository")
        .join(&group_path)
        .join(&module.name)
        .join(version)
        .join(jar_name);
    if maven_path.exists() {
        return Some(maven_path);
    }

    // Gradle: ~/.gradle/caches/modules-2/files-2.1/{groupId}/{artifactId}/{version}/{hash}/{jar_name}
    // The hash subdirectory is random — we need to scan it.
    let gradle_base = home
        .join(".gradle/caches/modules-2/files-2.1")
        .join(&module.org)
        .join(&module.name)
        .join(version);
    if gradle_base.is_dir() {
        if let Ok(entries) = fs::read_dir(&gradle_base) {
            for hash_dir in entries.flatten() {
                if let Ok(files) = fs::read_dir(hash_dir.path()) {
                    for file in files.flatten() {
                        if file.file_name() == jar_name {
                            return Some(file.path());
                        }
                    }
                }
            }
        }
    }

    // Coursier: ~/.cache/coursier/v1/https/repo1.maven.org/maven2/{group_path}/{artifactId}/{version}/{jar_name}
    // (Only handles Maven Central; other repos have different path structures)
    let coursier_path = home
        .join(".cache/coursier/v1/https/repo1.maven.org/maven2")
        .join(&group_path)
        .join(&module.name)
        .join(version)
        .join(jar_name);
    if coursier_path.exists() {
        return Some(coursier_path);
    }

    None
}

use std::fs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_coordinate() {
        let dep = parse_coordinate("com.google:guava:33.0.0").unwrap();
        assert_eq!(dep.module.org, "com.google");
        assert_eq!(dep.module.name, "guava");
        assert_eq!(dep.version, "33.0.0");
    }

    #[test]
    fn rejects_invalid_coordinate() {
        assert!(parse_coordinate("com.google:guava").is_err());
        assert!(parse_coordinate("").is_err());
    }

    #[test]
    fn parses_pom_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <packaging>jar</packaging>
  <dependencies>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version>2.0.13</version>
    </dependency>
    <dependency>
      <groupId>junit</groupId>
      <artifactId>junit</artifactId>
      <version>4.13.2</version>
      <scope>test</scope>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert_eq!(project.module.org, "com.example");
        assert_eq!(project.module.name, "my-app");
        assert_eq!(project.version, "1.0.0");
        assert_eq!(project.packaging, "jar");
        assert_eq!(project.dependencies.len(), 2);

        // First dep: compile scope (default)
        assert_eq!(project.dependencies[0].module.org, "org.slf4j");
        assert_eq!(project.dependencies[0].scope, Scope::Compile);

        // Second dep: test scope
        assert_eq!(project.dependencies[1].module.org, "junit");
        assert_eq!(project.dependencies[1].scope, Scope::Test);
    }

    #[test]
    fn parses_pom_with_exclusions() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>com.google</groupId>
      <artifactId>guava</artifactId>
      <version>33.0.0</version>
      <exclusions>
        <exclusion>
          <groupId>com.google</groupId>
          <artifactId>failureaccess</artifactId>
        </exclusion>
      </exclusions>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert_eq!(project.dependencies.len(), 1);
        assert_eq!(project.dependencies[0].exclusions.len(), 1);
        let excl = project.dependencies[0].exclusions.iter().next().unwrap();
        assert_eq!(excl.org, "com.google");
        assert_eq!(excl.name, "failureaccess");
    }

    #[test]
    fn extracts_only_compile_and_runtime_deps() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version>2.0.13</version>
    </dependency>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-simple</artifactId>
      <version>2.0.13</version>
      <scope>runtime</scope>
    </dependency>
    <dependency>
      <groupId>junit</groupId>
      <artifactId>junit</artifactId>
      <version>4.13.2</version>
      <scope>test</scope>
    </dependency>
    <dependency>
      <groupId>javax.servlet</groupId>
      <artifactId>javax.servlet-api</artifactId>
      <version>4.0.1</version>
      <scope>provided</scope>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        let parent_dep = Dependency {
            module: Module {
                org: "test".to_string(),
                name: "parent".to_string(),
            },
            version: "1.0".to_string(),
            scope: Scope::Compile,
            optional: false,
            exclusions: HashSet::new(),
            classifier: None,
        };
        let deps = extract_dependencies(&project, &parent_dep);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].module.name, "slf4j-api");
        assert_eq!(deps[1].module.name, "slf4j-simple");
    }

    #[test]
    fn skips_optional_deps() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version>2.0.13</version>
    </dependency>
    <dependency>
      <groupId>com.google.code.findbugs</groupId>
      <artifactId>jsr305</artifactId>
      <version>3.0.2</version>
      <optional>true</optional>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        let parent_dep = Dependency {
            module: Module {
                org: "test".to_string(),
                name: "parent".to_string(),
            },
            version: "1.0".to_string(),
            scope: Scope::Compile,
            optional: false,
            exclusions: HashSet::new(),
            classifier: None,
        };
        let deps = extract_dependencies(&project, &parent_dep);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].module.name, "slf4j-api");
    }

    #[test]
    fn applies_exclusions_from_parent() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>com.google</groupId>
      <artifactId>failureaccess</artifactId>
      <version>1.0.2</version>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        let parent_dep = Dependency {
            module: Module {
                org: "test".to_string(),
                name: "parent".to_string(),
            },
            version: "1.0".to_string(),
            scope: Scope::Compile,
            optional: false,
            exclusions: HashSet::from([Exclusion {
                org: "com.google".to_string(),
                name: "failureaccess".to_string(),
            }]),
            classifier: None,
        };
        let deps = extract_dependencies(&project, &parent_dep);
        assert!(deps.is_empty(), "excluded dep should be filtered out");
    }

    #[test]
    fn parses_pom_with_properties() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <properties>
    <slf4j.version>2.0.13</slf4j.version>
  </properties>
  <dependencies>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version>${slf4j.version}</version>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert_eq!(project.dependencies[0].version, "2.0.13");
    }

    #[test]
    fn substitutes_project_parent_version_property_in_dependencies() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <parent>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <version>1.2.3</version>
  </parent>
  <artifactId>child</artifactId>
  <dependencies>
    <dependency>
      <groupId>com.example</groupId>
      <artifactId>service</artifactId>
      <version>${project.parent.version}</version>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert_eq!(project.dependencies[0].version, "1.2.3");
    }

    #[test]
    fn resolves_chained_properties() {
        // jackson.version → jackson.core.version → 2.17.0
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <properties>
    <jackson.version>${jackson.core.version}</jackson.version>
    <jackson.core.version>2.17.0</jackson.core.version>
  </properties>
  <dependencies>
    <dependency>
      <groupId>com.fasterxml.jackson.core</groupId>
      <artifactId>jackson-databind</artifactId>
      <version>${jackson.version}</version>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert_eq!(project.dependencies[0].version, "2.17.0");
    }

    #[test]
    fn sanitize_path_segment_blocks_traversal() {
        assert_eq!(sanitize_path_segment("../../etc"), "_dotdot___dotdot__etc");
        assert_eq!(sanitize_path_segment("foo/bar"), "foo_bar");
        assert_eq!(sanitize_path_segment("foo\\bar"), "foo_bar");
        assert_eq!(sanitize_path_segment("normal"), "normal");
    }

    #[test]
    fn parses_dependency_management() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.0.0</version>
  <dependencyManagement>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version>2.0.13</version>
    </dependency>
  </dependencyManagement>
  <dependencies>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version></version>
    </dependency>
  </dependencies>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert_eq!(project.dependency_management.len(), 1);

        // Test that dependencyManagement fills empty version
        let parent_dep = Dependency {
            module: Module {
                org: "test".to_string(),
                name: "parent".to_string(),
            },
            version: "1.0".to_string(),
            scope: Scope::Compile,
            optional: false,
            exclusions: HashSet::new(),
            classifier: None,
        };
        let deps = extract_dependencies(&project, &parent_dep);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "2.0.13");
    }

    #[test]
    fn version_comparison() {
        assert_eq!(pick_higher_version("1.0", "2.0"), "2.0");
        assert_eq!(pick_higher_version("2.0", "1.0"), "2.0");
        assert_eq!(pick_higher_version("1.0.0", "1.0.1"), "1.0.1");
        assert_eq!(pick_higher_version("1.0.0", "1.0.0"), "1.0.0");
        assert_eq!(
            pick_higher_version("33.3.1-jre", "33.3.0-jre"),
            "33.3.1-jre"
        );
        // Release beats SNAPSHOT (SNAPSHOT is pre-release)
        assert_eq!(pick_higher_version("2.0.0-SNAPSHOT", "2.0.0"), "2.0.0");
        assert_eq!(pick_higher_version("2.0.0", "2.0.0-SNAPSHOT"), "2.0.0");
        // Release beats alpha/beta/RC
        assert_eq!(pick_higher_version("1.0-alpha1", "1.0"), "1.0");
        assert_eq!(pick_higher_version("1.0", "1.0-beta1"), "1.0");
        assert_eq!(pick_higher_version("1.0-RC1", "1.0"), "1.0");
        // RC beats beta beats alpha
        assert_eq!(pick_higher_version("1.0-alpha1", "1.0-beta1"), "1.0-beta1");
        assert_eq!(pick_higher_version("1.0-beta1", "1.0-RC1"), "1.0-RC1");
        assert_eq!(pick_higher_version("1.0-RC1", "1.0-SNAPSHOT"), "1.0-RC1");
        // Longer release version beats shorter (1.0.0 > 1.0)
        assert_eq!(pick_higher_version("1.0", "1.0.0"), "1.0.0");
    }

    #[test]
    fn version_range_selects_highest_matching_metadata_version() {
        let versions = vec![
            "1.79".to_string(),
            "1.80".to_string(),
            "1.80.1".to_string(),
            "1.81".to_string(),
        ];
        assert_eq!(
            select_version_from_range("[1.80,1.81)", &versions).unwrap(),
            "1.80.1"
        );
    }

    #[test]
    fn parses_pom_with_relocation() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>old-artifact</artifactId>
  <version>1.0.0</version>
  <distributionManagement>
    <relocation>
      <groupId>com.newexample</groupId>
      <artifactId>new-artifact</artifactId>
      <version>2.0.0</version>
    </relocation>
  </distributionManagement>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert!(project.relocation.is_some());
        let rel = project.relocation.unwrap();
        assert_eq!(rel.module.org, "com.newexample");
        assert_eq!(rel.module.name, "new-artifact");
        assert_eq!(rel.version, "2.0.0");
    }

    #[test]
    fn classifier_is_parsed_from_four_segment_coordinate() {
        let dep = parse_coordinate("org.example:lib:sources:1.0").unwrap();
        assert_eq!(dep.module.org, "org.example");
        assert_eq!(dep.module.name, "lib");
        assert_eq!(dep.classifier, Some("sources".to_string()));
        assert_eq!(dep.version, "1.0");
    }

    #[test]
    fn classifier_is_none_from_three_segment_coordinate() {
        let dep = parse_coordinate("org.example:lib:1.0").unwrap();
        assert_eq!(dep.classifier, None);
    }

    #[test]
    fn probes_maven_local_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let maven_repo = tmp.path().join(".m2/repository/com/example/lib/1.0");
        fs::create_dir_all(&maven_repo).unwrap();
        let jar = maven_repo.join("lib-1.0.jar");
        fs::write(&jar, "fake").unwrap();

        let module = Module {
            org: "com.example".to_string(),
            name: "lib".to_string(),
        };
        let result = probe_local_caches_with_home(&module, "1.0", "lib-1.0.jar", tmp.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), jar);
    }

    #[test]
    fn probes_gradle_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let hash_dir = tmp
            .path()
            .join(".gradle/caches/modules-2/files-2.1/com.example/lib/1.0/abcdef1234");
        fs::create_dir_all(&hash_dir).unwrap();
        let jar = hash_dir.join("lib-1.0.jar");
        fs::write(&jar, "fake").unwrap();

        let module = Module {
            org: "com.example".to_string(),
            name: "lib".to_string(),
        };
        let result = probe_local_caches_with_home(&module, "1.0", "lib-1.0.jar", tmp.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), jar);
    }

    #[test]
    fn probes_coursier_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let coursier_path = tmp
            .path()
            .join(".cache/coursier/v1/https/repo1.maven.org/maven2/com/example/lib/1.0");
        fs::create_dir_all(&coursier_path).unwrap();
        let jar = coursier_path.join("lib-1.0.jar");
        fs::write(&jar, "fake").unwrap();

        let module = Module {
            org: "com.example".to_string(),
            name: "lib".to_string(),
        };
        let result = probe_local_caches_with_home(&module, "1.0", "lib-1.0.jar", tmp.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), jar);
    }

    #[test]
    fn returns_none_when_not_cached() {
        let tmp = tempfile::tempdir().unwrap();
        let module = Module {
            org: "com.nonexistent".to_string(),
            name: "nothing".to_string(),
        };
        let result = probe_local_caches_with_home(&module, "9.9", "nothing-9.9.jar", tmp.path());
        assert!(result.is_none());
    }
}
