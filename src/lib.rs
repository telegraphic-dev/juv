use anyhow::{anyhow, Context, Result};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyValue {
    pub key: String,
    pub value: Option<String>,
}

impl KeyValue {
    pub fn parse(text: &str) -> Self {
        match text.split_once('=') {
            Some((key, value)) => Self {
                key: key.to_string(),
                value: Some(value.to_string()),
            },
            None => Self {
                key: text.to_string(),
                value: None,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Directives {
    pub deps: Vec<String>,
    pub repos: Vec<String>,
    pub sources: Vec<String>,
    pub files: Vec<String>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub native_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub manifest_options: Vec<KeyValue>,
    pub docs: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
    pub module: Option<String>,
    pub gav: Option<String>,
    pub description: Option<String>,
    pub enable_preview: bool,
    pub enable_cds: bool,
    pub disable_integrations: bool,
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub script: PathBuf,
    pub script_args: Vec<String>,
    pub extra_deps: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub main_class: Option<String>,
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub script: PathBuf,
    pub extra_deps: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub main_class: Option<String>,
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub classes_dir: PathBuf,
    pub classpath: Vec<PathBuf>,
    pub main_class: Option<String>,
    pub directives: Directives,
}

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub script: PathBuf,
    pub deps: Vec<String>,
    pub java_version: Option<String>,
    pub force: bool,
}

pub fn init_script(options: InitOptions) -> Result<PathBuf> {
    if options.script.exists() && !options.force {
        return Err(anyhow!(
            "file {} already exists; use --force to overwrite",
            options.script.display()
        ));
    }

    let base_name = options
        .script
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            anyhow!(
                "could not infer class name from {}",
                options.script.display()
            )
        })?;
    if !is_java_identifier(base_name) {
        return Err(anyhow!(
            "'{base_name}' is not a valid class name in Java; use a Java identifier filename"
        ));
    }

    if let Some(parent) = options
        .script
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(
        &options.script,
        render_default_init_script(base_name, &options),
    )?;
    Ok(options.script)
}

pub fn default_cache_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .ok_or_else(|| anyhow!("could not determine cache directory"))?
        .join("doj"))
}

pub fn clear_cache(cache_dir: Option<&Path>) -> Result<()> {
    let root = match cache_dir {
        Some(path) => path.to_path_buf(),
        None => default_cache_dir()?,
    };
    if root.exists() {
        fs::remove_dir_all(&root)
            .with_context(|| format!("failed to clear cache {}", root.display()))?;
    }
    Ok(())
}

fn render_default_init_script(base_name: &str, options: &InitOptions) -> String {
    let mut out = String::from("///usr/bin/env jbang \"$0\" \"$@\" ; exit $?\n");
    if let Some(version) = &options.java_version {
        out.push_str(&format!("//JAVA {version}\n"));
    }
    for dep in &options.deps {
        out.push_str(&format!("//DEPS {dep}\n"));
    }
    if options.deps.is_empty() {
        out.push_str("// //DEPS <dependency1> <dependency2>\n");
    }
    out.push_str("import static java.lang.System.*;\n\n");
    out.push_str(&format!(
        "public class {base_name} {{\n\n    public static void main(String... args) {{\n        out.println(\"Hello World\");\n    }}\n}}\n"
    ));
    out
}

fn is_java_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first == '$' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric())
}

