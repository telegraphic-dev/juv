use anyhow::{anyhow, Context, Result};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::write::SimpleFileOptions;

pub mod jdk;
pub mod maven_tool;
pub mod resolver;

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
    pub runtime_deps: Vec<String>,
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
    pub extra_repos: Vec<String>,
    pub extra_sources: Vec<String>,
    pub extra_files: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub trust_remote: bool,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub script: PathBuf,
    pub extra_deps: Vec<String>,
    pub extra_repos: Vec<String>,
    pub extra_sources: Vec<String>,
    pub extra_files: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub trust_remote: bool,
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
    pub template: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogAlias {
    pub name: String,
    pub script_ref: String,
    pub script: PathBuf,
    pub description: Option<String>,
    pub arguments: Vec<String>,
    pub deps: Vec<String>,
    pub repos: Vec<String>,
    pub sources: Vec<String>,
    pub files: Vec<String>,
    pub classpaths: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AliasAddOptions {
    pub script_ref: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub arguments: Vec<String>,
    pub deps: Vec<String>,
    pub repos: Vec<String>,
    pub sources: Vec<String>,
    pub files: Vec<String>,
    pub classpaths: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub docs: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
    pub force: bool,
    pub catalog_file: Option<PathBuf>,
    pub global: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AliasRemoveOptions {
    pub name: String,
    pub catalog_file: Option<PathBuf>,
    pub global: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogRefEntry {
    pub name: String,
    pub catalog_ref: String,
    pub catalog: PathBuf,
    pub description: Option<String>,
    pub import_items: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CatalogAddOptions {
    pub name: String,
    pub catalog_ref: String,
    pub description: Option<String>,
    pub import_items: bool,
    pub force: bool,
    pub catalog_file: Option<PathBuf>,
    pub global: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogTemplate {
    pub name: String,
    pub description: Option<String>,
    pub file_refs: Vec<(String, String)>,
    pub properties: Vec<(String, Option<String>)>,
    pub catalog_dir: PathBuf,
    pub base_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub script: PathBuf,
    pub classes_dir: PathBuf,
    pub cache_dir: PathBuf,
}

pub fn init_script(options: InitOptions) -> Result<PathBuf> {
    if options.script.exists() && !options.force {
        return Err(anyhow!(
            "file {} already exists; use --force to overwrite",
            options.script.display()
        ));
    }

    let builtin_template = init_template(options.template.as_deref());
    if let Ok(template) = builtin_template.as_ref() {
        validate_init_java_version(*template, options.java_version.as_deref())?;
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
    let class_name = match builtin_template.as_ref() {
        Ok(template) if template.name == "test" => java_class_name_from_stem(base_name),
        _ => base_name.to_string(),
    };
    if !is_java_identifier(&class_name) {
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

    let content = match builtin_template {
        Ok(template) => render_init_script(template, &class_name, &options),
        Err(builtin_error) => match options.template.as_deref() {
            Some(name) => {
                let Some(template) = resolve_catalog_template(name, &std::env::current_dir()?)?
                else {
                    return Err(builtin_error);
                };
                render_catalog_template(&template, base_name, &options)?
            }
            None => return Err(builtin_error),
        },
    };
    fs::write(&options.script, content)?;
    Ok(options.script)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitTemplate {
    pub name: &'static str,
    pub description: &'static str,
}

pub const INIT_TEMPLATES: &[InitTemplate] = &[
    InitTemplate {
        name: "hello",
        description: "Basic Java 25 unnamed-class Hello World script",
    },
    InitTemplate {
        name: "java",
        description: "Alias for hello",
    },
    InitTemplate {
        name: "compact",
        description: "Java 25 compact-source Hello World script",
    },
    InitTemplate {
        name: "cli",
        description: "Picocli command-line application",
    },
    InitTemplate {
        name: "agent",
        description: "Java agent skeleton",
    },
    InitTemplate {
        name: "test",
        description: "JUnit test class",
    },
];

pub fn init_templates() -> &'static [InitTemplate] {
    INIT_TEMPLATES
}

fn validate_init_java_version(template: InitTemplate, version: Option<&str>) -> Result<()> {
    let Some(version) = version else {
        return Ok(());
    };
    let major = jdk::parse_java_version_directive(version)?;
    if matches!(template.name, "hello" | "java" | "compact") && major < 25 {
        return Err(anyhow!(
            "template '{}' uses Java 25 unnamed classes; use --java 25+ or choose a class-based template",
            template.name
        ));
    }
    Ok(())
}

fn init_template(template: Option<&str>) -> Result<InitTemplate> {
    let name = template.unwrap_or("hello");
    INIT_TEMPLATES
        .iter()
        .copied()
        .find(|template| template.name == name)
        .ok_or_else(|| {
            anyhow!(
                "unknown init template '{name}'; supported templates: {}",
                INIT_TEMPLATES
                    .iter()
                    .map(|template| template.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

pub fn default_cache_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .ok_or_else(|| anyhow!("could not determine cache directory"))?
        .join("jbx"))
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

pub fn catalog_aliases(start_dir: &Path) -> Result<Vec<CatalogAlias>> {
    let Some(catalog_path) = find_catalog_file(start_dir) else {
        return Ok(Vec::new());
    };
    let mut seen = HashSet::new();
    read_catalog_aliases_recursive(&catalog_path, &mut seen)
}

pub fn resolve_catalog_alias(name: &str, start_dir: &Path) -> Result<Option<CatalogAlias>> {
    Ok(catalog_aliases(start_dir)?
        .into_iter()
        .find(|alias| alias.name == name))
}

pub fn catalog_refs(start_dir: &Path) -> Result<Vec<CatalogRefEntry>> {
    let Some(catalog_path) = find_catalog_file(start_dir) else {
        return Ok(Vec::new());
    };
    read_catalog_refs(&catalog_path)
}

pub fn catalog_add(options: CatalogAddOptions, start_dir: &Path) -> Result<PathBuf> {
    validate_catalog_entry_name(&options.name, "catalog")?;
    let catalog_path =
        writable_catalog_file(options.catalog_file.as_deref(), options.global, start_dir)?;
    let catalog_dir = catalog_path.parent().unwrap_or_else(|| Path::new("."));
    let description = match options.description {
        Some(description) => Some(description),
        None => read_catalog_description_from_ref(catalog_dir, &options.catalog_ref)
            .ok()
            .flatten(),
    };
    let mut catalog = read_catalog_json_for_write(&catalog_path)?;
    let catalogs = ensure_json_object(&mut catalog, "catalogs")?;
    if catalogs.contains_key(&options.name) && !options.force {
        return Err(anyhow!(
            "catalog '{}' already exists in {}; use --force to overwrite",
            options.name,
            catalog_path.display()
        ));
    }
    let mut entry = serde_json::Map::new();
    entry.insert(
        "catalog-ref".to_string(),
        serde_json::Value::String(options.catalog_ref),
    );
    insert_optional_string(&mut entry, "description", description);
    if options.import_items {
        entry.insert("import".to_string(), serde_json::Value::Bool(true));
    }
    catalogs.insert(options.name, serde_json::Value::Object(entry));
    write_catalog_json(&catalog_path, &catalog)?;
    Ok(catalog_path)
}

pub fn catalog_templates(start_dir: &Path) -> Result<Vec<CatalogTemplate>> {
    let mut templates = INIT_TEMPLATES
        .iter()
        .map(|template| CatalogTemplate {
            name: template.name.to_string(),
            description: Some(template.description.to_string()),
            file_refs: Vec::new(),
            properties: Vec::new(),
            catalog_dir: PathBuf::new(),
            base_ref: None,
        })
        .collect::<Vec<_>>();
    if let Some(catalog_path) = find_catalog_file(start_dir) {
        let mut seen = HashSet::new();
        templates.extend(read_catalog_templates_recursive(&catalog_path, &mut seen)?);
    }
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

pub fn resolve_catalog_template(name: &str, start_dir: &Path) -> Result<Option<CatalogTemplate>> {
    if let Some((template_name, catalog_name)) = name.split_once('@') {
        let Some(catalog) = catalog_refs(start_dir)?
            .into_iter()
            .find(|catalog| catalog.name == catalog_name)
        else {
            return Ok(None);
        };
        return Ok(read_catalog_templates(&catalog.catalog)?
            .into_iter()
            .find(|template| template.name == template_name));
    }
    Ok(catalog_templates(start_dir)?
        .into_iter()
        .find(|template| template.name == name))
}

pub fn alias_add(options: AliasAddOptions, start_dir: &Path) -> Result<PathBuf> {
    let name = match options.name {
        Some(name) => name,
        None => alias_name_from_ref(&options.script_ref)?,
    };
    validate_catalog_entry_name(&name, "alias")?;

    let catalog_path =
        writable_catalog_file(options.catalog_file.as_deref(), options.global, start_dir)?;
    let mut catalog = read_catalog_json_for_write(&catalog_path)?;
    let aliases = ensure_json_object(&mut catalog, "aliases")?;
    if aliases.contains_key(&name) && !options.force {
        return Err(anyhow!(
            "alias '{name}' already exists in {}; use --force to overwrite",
            catalog_path.display()
        ));
    }

    let mut alias = serde_json::Map::new();
    alias.insert(
        "script-ref".to_string(),
        serde_json::Value::String(options.script_ref),
    );
    insert_optional_string(&mut alias, "description", options.description);
    insert_string_list(&mut alias, "arguments", options.arguments);
    insert_string_list(&mut alias, "dependencies", options.deps);
    insert_string_list(&mut alias, "repositories", options.repos);
    insert_string_list(&mut alias, "sources", options.sources);
    insert_string_list(&mut alias, "files", options.files);
    insert_string_list(
        &mut alias,
        "classpaths",
        options
            .classpaths
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    );
    insert_string_list(&mut alias, "compile-options", options.javac_options);
    insert_string_list(&mut alias, "runtime-options", options.runtime_options);
    insert_string_list(
        &mut alias,
        "java-agents",
        options
            .java_agents
            .into_iter()
            .map(|kv| match kv.value {
                Some(value) => format!("{}={value}", kv.key),
                None => kv.key,
            })
            .collect(),
    );
    insert_string_list(
        &mut alias,
        "docs",
        options
            .docs
            .into_iter()
            .map(|kv| match kv.value {
                Some(value) => format!("{}={value}", kv.key),
                None => kv.key,
            })
            .collect(),
    );
    insert_optional_string(&mut alias, "java", options.java_version);
    insert_optional_string(&mut alias, "main", options.main_class);

    aliases.insert(name, serde_json::Value::Object(alias));
    write_catalog_json(&catalog_path, &catalog)?;
    Ok(catalog_path)
}

pub fn alias_remove(options: AliasRemoveOptions, start_dir: &Path) -> Result<bool> {
    validate_catalog_entry_name(&options.name, "alias")?;
    let catalog_path =
        writable_catalog_file(options.catalog_file.as_deref(), options.global, start_dir)?;
    if !catalog_path.is_file() {
        return Ok(false);
    }
    let mut catalog = read_catalog_json_for_write(&catalog_path)?;
    let removed = catalog
        .get_mut("aliases")
        .and_then(|value| value.as_object_mut())
        .and_then(|aliases| aliases.remove(&options.name))
        .is_some();
    if removed {
        write_catalog_json(&catalog_path, &catalog)?;
    }
    Ok(removed)
}

fn find_catalog_file(start_dir: &Path) -> Option<PathBuf> {
    let mut current = if start_dir.is_file() {
        start_dir.parent()?.to_path_buf()
    } else {
        start_dir.to_path_buf()
    };
    loop {
        let visible = current.join("jbang-catalog.json");
        if visible.is_file() {
            return Some(visible);
        }
        let hidden = current.join(".jbang").join("jbang-catalog.json");
        if hidden.is_file() {
            return Some(hidden);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn writable_catalog_file(
    catalog_file: Option<&Path>,
    global: bool,
    start_dir: &Path,
) -> Result<PathBuf> {
    if global {
        return Ok(dirs::home_dir()
            .ok_or_else(|| anyhow!("could not determine home directory"))?
            .join(".jbang")
            .join("jbang-catalog.json"));
    }
    if let Some(path) = catalog_file {
        if path.is_dir() {
            let visible = path.join("jbang-catalog.json");
            let hidden = path.join(".jbang").join("jbang-catalog.json");
            if !visible.exists() && hidden.exists() {
                return Ok(hidden);
            }
            return Ok(visible);
        }
        return Ok(path.to_path_buf());
    }
    Ok(find_catalog_file(start_dir).unwrap_or_else(|| start_dir.join("jbang-catalog.json")))
}

fn read_catalog_json_for_write(path: &Path) -> Result<serde_json::Value> {
    if path.exists() {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read catalog {}", path.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("failed to parse catalog {}", path.display()))
    } else {
        Ok(serde_json::json!({}))
    }
}

fn write_catalog_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("failed to write catalog {}", path.display()))
}

fn ensure_json_object<'a>(
    value: &'a mut serde_json::Value,
    key: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>> {
    if !value.is_object() {
        return Err(anyhow!("catalog root is not a JSON object"));
    }
    let root = value.as_object_mut().expect("catalog root is object");
    if !root.get(key).is_some_and(|value| value.is_object()) {
        root.insert(key.to_string(), serde_json::json!({}));
    }
    Ok(root
        .get_mut(key)
        .and_then(|value| value.as_object_mut())
        .expect("catalog section is object"))
}

fn insert_optional_string(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        map.insert(key.to_string(), serde_json::Value::String(value));
    }
}

fn insert_string_list(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    values: Vec<String>,
) {
    if !values.is_empty() {
        map.insert(
            key.to_string(),
            serde_json::Value::Array(values.into_iter().map(serde_json::Value::String).collect()),
        );
    }
}

fn alias_name_from_ref(script_ref: &str) -> Result<String> {
    let without_query = script_ref.split(['?', '#']).next().unwrap_or(script_ref);
    let file_name = without_query
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(without_query);
    let stem = ["java", "kt", "groovy", "jsh", "jav"]
        .iter()
        .find_map(|ext| file_name.strip_suffix(&format!(".{ext}")))
        .unwrap_or(file_name);
    if stem.is_empty() {
        Err(anyhow!(
            "could not infer alias name from {script_ref}; pass --name"
        ))
    } else {
        Ok(stem.to_string())
    }
}

fn validate_catalog_entry_name(name: &str, kind: &str) -> Result<()> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(anyhow!("{kind} name must not be empty"));
    };
    if !first.is_ascii_alphabetic()
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(anyhow!(
            "invalid {kind} name '{name}'; use a letter followed by letters, digits, underscores or hyphens"
        ));
    }
    Ok(())
}

fn read_catalog_value(catalog_path: &Path) -> Result<serde_json::Value> {
    let text = if is_remote_url(&catalog_path.to_string_lossy()) {
        fetch_remote_script(&catalog_path.to_string_lossy())?
    } else {
        fs::read_to_string(catalog_path)
            .with_context(|| format!("failed to read catalog {}", catalog_path.display()))?
    };
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse catalog {}", catalog_path.display()))
}

fn read_catalog_description_from_ref(
    catalog_dir: &Path,
    catalog_ref: &str,
) -> Result<Option<String>> {
    let catalog_path = resolve_catalog_file_ref(catalog_dir, catalog_ref);
    Ok(read_catalog_value(&catalog_path)?
        .get("description")
        .and_then(|value| value.as_str())
        .map(str::to_string))
}

fn read_catalog_refs(catalog_path: &Path) -> Result<Vec<CatalogRefEntry>> {
    let json = read_catalog_value(catalog_path)?;
    let Some(catalogs) = json.get("catalogs").and_then(|value| value.as_object()) else {
        return Ok(Vec::new());
    };
    let catalog_dir = catalog_path.parent().unwrap_or_else(|| Path::new("."));
    let mut out = Vec::new();
    for (name, value) in catalogs {
        let Some(catalog_ref) = value
            .get("catalog-ref")
            .or_else(|| value.get("catalogRef"))
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        out.push(CatalogRefEntry {
            name: name.to_string(),
            catalog_ref: catalog_ref.to_string(),
            catalog: resolve_catalog_file_ref(catalog_dir, catalog_ref),
            description: value
                .get("description")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            import_items: value
                .get("import")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn read_catalog_aliases_recursive(
    catalog_path: &Path,
    seen: &mut HashSet<PathBuf>,
) -> Result<Vec<CatalogAlias>> {
    if !seen.insert(catalog_seen_key(catalog_path)) {
        return Ok(Vec::new());
    }
    let mut out = read_catalog_aliases(catalog_path)?;
    for catalog in read_catalog_refs(catalog_path)? {
        if catalog.import_items {
            out.extend(read_catalog_aliases_recursive(&catalog.catalog, seen)?);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn read_catalog_aliases(catalog_path: &Path) -> Result<Vec<CatalogAlias>> {
    let json = read_catalog_value(catalog_path)?;
    let Some(aliases) = json.get("aliases").and_then(|value| value.as_object()) else {
        return Ok(Vec::new());
    };
    let catalog_dir = catalog_path.parent().unwrap_or_else(|| Path::new("."));
    let base_ref = json
        .get("base-ref")
        .or_else(|| json.get("baseRef"))
        .and_then(|value| value.as_str());
    let mut out = Vec::new();
    for (name, value) in aliases {
        let Some(script_ref) = value
            .get("script-ref")
            .or_else(|| value.get("scriptRef"))
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        out.push(CatalogAlias {
            name: name.to_string(),
            script_ref: script_ref.to_string(),
            script: resolve_catalog_script_ref(catalog_dir, base_ref, script_ref),
            description: value
                .get("description")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            arguments: json_string_list(value, "arguments"),
            deps: json_string_list(value, "dependencies"),
            repos: json_string_list(value, "repositories"),
            sources: json_string_list(value, "sources"),
            files: json_string_list(value, "files"),
            classpaths: json_string_list(value, "classpaths")
                .into_iter()
                .map(PathBuf::from)
                .collect(),
            javac_options: json_string_list(value, "compile-options"),
            runtime_options: json_string_list(value, "runtime-options")
                .into_iter()
                .chain(json_string_list(value, "java-options"))
                .collect(),
            java_agents: json_string_list(value, "java-agents")
                .into_iter()
                .map(|agent| KeyValue::parse(&agent))
                .collect(),
            java_version: value
                .get("java")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            main_class: value
                .get("main")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn json_string_list(value: &serde_json::Value, key: &str) -> Vec<String> {
    match value.get(key) {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Some(serde_json::Value::String(text)) => split_directive_words(text),
        _ => Vec::new(),
    }
}

fn resolve_catalog_script_ref(
    catalog_dir: &Path,
    base_ref: Option<&str>,
    script_ref: &str,
) -> PathBuf {
    if is_remote_url(script_ref) || Path::new(script_ref).is_absolute() {
        return PathBuf::from(script_ref);
    }
    let base = match base_ref {
        Some(base) if is_remote_url(base) => return PathBuf::from(join_url_path(base, script_ref)),
        Some(base) if Path::new(base).is_absolute() => PathBuf::from(base),
        Some(base) => catalog_dir.join(base),
        None => catalog_dir.to_path_buf(),
    };
    base.join(script_ref)
}

fn join_url_path(base: &str, child: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        child.trim_start_matches('/')
    )
}

fn resolve_catalog_file_ref(catalog_dir: &Path, catalog_ref: &str) -> PathBuf {
    let path = if is_remote_url(catalog_ref) || Path::new(catalog_ref).is_absolute() {
        PathBuf::from(catalog_ref)
    } else {
        catalog_dir.join(catalog_ref)
    };
    if path.file_name().and_then(|name| name.to_str()) == Some("jbang-catalog.json") {
        path
    } else {
        path.join("jbang-catalog.json")
    }
}

fn read_catalog_templates_recursive(
    catalog_path: &Path,
    seen: &mut HashSet<PathBuf>,
) -> Result<Vec<CatalogTemplate>> {
    if !seen.insert(catalog_seen_key(catalog_path)) {
        return Ok(Vec::new());
    }
    let mut out = read_catalog_templates(catalog_path)?;
    for catalog in read_catalog_refs(catalog_path)? {
        if catalog.import_items {
            out.extend(read_catalog_templates_recursive(&catalog.catalog, seen)?);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn catalog_seen_key(catalog_path: &Path) -> PathBuf {
    let text = catalog_path.to_string_lossy();
    if is_remote_url(&text) {
        return catalog_path.to_path_buf();
    }
    fs::canonicalize(catalog_path).unwrap_or_else(|_| catalog_path.to_path_buf())
}

fn read_catalog_templates(catalog_path: &Path) -> Result<Vec<CatalogTemplate>> {
    let json = read_catalog_value(catalog_path)?;
    let Some(templates) = json.get("templates").and_then(|value| value.as_object()) else {
        return Ok(Vec::new());
    };
    let catalog_dir = catalog_path.parent().unwrap_or_else(|| Path::new("."));
    let base_ref = json
        .get("base-ref")
        .or_else(|| json.get("baseRef"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let mut out = Vec::new();
    for (name, value) in templates {
        let file_refs = value
            .get("file-refs")
            .or_else(|| value.get("fileRefs"))
            .and_then(|value| value.as_object())
            .map(|refs| {
                refs.iter()
                    .filter_map(|(target, source)| {
                        source
                            .as_str()
                            .map(|source| (target.clone(), source.to_string()))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let properties = value
            .get("properties")
            .and_then(|value| value.as_object())
            .map(|properties| {
                properties
                    .iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            value
                                .get("default")
                                .and_then(|value| value.as_str())
                                .map(str::to_string),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        out.push(CatalogTemplate {
            name: name.to_string(),
            description: value
                .get("description")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            file_refs,
            properties,
            catalog_dir: catalog_dir.to_path_buf(),
            base_ref: base_ref.clone(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn cache_entries(cache_dir: Option<&Path>) -> Result<Vec<CacheEntry>> {
    let root = match cache_dir {
        Some(path) => path.to_path_buf(),
        None => default_cache_dir()?,
    };
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let cache_dir = entry.path();
        let metadata = cache_dir.join("cache-entry.tsv");
        if !metadata.exists() {
            continue;
        }
        let text = fs::read_to_string(&metadata)?;
        let mut parts = text.trim_end().split('\t');
        let Some(script) = parts.next().filter(|s| !s.is_empty()) else {
            continue;
        };
        let Some(classes_dir) = parts.next().filter(|s| !s.is_empty()) else {
            continue;
        };
        entries.push(CacheEntry {
            script: PathBuf::from(script),
            classes_dir: PathBuf::from(classes_dir),
            cache_dir,
        });
    }
    entries.sort_by(|a, b| a.script.cmp(&b.script));
    Ok(entries)
}

fn render_init_script(template: InitTemplate, base_name: &str, options: &InitOptions) -> String {
    match template.name {
        "compact" => render_compact_init_script(options),
        "cli" => render_cli_init_script(base_name, options),
        "agent" => render_agent_init_script(base_name, options),
        "test" => render_test_init_script(base_name, options),
        "hello" | "java" => render_hello_init_script(base_name, options),
        _ => unreachable!("template was validated"),
    }
}

fn render_catalog_template(
    template: &CatalogTemplate,
    base_name: &str,
    options: &InitOptions,
) -> Result<String> {
    let Some((_, source_ref)) = template.file_refs.first() else {
        return Err(anyhow!(
            "template '{}' does not define file-refs",
            template.name
        ));
    };
    let source_path = resolve_catalog_script_ref(
        &template.catalog_dir,
        template.base_ref.as_deref(),
        source_ref,
    );
    let mut content = if is_remote_url(&source_path.to_string_lossy()) {
        fetch_remote_script(&source_path.to_string_lossy())?
    } else {
        fs::read_to_string(&source_path)
            .with_context(|| format!("failed to read template {}", source_path.display()))?
    };
    let mut replacements = vec![
        ("basename".to_string(), base_name.to_string()),
        ("baseName".to_string(), base_name.to_string()),
        ("className".to_string(), base_name.to_string()),
        ("name".to_string(), base_name.to_string()),
    ];
    for (key, value) in &template.properties {
        let Some(value) = value else {
            return Err(anyhow!(
                "template property '{}' has no default value; property overrides are not supported yet",
                key
            ));
        };
        replacements.push((key.clone(), value.clone()));
    }
    replacements.push((
        "javaVersion".to_string(),
        options
            .java_version
            .clone()
            .unwrap_or_else(|| "25".to_string()),
    ));
    for (key, value) in replacements {
        content = content.replace(&format!("{{{{{key}}}}}"), &value);
        content = content.replace(&format!("{{{key}}}"), &value);
    }
    if !options.deps.is_empty() {
        content = insert_template_deps(&content, &options.deps);
    }
    Ok(content)
}

fn insert_template_deps(content: &str, deps: &[String]) -> String {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let mut insert_at = 0;
    if lines
        .first()
        .is_some_and(|line| line.starts_with("///usr/bin/env ") || line.starts_with("#!"))
    {
        insert_at = 1;
    }
    while lines
        .get(insert_at)
        .is_some_and(|line| line.starts_with("//JAVA "))
    {
        insert_at += 1;
    }

    let mut out = String::new();
    for line in &lines[..insert_at] {
        out.push_str(line);
    }
    for dep in deps {
        out.push_str(&format!("//DEPS {dep}\n"));
    }
    for line in &lines[insert_at..] {
        out.push_str(line);
    }
    if lines.is_empty() {
        out.push_str(content);
    }
    out
}

fn render_header(options: &InitOptions, default_java: Option<&str>, out: &mut String) {
    out.push_str("///usr/bin/env jbx \"$0\" \"$@\" ; exit $?\n");
    if let Some(version) = options.java_version.as_deref().or(default_java) {
        out.push_str(&format!("//JAVA {version}\n"));
    }
    for dep in &options.deps {
        out.push_str(&format!("//DEPS {dep}\n"));
    }
}

fn render_dependency_hint(options: &InitOptions, out: &mut String) {
    if options.deps.is_empty() {
        out.push_str("// //DEPS <dependency1> <dependency2>\n");
    }
}

fn render_hello_init_script(_base_name: &str, options: &InitOptions) -> String {
    let mut out = String::new();
    render_header(options, Some("25+"), &mut out);
    render_dependency_hint(options, &mut out);
    out.push_str("void main(String... args) {\n    IO.println(\"Hello World\");\n}\n");
    out
}

fn render_compact_init_script(options: &InitOptions) -> String {
    let mut out = String::new();
    render_header(options, Some("25+"), &mut out);
    render_dependency_hint(options, &mut out);
    out.push_str("void main(String... args) {\n    IO.println(\"Hello World\");\n}\n");
    out
}

fn render_cli_init_script(base_name: &str, options: &InitOptions) -> String {
    let mut out = String::new();
    render_header(options, None, &mut out);
    out.push_str("//DEPS info.picocli:picocli:4.7.6\n");
    render_dependency_hint(options, &mut out);
    out.push_str(&format!(
        r#"
import picocli.CommandLine;
import picocli.CommandLine.Command;
import picocli.CommandLine.Parameters;

import java.util.concurrent.Callable;

@Command(name = "{base_name}", mixinStandardHelpOptions = true, version = "{base_name} 0.1",
        description = "{base_name} made with jbx")
class {base_name} implements Callable<Integer> {{

    @Parameters(index = "0", description = "The greeting to print", defaultValue = "World!")
    private String greeting;

    public static void main(String... args) {{
        int exitCode = new CommandLine(new {base_name}()).execute(args);
        System.exit(exitCode);
    }}

    @Override
    public Integer call() {{
        System.out.println("Hello " + greeting);
        return 0;
    }}
}}
"#
    ));
    out
}

fn render_test_init_script(base_name: &str, options: &InitOptions) -> String {
    let mut out = String::new();
    render_header(options, None, &mut out);
    out.push_str("//DEPS org.junit.jupiter:junit-jupiter-api:5.11.4\n");
    render_dependency_hint(options, &mut out);
    out.push_str(&format!(
        r#"
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class {base_name} {{

    @Test
    void greets() {{
        assertEquals("Hello World", greeting());
    }}

    private String greeting() {{
        return "Hello World";
    }}
}}
"#
    ));
    out
}

fn render_agent_init_script(base_name: &str, options: &InitOptions) -> String {
    let mut out = String::new();
    render_header(options, None, &mut out);
    out.push_str(&format!(
        "//JAVAAGENT\n//MANIFEST Premain-Class={base_name}\n//MANIFEST Can-Redefine-Classes=true\n//MANIFEST Can-Retransform-Classes=true\n//MANIFEST Can-Set-Native-Method-Prefix=true\n"
    ));
    render_dependency_hint(options, &mut out);
    out.push('\n');
    out.push_str(&format!(
        r#"import java.lang.instrument.ClassFileTransformer;
import java.lang.instrument.IllegalClassFormatException;
import java.lang.instrument.Instrumentation;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.ProtectionDomain;

public class {base_name} {{

    public static void premain(String agentArgs, Instrumentation instrumentation) {{
        System.out.println("jbx agent {base_name} loaded. Will dump all loaded classes into `classes/`");
        instrumentation.addTransformer(new ClassLogger());
    }}

    public static void main(String[] args) {{
        System.out.println("This is a jbx javaagent.\n" +
                           "Usage: \n" +
                           "   jbx run --javaagent={base_name}.java yourApp.java");
    }}

    public static class ClassLogger implements ClassFileTransformer {{
        @Override
        public byte[] transform(ClassLoader loader,
                                String className,
                                Class<?> classBeingRedefined,
                                ProtectionDomain protectionDomain,
                                byte[] classfileBuffer) throws IllegalClassFormatException {{
            try {{
                Path path = Paths.get("classes/" + className + ".class");
                Files.createDirectories(path.getParent());
                Files.write(path, classfileBuffer);
            }} catch (Throwable ignored) {{
                System.err.println(ignored);
            }}
            return classfileBuffer;
        }}
    }}
}}
"#
    ));
    out
}

fn java_class_name_from_stem(stem: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            if ch == '_' || ch == '-' {
                uppercase_next = true;
                continue;
            }
            if uppercase_next {
                out.extend(ch.to_uppercase());
                uppercase_next = false;
            } else {
                out.push(ch);
            }
        } else {
            uppercase_next = true;
        }
    }
    if out.is_empty() || out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert_str(0, "Test");
    }
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
        let stripped = stripped.trim_start();
        if stripped.starts_with("//") {
            continue;
        }
        let Some(caps) = directive_re.captures(stripped) else {
            continue;
        };
        let key = caps.name("key").map(|m| m.as_str()).unwrap_or_default();
        let value = caps
            .name("value")
            .map(|m| m.as_str().trim())
            .unwrap_or_default();

        match key {
            "DEPS" => directives.deps.extend(split_directive_words(value)),
            "RUNTIME" => directives.runtime_deps.extend(split_directive_words(value)),
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
    let mut quote: Option<char> = None;
    for ch in text.chars() {
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

pub fn trust_entries(cache_dir: Option<&Path>) -> Result<Vec<(String, String)>> {
    let path = trust_store_path(cache_dir)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for line in fs::read_to_string(&path)?.lines() {
        let Some((url, hash)) = line.split_once('\t') else {
            continue;
        };
        if !url.is_empty() && !hash.is_empty() {
            entries.push((url.to_string(), hash.to_string()));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

pub fn trust_add(url: &str, cache_dir: Option<&Path>) -> Result<String> {
    ensure_remote_url(url)?;
    let source = fetch_remote_script(url)?;
    let directives = parse_directives(&source);
    let resources = collect_remote_relative_resources(&directives, url)?;
    let hash = trusted_remote_hash(&source, &resources);
    write_trust_entry(url, &hash, cache_dir)?;
    Ok(hash)
}

pub fn trust_remove(url: &str, cache_dir: Option<&Path>) -> Result<bool> {
    let path = trust_store_path(cache_dir)?;
    let mut entries = trust_entries(cache_dir)?;
    let before = entries.len();
    entries.retain(|(entry_url, _)| entry_url != url);
    write_trust_entries(&path, &entries)?;
    Ok(entries.len() != before)
}

pub fn trust_clear(cache_dir: Option<&Path>) -> Result<()> {
    let path = trust_store_path(cache_dir)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

struct MaterializedScript {
    path: PathBuf,
    source: String,
}

fn materialize_script(
    script: &Path,
    cache_dir: Option<&Path>,
    trust_remote: bool,
) -> Result<MaterializedScript> {
    let script_text = script.to_string_lossy();
    if is_remote_url(&script_text) {
        let source = fetch_remote_script(&script_text)?;
        let directives = parse_directives(&source);
        let resources = collect_remote_relative_resources(&directives, &script_text)?;
        let hash = trusted_remote_hash(&source, &resources);
        if trust_remote {
            write_trust_entry(&script_text, &hash, cache_dir)?;
        } else if !is_trusted_remote(&script_text, &hash, cache_dir)? {
            return Err(anyhow!(
                "remote script {} is not trusted; run `jbx trust add {}` or pass `jbx run --trust {}`",
                script_text,
                script_text,
                script_text
            ));
        }
        let root = match cache_dir {
            Some(path) => path.to_path_buf(),
            None => default_cache_dir()?,
        };
        let file_name = remote_file_name(&script_text);
        let remote_dir = root.join("remote-sources").join(&hash[..16]);
        fs::create_dir_all(&remote_dir)?;
        let path = remote_dir.join(file_name);
        fs::write(&path, &source)?;
        materialize_remote_relative_resources(&resources, &remote_dir)?;
        return Ok(MaterializedScript { path, source });
    }

    let path = fs::canonicalize(script)
        .with_context(|| format!("script not found: {}", script.display()))?;
    let source =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(MaterializedScript { path, source })
}

fn is_remote_url(text: &str) -> bool {
    text.starts_with("http://") || text.starts_with("https://")
}

fn ensure_remote_url(url: &str) -> Result<()> {
    if is_remote_url(url) {
        Ok(())
    } else {
        Err(anyhow!(
            "trusted script must be an http:// or https:// URL: {url}"
        ))
    }
}

fn fetch_remote_script(url: &str) -> Result<String> {
    ensure_remote_url(url)?;
    let response = ureq::get(url)
        .call()
        .map_err(|err| anyhow!("failed to download {url}: {err}"))?;
    response
        .into_string()
        .map_err(|err| anyhow!("failed to read response from {url}: {err}"))
}

#[derive(Debug)]
struct RemoteRelativeResource {
    resource_ref: String,
    url: String,
    body: String,
}

fn collect_remote_relative_resources(
    directives: &Directives,
    remote_url: &str,
) -> Result<Vec<RemoteRelativeResource>> {
    let mut resources = Vec::new();
    let source_refs = directives.sources.iter().chain(
        directives
            .deps
            .iter()
            .filter(|dep| !looks_like_binary_dependency(dep)),
    );
    for source_ref in source_refs {
        collect_remote_relative_ref(remote_url, source_ref, &mut resources)?;
    }

    for file_ref in &directives.files {
        let (_, source) = split_file_ref(file_ref);
        let source_ref = source
            .to_str()
            .ok_or_else(|| anyhow!("invalid non-UTF-8 //FILES resource: {file_ref}"))?;
        collect_remote_relative_ref(remote_url, source_ref, &mut resources)?;
    }

    resources.sort_by(|a, b| a.resource_ref.cmp(&b.resource_ref).then(a.url.cmp(&b.url)));
    Ok(resources)
}

fn collect_remote_relative_ref(
    remote_url: &str,
    resource_ref: &str,
    resources: &mut Vec<RemoteRelativeResource>,
) -> Result<()> {
    if resource_ref.is_empty() || looks_like_binary_dependency(resource_ref) {
        return Ok(());
    }
    validate_remote_relative_ref(resource_ref)?;
    let url = resolve_remote_resource_url(remote_url, resource_ref)?;
    let body = fetch_remote_script(&url)?;
    resources.push(RemoteRelativeResource {
        resource_ref: resource_ref.to_string(),
        url,
        body,
    });
    Ok(())
}

fn materialize_remote_relative_resources(
    resources: &[RemoteRelativeResource],
    remote_dir: &Path,
) -> Result<()> {
    for resource in resources {
        let local_path = remote_dir.join(&resource.resource_ref);
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local_path, &resource.body).with_context(|| {
            format!(
                "failed to cache remote resource {} at {}",
                resource.url,
                local_path.display()
            )
        })?;
    }
    Ok(())
}

fn validate_remote_relative_ref(resource_ref: &str) -> Result<()> {
    if Path::new(resource_ref).is_absolute()
        || resource_ref.starts_with('/')
        || resource_ref.contains('\\')
    {
        return Err(anyhow!(
            "only relative URL paths are supported for remote resources: {resource_ref}"
        ));
    }
    if resource_ref
        .split('/')
        .any(|part| part == ".." || part == "." || part.is_empty())
    {
        return Err(anyhow!(
            "remote resource paths must not contain empty or parent segments: {resource_ref}"
        ));
    }
    Ok(())
}

fn trusted_remote_hash(source: &str, resources: &[RemoteRelativeResource]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"jbx-remote-v1\nmain\0");
    hasher.update(source.as_bytes());
    for resource in resources {
        hasher.update(b"\nresource\0");
        hasher.update(resource.resource_ref.as_bytes());
        hasher.update(b"\0");
        hasher.update(resource.url.as_bytes());
        hasher.update(b"\0");
        hasher.update(resource.body.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn resolve_remote_resource_url(remote_url: &str, resource_ref: &str) -> Result<String> {
    let no_query = remote_url
        .split(['?', '#'])
        .next()
        .ok_or_else(|| anyhow!("invalid remote URL: {remote_url}"))?;
    let scheme_end = no_query
        .find("://")
        .ok_or_else(|| anyhow!("invalid remote URL: {remote_url}"))?;
    let after_scheme = scheme_end + 3;
    let host_end = no_query[after_scheme..]
        .find('/')
        .map(|idx| after_scheme + idx)
        .unwrap_or(no_query.len());
    let origin = &no_query[..host_end];

    if resource_ref.starts_with('/') {
        return Ok(format!("{origin}{resource_ref}"));
    }

    let base_dir = match no_query.rfind('/') {
        Some(idx) if idx >= host_end => &no_query[..idx + 1],
        _ => &format!("{origin}/"),
    };
    Ok(format!("{base_dir}{resource_ref}"))
}

fn is_trusted_remote(url: &str, hash: &str, cache_dir: Option<&Path>) -> Result<bool> {
    let entries = trust_entries(cache_dir)?;
    Ok(entries
        .iter()
        .any(|(entry_url, entry_hash)| entry_url == url && entry_hash == hash))
}

fn write_trust_entry(url: &str, hash: &str, cache_dir: Option<&Path>) -> Result<()> {
    let path = trust_store_path(cache_dir)?;
    let mut entries = trust_entries(cache_dir)?;
    entries.retain(|(entry_url, _)| entry_url != url);
    entries.push((url.to_string(), hash.to_string()));
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    write_trust_entries(&path, &entries)
}

fn write_trust_entries(path: &Path, entries: &[(String, String)]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    for (url, hash) in entries {
        out.push_str(url);
        out.push('\t');
        out.push_str(hash);
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

fn trust_store_path(cache_dir: Option<&Path>) -> Result<PathBuf> {
    let root = match cache_dir {
        Some(path) => path.to_path_buf(),
        None => default_cache_dir()?,
    };
    Ok(root.join("trust.tsv"))
}

fn remote_file_name(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let name = path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("RemoteScript.java");
    if name.ends_with(".java") {
        name.to_string()
    } else {
        format!("{name}.java")
    }
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn dedupe_key_values(values: &mut Vec<KeyValue>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert((value.key.clone(), value.value.clone())));
}

fn collect_declared_source_directives(
    base_dir: &Path,
    source_refs: impl IntoIterator<Item = String>,
) -> Result<(Vec<PathBuf>, Directives)> {
    let mut directives = Directives::default();
    let mut sources = Vec::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    for source_ref in source_refs {
        queue.push_back(base_dir.join(source_ref));
    }
    while let Some(source_path) = queue.pop_front() {
        let key = source_path
            .canonicalize()
            .unwrap_or_else(|_| source_path.clone());
        if !visited.insert(key) {
            continue;
        }
        let source = fs::read_to_string(&source_path)
            .with_context(|| format!("failed to read source {}", source_path.display()))?;
        let parsed = parse_directives(&source);
        let source_dir = source_path.parent().unwrap_or(base_dir);
        for nested in parsed.sources.iter().cloned().chain(
            parsed
                .deps
                .iter()
                .filter(|dep| !looks_like_binary_dependency(dep))
                .cloned(),
        ) {
            queue.push_back(source_dir.join(nested));
        }
        directives.deps.extend(parsed.deps);
        directives.repos.extend(parsed.repos);
        directives.javac_options.extend(parsed.javac_options);
        directives.runtime_options.extend(parsed.runtime_options);
        directives.native_options.extend(parsed.native_options);
        directives.java_agents.extend(parsed.java_agents);
        directives.files.extend(parsed.files);
        directives.sources.extend(parsed.sources);
        sources.push(source_path);
    }
    sources.sort();
    sources.dedup();
    dedupe_strings(&mut directives.deps);
    dedupe_strings(&mut directives.repos);
    dedupe_strings(&mut directives.javac_options);
    dedupe_strings(&mut directives.runtime_options);
    dedupe_strings(&mut directives.native_options);
    dedupe_key_values(&mut directives.java_agents);
    dedupe_strings(&mut directives.files);
    dedupe_strings(&mut directives.sources);
    Ok((sources, directives))
}

pub fn build_java(options: BuildOptions) -> Result<BuildOutput> {
    let materialized = materialize_script(
        &options.script,
        options.cache_dir.as_deref(),
        options.trust_remote,
    )?;
    let script = materialized.path;
    let source = materialized.source;
    let mut directives = parse_directives(&source);
    directives.deps.extend(options.extra_deps);
    directives.repos.extend(options.extra_repos);
    directives.sources.extend(options.extra_sources);
    directives.files.extend(options.extra_files);
    directives.javac_options.extend(options.javac_options);
    directives.runtime_options.extend(options.runtime_options);
    directives.java_agents.extend(options.java_agents);
    if options.java_version.is_some() {
        directives.java_version = options.java_version;
    }
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
    let seed_source_refs = directives.sources.iter().cloned().chain(
        directives
            .deps
            .iter()
            .filter(|dep| !looks_like_binary_dependency(dep))
            .cloned(),
    );
    let (declared_sources, companion_directives) =
        collect_declared_source_directives(base_dir, seed_source_refs)?;
    directives.deps.extend(companion_directives.deps);
    directives.repos.extend(companion_directives.repos);
    directives
        .javac_options
        .extend(companion_directives.javac_options);
    directives
        .runtime_options
        .extend(companion_directives.runtime_options);
    directives
        .native_options
        .extend(companion_directives.native_options);
    directives
        .java_agents
        .extend(companion_directives.java_agents);
    directives.files.extend(companion_directives.files);
    directives.sources.extend(companion_directives.sources);
    dedupe_strings(&mut directives.deps);
    dedupe_strings(&mut directives.repos);
    dedupe_strings(&mut directives.javac_options);
    dedupe_strings(&mut directives.runtime_options);
    dedupe_strings(&mut directives.native_options);
    dedupe_key_values(&mut directives.java_agents);
    dedupe_strings(&mut directives.files);
    dedupe_strings(&mut directives.sources);

    let (binary_deps, _source_deps): (Vec<_>, Vec<_>) = directives
        .deps
        .iter()
        .cloned()
        .partition(|dep| looks_like_binary_dependency(dep));
    let mut sources = vec![script.clone()];
    sources.extend(declared_sources);

    let dep_cp = resolve_dependencies(&binary_deps, &directives.repos, &work_dir)?;
    let mut cp_entries = options.classpath;
    cp_entries.extend(dep_cp);

    let jdk_root = jdk::resolve_jdk(&directives.java_version, true)?;
    let javac = jdk::javac_bin_path(&jdk_root).display().to_string();
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
            let release_version = jdk::detect_jdk_major_version(&jdk_root).with_context(|| {
                format!("could not determine JDK version at {}", jdk_root.display())
            })?;
            javac_cmd.arg("--release").arg(release_version.to_string());
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
    write_cache_entry(&work_dir, &script, &classes_dir)?;

    let main_class = directives
        .main_class
        .clone()
        .or_else(|| infer_main_class_from_source(&script, &source));

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
        extra_repos: options.extra_repos,
        extra_sources: options.extra_sources,
        extra_files: options.extra_files,
        classpath: options.classpath,
        javac_options: options.javac_options,
        runtime_options: Vec::new(),
        java_agents: options.java_agents,
        java_version: options.java_version,
        main_class: options.main_class,
        cache_dir: options.cache_dir,
        trust_remote: options.trust_remote,
    })?;

    let main_class = build.main_class.ok_or_else(|| {
        anyhow!("could not infer main class; add //MAIN fully.qualified.ClassName")
    })?;

    let jdk_root = jdk::resolve_jdk(&build.directives.java_version, true)?;
    let java = jdk::java_bin_path(&jdk_root).display().to_string();
    let mut runtime_cp = vec![build.classes_dir];
    runtime_cp.extend(build.classpath);
    let mut java_cmd = Command::new(&java);
    for agent in &build.directives.java_agents {
        java_cmd.arg(format_java_agent(agent));
    }
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

fn format_java_agent(agent: &KeyValue) -> String {
    match &agent.value {
        Some(value) => format!("-javaagent:{}={}", agent.key, value),
        None => format!("-javaagent:{}", agent.key),
    }
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
    dep.matches(':').count() >= 1 && !dep.ends_with(".java")
}

fn resolve_dependencies(
    deps: &[String],
    repos: &[String],
    _work_dir: &Path,
) -> Result<Vec<PathBuf>> {
    if deps.is_empty() {
        return Ok(Vec::new());
    }

    let cache_dir = default_cache_dir()?.join("deps");
    let mut maven_repos = vec![resolver::Repository::central()];
    for repo in repos {
        if let Some((id, url)) = repo.split_once('=') {
            maven_repos.push(resolver::Repository {
                id: id.to_string(),
                url: url.to_string(),
            });
        } else if repo.starts_with("http") {
            maven_repos.push(resolver::Repository {
                id: repo.clone(),
                url: repo.clone(),
            });
        } else if repo == "mavenCentral" || repo == "central" {
            // already included
        } else {
            maven_repos.push(resolver::Repository {
                id: repo.clone(),
                url: repo.clone(),
            });
        }
    }

    let paths = resolver::resolve_classpath(deps, &maven_repos, &cache_dir)?;
    Ok(paths)
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

fn write_cache_entry(work_dir: &Path, script: &Path, classes_dir: &Path) -> Result<()> {
    fs::write(
        work_dir.join("cache-entry.tsv"),
        format!("{}\t{}\n", script.display(), classes_dir.display()),
    )?;
    Ok(())
}

pub fn infer_main_class_from_source(script: &Path, source: &str) -> Option<String> {
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

// ── Export local / portable JARs ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Local,
    Portable,
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub script: PathBuf,
    pub output: Option<PathBuf>,
    pub force: bool,
    pub kind: ExportKind,
    pub extra_deps: Vec<String>,
    pub extra_repos: Vec<String>,
    pub extra_sources: Vec<String>,
    pub extra_files: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub trust_remote: bool,
}

#[derive(Debug, Clone)]
pub struct NativeExportOptions {
    pub script: PathBuf,
    pub output: Option<PathBuf>,
    pub force: bool,
    pub native_image: Option<PathBuf>,
    pub extra_native_options: Vec<String>,
    pub extra_deps: Vec<String>,
    pub extra_repos: Vec<String>,
    pub extra_sources: Vec<String>,
    pub extra_files: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub javac_options: Vec<String>,
    pub runtime_options: Vec<String>,
    pub java_agents: Vec<KeyValue>,
    pub java_version: Option<String>,
    pub main_class: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub trust_remote: bool,
}

pub fn export_jar(options: ExportOptions) -> Result<PathBuf> {
    let output_path = export_output_path(&options.script, options.output)?;
    if output_path.exists() && !options.force {
        return Err(anyhow!(
            "export target {} already exists; use --force to overwrite",
            output_path.display()
        ));
    }
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let build = build_java(BuildOptions {
        script: options.script,
        extra_deps: options.extra_deps,
        extra_repos: options.extra_repos,
        extra_sources: options.extra_sources,
        extra_files: options.extra_files,
        classpath: options.classpath,
        javac_options: options.javac_options,
        runtime_options: options.runtime_options,
        java_agents: options.java_agents,
        java_version: options.java_version,
        main_class: options.main_class,
        cache_dir: options.cache_dir,
        trust_remote: options.trust_remote,
    })?;
    let main_class = build.main_class.ok_or_else(|| {
        anyhow!("could not infer main class; add //MAIN fully.qualified.ClassName")
    })?;

    match options.kind {
        ExportKind::Local => write_classes_jar(
            &build.classes_dir,
            &output_path,
            &main_class,
            &manifest_classpath(&build.classpath)?,
        )?,
        ExportKind::Portable => {
            let manifest_cp =
                copy_portable_classpath(&output_path, &build.classpath, options.force)?;
            write_classes_jar(&build.classes_dir, &output_path, &main_class, &manifest_cp)?;
        }
    }
    Ok(output_path)
}

pub fn export_native(options: NativeExportOptions) -> Result<PathBuf> {
    let output_path = native_export_output_path(&options.script, options.output)?;
    if output_path.exists() && !options.force {
        return Err(anyhow!(
            "native export target {} already exists; use --force to overwrite",
            output_path.display()
        ));
    }
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let build = build_java(BuildOptions {
        script: options.script,
        extra_deps: options.extra_deps,
        extra_repos: options.extra_repos,
        extra_sources: options.extra_sources,
        extra_files: options.extra_files,
        classpath: options.classpath,
        javac_options: options.javac_options,
        runtime_options: options.runtime_options,
        java_agents: options.java_agents,
        java_version: options.java_version,
        main_class: options.main_class,
        cache_dir: options.cache_dir,
        trust_remote: options.trust_remote,
    })?;
    let main_class = build.main_class.ok_or_else(|| {
        anyhow!("could not infer main class; add //MAIN fully.qualified.ClassName")
    })?;

    let native_image = match options.native_image {
        Some(path) => path,
        None => find_native_image(&build.directives.java_version)?,
    };
    let mut native_cmd = Command::new(&native_image);
    native_cmd.args(&build.directives.native_options);
    native_cmd.args(&options.extra_native_options);
    let mut runtime_cp = vec![build.classes_dir];
    runtime_cp.extend(build.classpath);
    native_cmd.arg("-cp").arg(join_classpath(&runtime_cp));
    let raw_name = native_output_name(&output_path)?;
    let image_name = if cfg!(windows) {
        raw_name
            .strip_suffix(".exe")
            .unwrap_or(&raw_name)
            .to_string()
    } else {
        raw_name
    };
    native_cmd.arg(format!("-H:Name={image_name}"));
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        native_cmd.arg(format!("-H:Path={}", parent.display()));
    }
    native_cmd.arg(main_class);
    let status = native_cmd
        .status()
        .with_context(|| format!("failed to execute {}", native_image.display()))?;
    if !status.success() {
        return Err(anyhow!(
            "native-image failed with exit code {}",
            status.code().unwrap_or(1)
        ));
    }
    Ok(output_path)
}

fn native_export_output_path(script: &Path, output: Option<PathBuf>) -> Result<PathBuf> {
    let mut path = match output {
        Some(path) => path,
        None => {
            let stem = script.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                anyhow!(
                    "could not infer native export filename from {}",
                    script.display()
                )
            })?;
            PathBuf::from(stem)
        }
    };
    if cfg!(windows) && path.extension().is_none() {
        path.set_extension("exe");
    }
    Ok(path)
}

fn native_output_name(output_path: &Path) -> Result<String> {
    output_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow!("invalid native export path: {}", output_path.display()))
}

fn find_native_image(java_version: &Option<String>) -> Result<PathBuf> {
    let jdk_root = jdk::resolve_jdk(java_version, true)?;
    let candidate = jdk_root.join("bin").join(if cfg!(windows) {
        "native-image.cmd"
    } else {
        "native-image"
    });
    if candidate.is_file() {
        return Ok(candidate);
    }
    which::which("native-image").with_context(|| {
        format!(
            "could not find native-image at {} or on PATH; install GraalVM native-image or pass --native-image",
            candidate.display()
        )
    })
}

fn export_output_path(script: &Path, output: Option<PathBuf>) -> Result<PathBuf> {
    let mut path = match output {
        Some(path) => path,
        None => {
            let stem = script.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                anyhow!("could not infer export filename from {}", script.display())
            })?;
            PathBuf::from(stem)
        }
    };
    if path.extension().and_then(|ext| ext.to_str()) != Some("jar") {
        path.set_extension("jar");
    }
    Ok(path)
}

fn manifest_classpath(paths: &[PathBuf]) -> Result<String> {
    paths
        .iter()
        .map(|path| manifest_file_url(path))
        .collect::<Result<Vec<_>>>()
        .map(|entries| entries.join(" "))
}

fn manifest_file_url(path: &Path) -> Result<String> {
    let absolute = fs::canonicalize(path)
        .with_context(|| format!("failed to resolve classpath entry {}", path.display()))?;
    let mut text = absolute.to_string_lossy().replace('\\', "/");
    if cfg!(windows) && text.len() >= 2 && text.as_bytes()[1] == b':' {
        text.insert(0, '/');
    }
    Ok(format!("file://{}", percent_encode_manifest_path(&text)))
}

fn percent_encode_manifest_path(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn percent_encode_manifest_segment(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn copy_portable_classpath(
    output_path: &Path,
    classpath: &[PathBuf],
    force: bool,
) -> Result<String> {
    if classpath.is_empty() {
        return Ok(String::new());
    }
    let lib_dir = output_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("lib");
    fs::create_dir_all(&lib_dir)?;
    let mut seen_names = HashSet::new();
    let mut manifest_entries = Vec::new();
    for entry in classpath {
        if !entry.is_file() {
            return Err(anyhow!(
                "portable export only supports file classpath entries for now: {}",
                entry.display()
            ));
        }
        let file_name = entry
            .file_name()
            .ok_or_else(|| anyhow!("invalid classpath entry: {}", entry.display()))?;
        let manifest_name = percent_encode_manifest_segment(&file_name.to_string_lossy());
        if !seen_names.insert(manifest_name.clone()) {
            return Err(anyhow!(
                "portable export has duplicate dependency filename {}; use unique filenames before exporting",
                file_name.to_string_lossy()
            ));
        }
        let target = lib_dir.join(file_name);
        if target.exists() && !force {
            return Err(anyhow!(
                "portable dependency {} already exists; use --force to overwrite",
                target.display()
            ));
        }
        fs::copy(entry, &target).with_context(|| {
            format!(
                "failed to copy portable dependency {} to {}",
                entry.display(),
                target.display()
            )
        })?;
        manifest_entries.push(format!("lib/{manifest_name}"));
    }
    Ok(manifest_entries.join(" "))
}

fn write_classes_jar(
    classes_dir: &Path,
    output_path: &Path,
    main_class: &str,
    manifest_classpath: &str,
) -> Result<()> {
    let file = File::create(output_path)
        .with_context(|| format!("failed to create {}", output_path.display()))?;
    let mut jar = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    jar.start_file("META-INF/MANIFEST.MF", options)?;
    jar.write_all(render_manifest(main_class, manifest_classpath).as_bytes())?;

    for entry in walkdir::WalkDir::new(classes_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(classes_dir)?;
        let jar_path = rel.to_string_lossy().replace('\\', "/");
        jar.start_file(jar_path, options)?;
        let mut input = File::open(entry.path())?;
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer)?;
        jar.write_all(&buffer)?;
    }
    jar.finish()?;
    Ok(())
}

fn render_manifest(main_class: &str, classpath: &str) -> String {
    let mut manifest = String::new();
    manifest.push_str(&fold_manifest_line("Manifest-Version: 1.0"));
    manifest.push_str(&fold_manifest_line(&format!("Main-Class: {main_class}")));
    if !classpath.is_empty() {
        manifest.push_str(&fold_manifest_line(&format!("Class-Path: {classpath}")));
    }
    manifest.push('\n');
    manifest
}

fn fold_manifest_line(line: &str) -> String {
    const MAX_BYTES: usize = 72;
    let mut out = String::new();
    let mut current = String::new();
    let mut first = true;
    for ch in line.chars() {
        let limit = if first { MAX_BYTES } else { MAX_BYTES - 1 };
        if !current.is_empty() && current.len() + ch.len_utf8() > limit {
            if !first {
                out.push(' ');
            }
            out.push_str(&current);
            out.push('\n');
            current.clear();
            first = false;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        if !first {
            out.push(' ');
        }
        out.push_str(&current);
        out.push('\n');
    }
    out
}

// ── App install / uninstall / list ──────────────────────────────────────

pub fn app_bin_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| anyhow!("could not determine local data directory"))?
        .join("jbx")
        .join("bin"))
}

pub struct AppInstallOptions {
    pub script: PathBuf,
    pub name: Option<String>,
    pub force: bool,
}

pub fn app_install(options: AppInstallOptions) -> Result<PathBuf> {
    let bin_dir = app_bin_dir()?;
    fs::create_dir_all(&bin_dir)?;

    let script = fs::canonicalize(&options.script)
        .with_context(|| format!("script not found: {}", options.script.display()))?;

    let name = options
        .name
        .or_else(|| {
            script
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| anyhow!("could not determine command name from script path"))?;

    validate_app_name(&name)?;

    let wrapper = bin_dir.join(&name);
    if wrapper.exists() && !options.force {
        return Err(anyhow!(
            "command '{}' already exists; use --force to overwrite",
            name
        ));
    }

    let juv_path = find_jbx_binary()?;

    // Build the wrapper script content
    let content = format!(
        "#!/bin/sh\nexec {} run -- {} \"$@\"\n",
        shell_quote_path(&juv_path),
        shell_quote_path(&script)
    );
    fs::write(&wrapper, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))?;
    }

    Ok(wrapper)
}

pub fn app_uninstall(name: &str) -> Result<bool> {
    validate_app_name(name)?;
    let bin_dir = app_bin_dir()?;
    let mut removed = false;
    // Remove all variants: name, name.cmd, name.ps1
    for ext in &["", ".cmd", ".ps1"] {
        let path = bin_dir.join(format!("{name}{ext}"));
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            removed = true;
        }
    }
    Ok(removed)
}

pub struct AppEntry {
    pub name: String,
    pub target: String,
}

pub fn app_list() -> Result<Vec<AppEntry>> {
    let bin_dir = app_bin_dir()?;
    if !bin_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&bin_dir)? {
        let entry = entry?;
        let path = entry.path();
        // Skip Windows helper scripts
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext == "cmd" || ext == "ps1" {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        // Parse the wrapper to extract the target script
        let target = parse_wrapper_target(&path).unwrap_or_else(|| "(unknown)".to_string());
        entries.push(AppEntry { name, target });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn validate_app_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("command name cannot be empty"));
    }
    if name == "jbx" {
        return Err(anyhow!("'jbx' is a reserved command name"));
    }
    // Must be a portable filename
    let valid =
        !name.chars().any(|c| c == '/' || c == '\\' || c == '\0') && name != "." && name != "..";
    if !valid {
        return Err(anyhow!("'{name}' is not a valid command name"));
    }
    Ok(())
}

fn find_jbx_binary() -> Result<PathBuf> {
    // Prefer the currently-running binary if possible
    if let Ok(exe) = std::env::current_exe() {
        if exe.exists() {
            return Ok(exe);
        }
    }
    which::which("jbx").context("could not locate jbx binary on PATH")
}

fn shell_quote_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    // Simple quoting: wrap in single quotes, escape any embedded single quotes
    if s.contains('\'') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else if s.contains(char::is_whitespace) || s.contains('$') {
        format!("'{s}'")
    } else {
        s.to_string()
    }
}

fn parse_wrapper_target(wrapper: &Path) -> Option<String> {
    let content = fs::read_to_string(wrapper).ok()?;
    // Wrapper line looks like: exec /path/to/jbx run -- /path/to/script.java "$@"
    // Or with quoting:          exec /path/to/jbx run -- '/path/with spaces/script.java' "$@"
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("exec ") {
            if let Some(idx) = rest.find(" run -- ") {
                let marker = " run -- ";
                let after = &rest[idx + marker.len()..];
                let target = after.strip_suffix(" \"$@\"").unwrap_or(after);
                let target = target.trim();
                // Strip single-quote wrapping added by shell_quote_path
                let target = target
                    .strip_prefix('\'')
                    .and_then(|t| t.strip_suffix('\''))
                    .unwrap_or(target);
                return Some(target.trim().to_string());
            }
        }
    }
    None
}
