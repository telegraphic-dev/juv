//! Native Maven dependency resolver for juv.
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
                        in_dep = true;
                        dep_group_id.clear();
                        dep_artifact_id.clear();
                        dep_version.clear();
                        dep_scope.clear();
                        dep_optional = false;
                        dep_exclusions.clear();
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

    // Apply property substitution
    let substitute = |s: &str, props: &HashMap<String, String>| -> String {
        let mut result = s.to_string();
        for (key, value) in props {
            let pattern = format!("${{{key}}}");
            if result.contains(&pattern) {
                result = result.replace(&pattern, value);
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

    /// Build the URL for a module's JAR.
    pub fn jar_url(&self, module: &Module, version: &str) -> String {
        let group_path = module.org.replace('.', "/");
        format!(
            "{}/{}/{}/{}/{}-{}.jar",
            self.url.trim_end_matches('/'),
            group_path,
            module.name,
            version,
            module.name,
            version
        )
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

/// Resolved artifact: a module at a specific version with its JAR path.
#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub module: Module,
    pub version: String,
}

impl fmt::Display for ResolvedArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.module, self.version)
    }
}

/// Resolve a set of root coordinates into a full dependency list.
///
/// `coordinates` are Maven coordinates like `org.slf4j:slf4j-api:2.0.13`.
/// `repos` is the list of Maven repositories to search.
/// `cache_dir` is where downloaded JARs are stored.
pub fn resolve(
    coordinates: &[String],
    repos: &[Repository],
    cache_dir: &Path,
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
    let mut resolved: HashMap<Module, String> = HashMap::new(); // module → chosen version
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
            let version = if let Some(chosen) = resolved.get(&dep.module) {
                chosen.clone()
            } else {
                dep.version.clone()
            };

            // Skip if already fully processed
            let cache_key = (dep.module.clone(), version.clone());
            if project_cache.contains_key(&cache_key) {
                // Already fetched; just extract transitive deps below
            } else {
                // Fetch POM
                let project = fetch_pom(&dep.module, &version, repos)?;

                // Resolve parent chain first
                let effective = resolve_parent_chain(&project, repos, &mut project_cache)?;
                project_cache.insert(cache_key.clone(), effective);
            }

            // Record the chosen version
            resolved.insert(dep.module.clone(), version);
        }

        // Extract transitive deps from all newly-fetched projects
        for dep in &queue {
            let version = resolved
                .get(&dep.module)
                .cloned()
                .unwrap_or_else(|| dep.version.clone());
            let cache_key = (dep.module.clone(), version.clone());
            if let Some(project) = project_cache.get(&cache_key) {
                let transitive = extract_dependencies(project, dep);
                for t in transitive {
                    // Version reconciliation: if already resolved, use higher version
                    let effective_version = match resolved.get(&t.module) {
                        Some(existing) => {
                            let higher = pick_higher_version(existing, &t.version);
                            resolved.insert(t.module.clone(), higher.clone());
                            higher
                        }
                        None => {
                            resolved.insert(t.module.clone(), t.version.clone());
                            t.version.clone()
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
        .map(|(module, version)| ResolvedArtifact {
            module: module.clone(),
            version: version.clone(),
        })
        .collect();

    // Sort for determinism
    artifacts.sort_by(|a, b| {
        a.module
            .org
            .cmp(&b.module.org)
            .then_with(|| a.module.name.cmp(&b.module.name))
    });

    // Download JARs
    let mut jar_paths: Vec<PathBuf> = Vec::new();
    fs::create_dir_all(cache_dir).context("failed to create cache directory")?;
    for artifact in &artifacts {
        match download_jar(artifact, repos, cache_dir) {
            Ok(path) => jar_paths.push(path),
            Err(e) => {
                // POM-only artifacts (BOMs, parent POMs) won't have JARs — skip silently
                if let Some(project) =
                    project_cache.get(&(artifact.module.clone(), artifact.version.clone()))
                {
                    if project.packaging == "pom" {
                        continue;
                    }
                }
                return Err(e.context(format!("failed to download JAR for {artifact}")));
            }
        }
    }

    // Return as resolved artifacts (caller can get jar paths from cache)
    Ok(artifacts)
}

/// Build a classpath from resolved artifacts.
pub fn resolve_classpath(
    coordinates: &[String],
    repos: &[Repository],
    cache_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let artifacts = resolve(coordinates, repos, cache_dir)?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for artifact in &artifacts {
        let jar_name = format!("{}-{}.jar", artifact.module.name, artifact.version);
        let jar_path = cache_dir.join(&jar_name);
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
            "invalid Maven coordinate '{coord}' (expected groupId:artifactId:version)"
        ));
    }
    Ok(Dependency {
        module: Module {
            org: parts[0].to_string(),
            name: parts[1].to_string(),
        },
        version: parts[2].to_string(),
        scope: Scope::Compile,
        optional: false,
        exclusions: HashSet::new(),
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

    // Handle relocation
    if let Some(ref relocation) = project.relocation {
        deps.push(relocation.clone());
    }

    deps
}

/// Resolve the parent POM chain, merging inherited properties and dependencyManagement.
fn resolve_parent_chain(
    project: &Project,
    repos: &[Repository],
    cache: &mut HashMap<(Module, String), Project>,
) -> Result<Project> {
    let mut effective = project.clone();

    if let Some((ref parent_module, ref parent_version)) = project.parent {
        let cache_key = (parent_module.clone(), parent_version.clone());
        let parent = if let Some(p) = cache.get(&cache_key) {
            p.clone()
        } else {
            let p = fetch_pom(parent_module, parent_version, repos)?;
            let resolved = resolve_parent_chain(&p, repos, cache)?;
            cache.insert(cache_key, resolved.clone());
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

    // Re-apply property substitution with merged properties
    let substitute = |s: &str, props: &HashMap<String, String>| -> String {
        let mut result = s.to_string();
        for (key, value) in props {
            let pattern = format!("${{{key}}}");
            if result.contains(&pattern) {
                result = result.replace(&pattern, value);
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

/// Pick the higher of two Maven version strings.
/// Simplified: compare segment-by-segment (numeric where possible).
fn pick_higher_version(a: &str, b: &str) -> String {
    if a == b {
        return a.to_string();
    }
    let pa = parse_version_parts(a);
    let pb = parse_version_parts(b);
    for (sa, sb) in pa.iter().zip(pb.iter()) {
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
    // If all compared parts are equal, the longer version wins (1.0.0 > 1.0)
    if pa.len() >= pb.len() {
        a.to_string()
    } else {
        b.to_string()
    }
}

fn parse_version_parts(v: &str) -> Vec<String> {
    v.split(['.', '-']).map(|s| s.to_string()).collect()
}

/// Download a JAR to the cache directory.
fn download_jar(
    artifact: &ResolvedArtifact,
    repos: &[Repository],
    cache_dir: &Path,
) -> Result<PathBuf> {
    let jar_name = format!("{}-{}.jar", artifact.module.name, artifact.version);
    let jar_path = cache_dir.join(&jar_name);

    if jar_path.exists() {
        return Ok(jar_path);
    }

    for repo in repos {
        let url = repo.jar_url(&artifact.module, &artifact.version);
        match ureq::get(&url).call() {
            Ok(response) => {
                let mut body = response.into_reader();
                let mut file = std::fs::File::create(&jar_path)
                    .context(format!("failed to create {jar_path:?}"))?;
                std::io::copy(&mut body, &mut file)?;
                return Ok(jar_path);
            }
            Err(_) => continue,
        }
    }

    Err(anyhow!("JAR for {artifact} not found in any repository"))
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
    fn parses_pom_with_parent() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <parent>
    <groupId>com.example</groupId>
    <artifactId>parent-pom</artifactId>
    <version>1.0.0</version>
  </parent>
  <artifactId>child-module</artifactId>
  <version>1.0.0</version>
</project>"#;
        let project = parse_pom(xml).unwrap();
        assert!(project.parent.is_some());
        let (parent_module, parent_version) = project.parent.unwrap();
        assert_eq!(parent_module.org, "com.example");
        assert_eq!(parent_module.name, "parent-pom");
        assert_eq!(parent_version, "1.0.0");
    }
}