pub fn parse_directives(source: &str) -> Directives {
    let mut directives = Directives::default();
    let directive_re =
        Regex::new(r"^(?://)?(?P<key>(?:[A-Z]+:)?[A-Z_]+)(?:\s+(?P<value>.*?))?(?:\s//\s.*)?$")
            .expect("valid directive regex");

    for raw in source.lines() {
        let line = raw.trim_start();
        let Some(stripped) = line.strip_prefix("//") else {
            continue;
        };
        let Some(caps) = directive_re.captures(stripped.trim_start()) else {
            continue;
        };
        let key = caps.name("key").map(|m| m.as_str()).unwrap_or_default();
        let value = caps
            .name("value")
            .map(|m| m.as_str().trim())
            .unwrap_or_default();

        match key {
            "DEPS" => directives.deps.extend(split_directive_words(value)),
            "REPOS" => directives.repos.extend(split_directive_words(value)),
            "SOURCES" => directives.sources.extend(split_directive_words(value)),
            "FILES" => directives.files.extend(split_directive_words(value)),
            "JAVAC_OPTIONS" | "COMPILE_OPTIONS" => {
                directives.javac_options.extend(split_space_words(value))
            }
            "RUNTIME_OPTIONS" | "JAVA_OPTIONS" => {
                directives.runtime_options.extend(split_space_words(value))
            }
            "NATIVE_OPTIONS" => directives.native_options.extend(split_space_words(value)),
            "JAVAAGENT" => directives.java_agents.extend(
                split_space_words(value)
                    .iter()
                    .map(|word| KeyValue::parse(word)),
            ),
            "MANIFEST" => directives.manifest_options.extend(
                split_space_words(value)
                    .iter()
                    .map(|word| KeyValue::parse(word)),
            ),
            "DOCS" => directives.docs.push(KeyValue::parse(value)),
            "JAVA" => directives.java_version = Some(value.to_string()),
            "MAIN" => directives.main_class = Some(value.to_string()),
            "MODULE" => directives.module = Some(value.to_string()),
            "GAV" => directives.gav = Some(value.to_string()),
            "DESCRIPTION" => {
                directives.description = Some(match directives.description.take() {
                    Some(existing) => format!("{existing}\n{value}"),
                    None => value.to_string(),
                });
            }
            "PREVIEW" => directives.enable_preview = true,
            "CDS" => directives.enable_cds = true,
            "NOINTEGRATIONS" => directives.disable_integrations = true,
            _ => {}
        }
    }
    directives
}

pub fn split_directive_words(text: &str) -> Vec<String> {
    split_words(text, true)
}

fn split_space_words(text: &str) -> Vec<String> {
    split_words(text, false)
}

fn split_words(text: &str, comma_semicolon_are_separators: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => cur.push(ch),
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None if ch.is_whitespace()
                || (comma_semicolon_are_separators && (ch == ',' || ch == ';')) =>
            {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn build_java(options: BuildOptions) -> Result<BuildOutput> {
    let script = fs::canonicalize(&options.script)
        .with_context(|| format!("script not found: {}", options.script.display()))?;
    let source = fs::read_to_string(&script)
        .with_context(|| format!("failed to read {}", script.display()))?;
    let mut directives = parse_directives(&source);
    directives.deps.extend(options.extra_deps);
    directives.javac_options.extend(options.javac_options);
    if options.main_class.is_some() {
        directives.main_class = options.main_class;
    }

    let work_dir = cache_project_dir(options.cache_dir.as_deref(), &script, &source)?;
    fs::create_dir_all(&work_dir)?;
    let classes_dir = work_dir.join("classes");
    if classes_dir.exists() {
        fs::remove_dir_all(&classes_dir)?;
    }
    fs::create_dir_all(&classes_dir)?;

    let base_dir = script.parent().unwrap_or_else(|| Path::new("."));
    let (binary_deps, source_deps): (Vec<_>, Vec<_>) = directives
        .deps
        .iter()
        .cloned()
        .partition(|dep| looks_like_binary_dependency(dep));
    let mut sources = vec![script.clone()];
    for extra in directives.sources.iter().chain(source_deps.iter()) {
        sources.push(base_dir.join(extra));
    }

    let dep_cp = resolve_dependencies(&binary_deps, &directives.repos, &work_dir)?;
    let mut cp_entries = options.classpath;
    cp_entries.extend(dep_cp);

    let javac = javac_for(&directives.java_version);
    let mut javac_cmd = Command::new(&javac);
    javac_cmd.arg("-d").arg(&classes_dir);
    if !cp_entries.is_empty() {
        javac_cmd.arg("-classpath").arg(join_classpath(&cp_entries));
    }
    javac_cmd.args(&directives.javac_options);
    if directives.enable_preview {
        if !directives
            .javac_options
            .iter()
            .any(|o| o == "--enable-preview")
        {
            javac_cmd.arg("--enable-preview");
        }
        if !has_source_or_release_option(&directives.javac_options) {
            javac_cmd.arg("--release").arg(javac_major_version(&javac)?);
        }
    }
    javac_cmd.args(&sources);
    let status = javac_cmd
        .status()
        .with_context(|| format!("failed to execute {javac}"))?;
    if !status.success() {
        return Err(anyhow!(
            "javac failed with exit code {}",
            status.code().unwrap_or(1)
        ));
    }

    copy_declared_files(base_dir, &classes_dir, &directives.files)?;

    let main_class = directives
        .main_class
        .clone()
        .or_else(|| infer_main_class(&script, &source));

    Ok(BuildOutput {
        classes_dir,
        classpath: cp_entries,
        main_class,
        directives,
    })
}

pub fn run_java(options: RunOptions) -> Result<i32> {
    let script_args = options.script_args;
    let runtime_options = options.runtime_options;
    let build = build_java(BuildOptions {
        script: options.script,
        extra_deps: options.extra_deps,
        classpath: options.classpath,
        javac_options: options.javac_options,
        main_class: options.main_class,
        cache_dir: options.cache_dir,
    })?;

    let main_class = build.main_class.ok_or_else(|| {
        anyhow!("could not infer main class; add //MAIN fully.qualified.ClassName")
    })?;

    let java = java_for(&build.directives.java_version);
    let mut runtime_cp = vec![build.classes_dir];
    runtime_cp.extend(build.classpath);
    let mut java_cmd = Command::new(&java);
    java_cmd.args(&build.directives.runtime_options);
    java_cmd.args(&runtime_options);
    if build.directives.enable_preview
        && !build
            .directives
            .runtime_options
            .iter()
            .chain(runtime_options.iter())
            .any(|o| o == "--enable-preview")
    {
        java_cmd.arg("--enable-preview");
    }
    java_cmd.arg("-cp").arg(join_classpath(&runtime_cp));
    java_cmd.arg(main_class);
    java_cmd.args(script_args);
    let status = java_cmd
        .status()
        .with_context(|| format!("failed to execute {java}"))?;
    Ok(status.code().unwrap_or(1))
}

fn cache_project_dir(cache_dir: Option<&Path>, script: &Path, source: &str) -> Result<PathBuf> {
    let root = match cache_dir {
        Some(path) => path.to_path_buf(),
        None => default_cache_dir()?,
    };
    let mut hasher = Sha256::new();
    hasher.update(script.to_string_lossy().as_bytes());
    hasher.update(source.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    Ok(root.join(&hash[..16]))
}

fn looks_like_binary_dependency(dep: &str) -> bool {
    dep.matches(':').count() >= 2 && !dep.ends_with(".java")
}

fn resolve_dependencies(
    deps: &[String],
    repos: &[String],
    work_dir: &Path,
) -> Result<Vec<PathBuf>> {
    if deps.is_empty() {
        return Ok(Vec::new());
    }
    let Some(coursier) = find_command(&["cs", "coursier"]) else {
        return Err(anyhow!(
            "//DEPS requires Coursier (`cs` or `coursier`) on PATH for now"
        ));
    };
    let cp_file = work_dir.join("classpath.txt");
    let mut cmd = Command::new(coursier);
    cmd.arg("fetch").arg("--classpath-file").arg(&cp_file);
    for repo in repos {
        cmd.arg("--repository").arg(repo);
    }
    cmd.args(deps);
    let status = cmd.status().context("failed to execute Coursier")?;
    if !status.success() {
        return Ok(Vec::new());
    }
    let cp = fs::read_to_string(cp_file)?;
    Ok(split_classpath(&cp))
}

fn find_command(names: &[&str]) -> Option<String> {
    for name in names {
        if Command::new(name).arg("--help").output().is_ok() {
            return Some((*name).to_string());
        }
    }
    None
}

fn javac_for(java_version: &Option<String>) -> String {
    versioned_tool("javac", java_version)
}

fn java_for(java_version: &Option<String>) -> String {
    versioned_tool("java", java_version)
}

fn has_source_or_release_option(options: &[String]) -> bool {
    options.iter().any(|option| {
        matches!(
            option.as_str(),
            "--release" | "-release" | "--source" | "-source" | "-sourcepath" | "--source-path"
        ) || option.starts_with("--release=")
            || option.starts_with("--source=")
            || option.starts_with("-source")
    })
}

fn javac_major_version(javac: &str) -> Result<String> {
    let output = Command::new(javac)
        .arg("-version")
        .output()
        .with_context(|| format!("failed to execute {javac} -version"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let version_re = Regex::new(r"javac\s+(?:1\.)?(\d+)").expect("valid javac version regex");
    version_re
        .captures(&text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("could not determine javac version from: {text}"))
}

fn versioned_tool(tool: &str, java_version: &Option<String>) -> String {
    if let Some(version) = java_version {
        let sdkman = PathBuf::from(format!(
            "{}/.sdkman/candidates/java/current/bin/{tool}",
            std::env::var("HOME").unwrap_or_default()
        ));
        if sdkman.exists() {
            return sdkman.display().to_string();
        }
        let common = [
            format!("/usr/lib/jvm/java-{version}-openjdk/bin/{tool}"),
            format!("/usr/lib/jvm/java-{version}-openjdk-amd64/bin/{tool}"),
            format!("/usr/lib/jvm/java-{version}-openjdk-arm64/bin/{tool}"),
        ];
        for candidate in common {
            if Path::new(&candidate).exists() {
                return candidate;
            }
        }
    }
    tool.to_string()
}

fn copy_declared_files(base_dir: &Path, classes_dir: &Path, files: &[String]) -> Result<()> {
    for file_ref in files {
        let (target, source) = split_file_ref(file_ref);
        if source.is_absolute() || target.as_ref().is_some_and(|p| p.is_absolute()) {
            return Err(anyhow!(
                "only relative paths are allowed in //FILES: {file_ref}"
            ));
        }

        let source = base_dir.join(source);
        if source.is_dir() {
            copy_resource_dir(&source, classes_dir, target.as_deref())?;
        } else {
            let target = target.unwrap_or_else(|| {
                source
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(file_ref))
            });
            copy_resource_file(&source, &classes_dir.join(target))?;
        }
    }
    Ok(())
}

fn split_file_ref(file_ref: &str) -> (Option<PathBuf>, PathBuf) {
    match file_ref.split_once('=') {
        Some((target, source)) => (Some(PathBuf::from(target)), PathBuf::from(source)),
        None => (None, PathBuf::from(file_ref)),
    }
}

fn copy_resource_dir(source_dir: &Path, classes_dir: &Path, target: Option<&Path>) -> Result<()> {
    for entry in walkdir::WalkDir::new(source_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(source_dir)?;
        let dest = match target {
            Some(target) => classes_dir.join(target).join(rel),
            None => classes_dir.join(rel),
        };
        copy_resource_file(entry.path(), &dest)?;
    }
    Ok(())
}

fn copy_resource_file(source: &Path, dest: &Path) -> Result<()> {
    let parent = dest
        .parent()
        .ok_or_else(|| anyhow!("invalid resource destination: {}", dest.display()))?;
    fs::create_dir_all(parent)?;
    fs::copy(source, dest).with_context(|| {
        format!(
            "failed to copy //FILES resource {} to {}",
            source.display(),
            dest.display()
        )
    })?;
    Ok(())
}

fn infer_main_class(script: &Path, source: &str) -> Option<String> {
    let simple_name = script
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())?;
    match package_name(source) {
        Some(package) => Some(format!("{package}.{simple_name}")),
        None => Some(simple_name),
    }
}

fn package_name(source: &str) -> Option<String> {
    let package_re =
        Regex::new(r"(?m)^\s*package\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*;")
            .expect("valid package regex");
    package_re
        .captures(source)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

fn join_classpath(paths: &[PathBuf]) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    paths
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join(sep)
}

fn split_classpath(text: &str) -> Vec<PathBuf> {
    let sep = if cfg!(windows) { ';' } else { ':' };
    text.trim()
        .split(sep)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}
