use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
};

use juv::{
    app_bin_dir, app_install, app_list, app_uninstall, build_java, cache_entries, catalog_aliases,
    clear_cache, default_cache_dir, export_jar, init_script, resolve_catalog_alias, run_java,
    split_directive_words, trust_add, trust_clear, trust_entries, trust_remove, AppInstallOptions,
    BuildOptions, ExportKind, ExportOptions, InitOptions, KeyValue, RunOptions,
};

#[derive(Parser, Debug)]
#[command(name = "juv", version, about = "juv: a Rust port of JBang")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Script to run when no subcommand is given, JBang-style.
    script: Option<PathBuf>,

    /// Arguments passed to the script when no subcommand is given.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compile and run a Java source file.
    Run(RunCommand),
    /// Compile and store script in the cache without running it.
    Build(BuildCommand),
    /// Initialize a Java script.
    Init(InitCommand),
    /// Manage compiled script cache.
    Cache(CacheCommand),
    /// Manage trusted remote scripts.
    Trust(TrustCommand),
    /// Print parsed JBang directives.
    Info(InfoCommand),
    /// Manage scripts installed as commands on PATH.
    App(AppCommand),
    /// Manage aliases from jbang-catalog.json.
    Alias(AliasCommand),
    /// Export runnable JARs.
    Export(ExportCommand),
    /// Resolve Maven dependencies without running.
    Resolve(ResolveCommand),
    /// Fetch Maven dependency artifacts and print classpath.
    Fetch(FetchCommand),
    /// Manage installed JDKs.
    Jdk(JdkCommand),
}

#[derive(Parser, Debug)]
struct RunCommand {
    /// Additional dependency coordinates, same shape as //DEPS.
    #[arg(long = "deps")]
    deps: Vec<String>,
    /// Additional repository, same shape as //REPOS.
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Additional source file, same shape as //SOURCES.
    #[arg(long = "source", alias = "sources")]
    sources: Vec<String>,

    /// Additional file/resource, same shape as //FILES.
    #[arg(long = "files", alias = "file")]
    files: Vec<String>,

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(
        long = "javac-option",
        alias = "compile-option",
        allow_hyphen_values = true
    )]
    javac_options: Vec<String>,

    /// Additional java runtime option.
    #[arg(
        long = "runtime-option",
        alias = "java-option",
        allow_hyphen_values = true
    )]
    runtime_options: Vec<String>,

    /// Override //JAVA requested version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Additional java agent, same shape as //JAVAAGENT.
    #[arg(long = "javaagent")]
    java_agents: Vec<String>,

    /// Override //MAIN / inferred class name.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Trust this remote script content hash before running.
    #[arg(long = "trust")]
    trust: bool,

    /// Java source file.
    script: PathBuf,

    /// Arguments passed to the script.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Parser, Debug)]
struct BuildCommand {
    /// Additional dependency coordinates, same shape as //DEPS.
    #[arg(long = "deps")]
    deps: Vec<String>,
    /// Additional repository, same shape as //REPOS.
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Additional source file, same shape as //SOURCES.
    #[arg(long = "source", alias = "sources")]
    sources: Vec<String>,

    /// Additional file/resource, same shape as //FILES.
    #[arg(long = "files", alias = "file")]
    files: Vec<String>,

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(
        long = "javac-option",
        alias = "compile-option",
        allow_hyphen_values = true
    )]
    javac_options: Vec<String>,

    /// Additional java runtime option, same shape as //JAVA_OPTIONS.
    #[arg(
        long = "runtime-option",
        alias = "java-option",
        allow_hyphen_values = true
    )]
    runtime_options: Vec<String>,

    /// Override //JAVA requested version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Additional java agent, same shape as //JAVAAGENT.
    #[arg(long = "javaagent")]
    java_agents: Vec<String>,

    /// Override //MAIN / inferred class name.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Trust this remote script content hash before building.
    #[arg(long = "trust")]
    trust: bool,

    /// Java source file.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct InitCommand {
    /// Init script with the default Java template for now.
    #[arg(long = "template", short = 't')]
    template: Option<String>,

    /// Force overwrite of existing files.
    #[arg(long = "force")]
    force: bool,

    /// Java version directive to write.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Add dependencies, separated by comma, semicolon, or whitespace.
    #[arg(long = "deps")]
    deps: Vec<String>,

    /// Java source file to initialize.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct CacheCommand {
    #[command(subcommand)]
    command: CacheSubcommand,
}

#[derive(Subcommand, Debug)]
enum CacheSubcommand {
    /// Clear the juv cache directory.
    Clear(CacheClearCommand),
    /// Print the effective juv cache directory.
    Path(CachePathCommand),
    /// List cached script entries.
    List(CacheListCommand),
}

#[derive(Parser, Debug)]
struct TrustCommand {
    #[command(subcommand)]
    command: TrustSubcommand,
}

#[derive(Subcommand, Debug)]
enum TrustSubcommand {
    /// Trust the current content hash of a remote script URL.
    Add(TrustUrlCommand),
    /// Remove a trusted remote script URL.
    Remove(TrustUrlCommand),
    /// List trusted remote script URLs and hashes.
    List(TrustListCommand),
    /// Clear all trusted remote script entries.
    Clear(TrustListCommand),
}

#[derive(Parser, Debug)]
struct TrustUrlCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Remote http(s) Java source URL.
    url: String,
}

#[derive(Parser, Debug)]
struct TrustListCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct AppCommand {
    #[command(subcommand)]
    command: AppSubcommand,
}

#[derive(Subcommand, Debug)]
enum AppSubcommand {
    /// Install a script as a command on PATH.
    Install(AppInstallCommand),
    /// Remove an installed command.
    Uninstall(AppUninstallCommand),
    /// List installed script commands.
    List(AppListCommand),
}

#[derive(Parser, Debug)]
struct AppInstallCommand {
    /// Command name (defaults to the script filename stem).
    #[arg(long = "name", short = 'n')]
    name: Option<String>,

    /// Force overwrite an existing command.
    #[arg(long = "force")]
    force: bool,

    /// Java source file to install.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct AppUninstallCommand {
    /// Command name to remove.
    name: String,
}

#[derive(Parser, Debug)]
struct AppListCommand;

#[derive(Parser, Debug)]
struct JdkCommand {
    #[command(subcommand)]
    command: JdkSubcommand,
}

#[derive(Subcommand, Debug)]
enum JdkSubcommand {
    /// List discovered and installed JDKs.
    List(JdkListCommand),
    /// Install a JDK from Adoptium (Eclipse Temurin).
    Install(JdkInstallCommand),
    /// Show JDK home directory for a given version.
    Home(JdkHomeCommand),
}

#[derive(Parser, Debug)]
struct JdkListCommand;

#[derive(Parser, Debug)]
struct JdkInstallCommand {
    /// JDK version to install (e.g. 21, 25, 25+).
    version: String,
}

#[derive(Parser, Debug)]
struct JdkHomeCommand {
    /// JDK version (defaults to 25).
    #[arg(default_value = "25")]
    version: String,
}

#[derive(Parser, Debug)]
struct AliasCommand {
    #[command(subcommand)]
    command: AliasSubcommand,
}

#[derive(Subcommand, Debug)]
enum AliasSubcommand {
    /// List aliases from the nearest jbang-catalog.json.
    List(AliasListCommand),
}

#[derive(Parser, Debug)]
struct AliasListCommand {
    /// Print JSON instead of tab-separated text.
    #[arg(long = "json")]
    json: bool,
}

#[derive(Parser, Debug)]
struct ExportCommand {
    #[command(subcommand)]
    command: ExportSubcommand,
}

#[derive(Subcommand, Debug)]
enum ExportSubcommand {
    /// Export a runnable JAR with manifest classpath entries pointing at local paths.
    Local(ExportJarCommand),
    /// Export a runnable JAR plus lib/ dependencies for portable use.
    Portable(ExportJarCommand),
}

#[derive(Parser, Debug)]
struct ExportJarCommand {
    /// Output JAR path (defaults to <script>.jar).
    #[arg(long = "output", short = 'o')]
    output: Option<PathBuf>,

    /// Force overwrite of existing output files.
    #[arg(long = "force")]
    force: bool,

    /// Additional dependency coordinates, same shape as //DEPS.
    #[arg(long = "deps")]
    deps: Vec<String>,

    /// Additional repository, same shape as //REPOS.
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Additional source file, same shape as //SOURCES.
    #[arg(long = "source", alias = "sources")]
    sources: Vec<String>,

    /// Additional file/resource, same shape as //FILES.
    #[arg(long = "files", alias = "file")]
    files: Vec<String>,

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(
        long = "javac-option",
        alias = "compile-option",
        allow_hyphen_values = true
    )]
    javac_options: Vec<String>,

    /// Additional java runtime option, same shape as //JAVA_OPTIONS.
    #[arg(
        long = "runtime-option",
        alias = "java-option",
        allow_hyphen_values = true
    )]
    runtime_options: Vec<String>,

    /// Override //JAVA requested version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Additional java agent, same shape as //JAVAAGENT.
    #[arg(long = "javaagent")]
    java_agents: Vec<String>,

    /// Override //MAIN / inferred class name.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Trust this remote script content hash before exporting.
    #[arg(long = "trust")]
    trust: bool,

    /// Java source file or catalog alias to export.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct ResolveCommand {
    /// Maven coordinates to resolve (groupId:artifactId:version).
    #[arg(required = true)]
    coordinates: Vec<String>,

    /// Additional repository (id=url format or bare URL).
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Print classpath (JAR paths) instead of coordinates.
    #[arg(long = "classpath", short = 'c')]
    classpath: bool,
}

#[derive(Parser, Debug)]
struct FetchCommand {
    /// Maven coordinates to fetch (groupId:artifactId:version).
    #[arg(required = true)]
    coordinates: Vec<String>,

    /// Additional repository (id=url format or bare URL).
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Print resolved coordinates instead of classpath.
    #[arg(long = "deps-only")]
    deps_only: bool,
}

#[derive(Parser, Debug)]
struct CacheClearCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct CachePathCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct CacheListCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Print cache entries as JSON.
    #[arg(long = "json")]
    json: bool,
}

#[derive(Parser, Debug)]
struct InfoCommand {
    #[command(subcommand)]
    command: InfoSubcommand,
}

#[derive(Subcommand, Debug)]
enum InfoSubcommand {
    /// Print classpath used by the script.
    Classpath(InfoClasspathCommand),
    /// Print a json description for tools/IDEs.
    Tools(InfoToolsCommand),
    /// Print documentation references declared by the script.
    Docs(InfoDocsCommand),
    /// Print the effective juv cache directory.
    Cache(InfoCacheCommand),
    /// Print effective main class.
    Main(InfoScriptCommand),
    /// Print requested Java version.
    Java(InfoScriptCommand),
    /// Print script description.
    Description(InfoScriptCommand),
    /// Print Maven GAV.
    Gav(InfoScriptCommand),
    /// Print Java module name.
    Module(InfoScriptCommand),
    /// Print dependency directives.
    Deps(InfoScriptCommand),
    /// Print repository directives.
    Repos(InfoScriptCommand),
    /// Print source directives.
    Sources(InfoScriptCommand),
    /// Print file/resource directives.
    Files(InfoScriptCommand),
    /// Print compile option directives.
    CompileOptions(InfoScriptCommand),
    /// Print runtime/java option directives.
    RuntimeOptions(InfoScriptCommand),
    /// Print native option directives.
    NativeOptions(InfoScriptCommand),
    /// Print java agent directives.
    Javaagents(InfoScriptCommand),
    /// Print manifest directives.
    Manifest(InfoScriptCommand),
    /// Print parsed JBang directives.
    Directives(InfoDirectivesCommand),
}

#[derive(Parser, Debug)]
struct InfoClasspathCommand {
    /// Only include dependency/classpath entries, not compiled script classes.
    #[arg(long = "deps-only")]
    deps_only: bool,

    /// Additional dependency coordinates, same shape as //DEPS.
    #[arg(long = "deps")]
    deps: Vec<String>,
    /// Additional repository, same shape as //REPOS.
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Additional source file, same shape as //SOURCES.
    #[arg(long = "source", alias = "sources")]
    sources: Vec<String>,

    /// Additional file/resource, same shape as //FILES.
    #[arg(long = "files", alias = "file")]
    files: Vec<String>,

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(
        long = "javac-option",
        alias = "compile-option",
        allow_hyphen_values = true
    )]
    javac_options: Vec<String>,

    /// Additional java runtime option, same shape as //JAVA_OPTIONS.
    #[arg(
        long = "runtime-option",
        alias = "java-option",
        allow_hyphen_values = true
    )]
    runtime_options: Vec<String>,

    /// Override //JAVA requested version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Additional java agent, same shape as //JAVAAGENT.
    #[arg(long = "javaagent")]
    java_agents: Vec<String>,

    /// Override //MAIN / inferred class name.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Java source file.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct InfoToolsCommand {
    /// Select a single field from the tools JSON payload.
    #[arg(long = "select")]
    select: Option<String>,

    /// Additional dependency coordinates, same shape as //DEPS.
    #[arg(long = "deps")]
    deps: Vec<String>,
    /// Additional repository, same shape as //REPOS.
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Additional source file, same shape as //SOURCES.
    #[arg(long = "source", alias = "sources")]
    sources: Vec<String>,

    /// Additional file/resource, same shape as //FILES.
    #[arg(long = "files", alias = "file")]
    files: Vec<String>,

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(
        long = "javac-option",
        alias = "compile-option",
        allow_hyphen_values = true
    )]
    javac_options: Vec<String>,

    /// Additional java runtime option, same shape as //JAVA_OPTIONS.
    #[arg(
        long = "runtime-option",
        alias = "java-option",
        allow_hyphen_values = true
    )]
    runtime_options: Vec<String>,

    /// Override //JAVA requested version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Additional java agent, same shape as //JAVAAGENT.
    #[arg(long = "javaagent")]
    java_agents: Vec<String>,

    /// Override //MAIN / inferred class name.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Java source file.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct InfoDocsCommand {
    /// Java source file.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct InfoCacheCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct InfoScriptCommand {
    /// Java source file.
    script: PathBuf,
}

#[derive(Parser, Debug)]
struct InfoDirectivesCommand {
    /// Java source file.
    script: PathBuf,
}

fn repo_json(repo: &str) -> serde_json::Value {
    match repo.split_once('=') {
        Some((id, url)) => serde_json::json!({ "id": id, "url": url }),
        None => serde_json::json!({ "id": null, "url": repo }),
    }
}

fn key_values_json(values: &[juv::KeyValue]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|kv| serde_json::json!({ "key": kv.key, "value": kv.value }))
            .collect(),
    )
}

fn docs_json(values: &[juv::KeyValue]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for kv in values {
        let (id, target) = match &kv.value {
            Some(value) => (kv.key.clone(), value.clone()),
            None => ("main".to_string(), kv.key.clone()),
        };
        let entry = map
            .entry(id)
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if let serde_json::Value::Array(items) = entry {
            items.push(serde_json::json!({ "originalResource": target }));
        }
    }
    serde_json::Value::Object(map)
}

fn print_lines(values: &[String]) {
    for value in values {
        println!("{value}");
    }
}

fn print_key_values(values: &[KeyValue]) {
    for value in values {
        match &value.value {
            Some(v) => println!("{}={}", value.key, v),
            None => println!("{}", value.key),
        }
    }
}

fn split_cli_words(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| split_directive_words(value))
        .collect()
}

fn split_cli_key_values(values: &[String]) -> Vec<KeyValue> {
    split_cli_words(values)
        .into_iter()
        .map(|value| KeyValue::parse(&value))
        .collect()
}

fn export_options(cmd: ExportJarCommand, kind: ExportKind) -> ExportOptions {
    ExportOptions {
        script: cmd.script,
        output: cmd.output,
        force: cmd.force,
        kind,
        extra_deps: split_cli_words(&cmd.deps),
        extra_repos: split_cli_words(&cmd.repos),
        extra_sources: split_cli_words(&cmd.sources),
        extra_files: split_cli_words(&cmd.files),
        classpath: cmd.classpath,
        javac_options: cmd.javac_options,
        runtime_options: cmd.runtime_options,
        java_agents: split_cli_key_values(&cmd.java_agents),
        java_version: cmd.java_version,
        main_class: cmd.main_class,
        cache_dir: cmd.cache_dir,
        trust_remote: cmd.trust,
    }
}

fn print_required(value: Option<&str>, missing: &str) -> Result<()> {
    let Some(value) = value else {
        anyhow::bail!("{missing}");
    };
    println!("{value}");
    Ok(())
}

fn parsed_directives(script: &PathBuf) -> Result<juv::Directives> {
    let source = fs::read_to_string(script)?;
    Ok(juv::parse_directives(&source))
}

fn print_cache_path(cache_dir: Option<PathBuf>) -> Result<()> {
    let cache_dir = match cache_dir {
        Some(path) => path,
        None => default_cache_dir()?,
    };
    println!("{}", cache_dir.display());
    Ok(())
}

fn apply_alias_to_run(mut options: RunOptions) -> Result<RunOptions> {
    if let Some(alias) = alias_for_script(&options.script)? {
        merge_alias_common(
            &alias,
            &mut options.script,
            &mut options.extra_deps,
            &mut options.extra_repos,
            &mut options.extra_sources,
            &mut options.extra_files,
            &mut options.classpath,
            &mut options.javac_options,
            &mut options.runtime_options,
            &mut options.java_agents,
            &mut options.java_version,
            &mut options.main_class,
        );
        options.script_args = prepend(alias.arguments, options.script_args);
    }
    Ok(options)
}

fn apply_alias_to_build(mut options: BuildOptions) -> Result<BuildOptions> {
    if let Some(alias) = alias_for_script(&options.script)? {
        merge_alias_common(
            &alias,
            &mut options.script,
            &mut options.extra_deps,
            &mut options.extra_repos,
            &mut options.extra_sources,
            &mut options.extra_files,
            &mut options.classpath,
            &mut options.javac_options,
            &mut options.runtime_options,
            &mut options.java_agents,
            &mut options.java_version,
            &mut options.main_class,
        );
    }
    Ok(options)
}

fn apply_alias_to_export(mut options: ExportOptions) -> Result<ExportOptions> {
    if let Some(alias) = alias_for_script(&options.script)? {
        merge_alias_common(
            &alias,
            &mut options.script,
            &mut options.extra_deps,
            &mut options.extra_repos,
            &mut options.extra_sources,
            &mut options.extra_files,
            &mut options.classpath,
            &mut options.javac_options,
            &mut options.runtime_options,
            &mut options.java_agents,
            &mut options.java_version,
            &mut options.main_class,
        );
    }
    Ok(options)
}

fn alias_for_script(script: &Path) -> Result<Option<juv::CatalogAlias>> {
    let name = script.to_string_lossy().to_string();
    if script.exists() || name.starts_with("http://") || name.starts_with("https://") {
        return Ok(None);
    }
    resolve_catalog_alias(&name, &std::env::current_dir()?)
}

#[allow(clippy::too_many_arguments)]
fn merge_alias_common(
    alias: &juv::CatalogAlias,
    script: &mut PathBuf,
    extra_deps: &mut Vec<String>,
    extra_repos: &mut Vec<String>,
    extra_sources: &mut Vec<String>,
    extra_files: &mut Vec<String>,
    classpath: &mut Vec<PathBuf>,
    javac_options: &mut Vec<String>,
    runtime_options: &mut Vec<String>,
    java_agents: &mut Vec<KeyValue>,
    java_version: &mut Option<String>,
    main_class: &mut Option<String>,
) {
    *script = alias.script.clone();
    *extra_deps = prepend(alias.deps.clone(), std::mem::take(extra_deps));
    *extra_repos = prepend(alias.repos.clone(), std::mem::take(extra_repos));
    *extra_sources = prepend(alias.sources.clone(), std::mem::take(extra_sources));
    *extra_files = prepend(alias.files.clone(), std::mem::take(extra_files));
    *classpath = prepend(alias.classpaths.clone(), std::mem::take(classpath));
    *javac_options = prepend(alias.javac_options.clone(), std::mem::take(javac_options));
    *runtime_options = prepend(
        alias.runtime_options.clone(),
        std::mem::take(runtime_options),
    );
    *java_agents = prepend(alias.java_agents.clone(), std::mem::take(java_agents));
    if java_version.is_none() {
        *java_version = alias.java_version.clone();
    }
    if main_class.is_none() {
        *main_class = alias.main_class.clone();
    }
}

fn prepend<T>(prefix: Vec<T>, existing: Vec<T>) -> Vec<T> {
    prefix.into_iter().chain(existing).collect()
}

fn print_aliases(json: bool) -> Result<()> {
    let aliases = catalog_aliases(&std::env::current_dir()?)?;
    if json {
        let payload = aliases
            .iter()
            .map(|alias| {
                serde_json::json!({
                    "name": alias.name,
                    "scriptRef": alias.script_ref,
                    "script": alias.script.to_string_lossy(),
                    "description": alias.description,
                    "arguments": alias.arguments,
                    "dependencies": alias.deps,
                    "repositories": alias.repos,
                    "sources": alias.sources,
                    "files": alias.files,
                    "classpaths": alias.classpaths.iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>(),
                    "compileOptions": alias.javac_options,
                    "runtimeOptions": alias.runtime_options,
                    "javaAgents": key_values_json(&alias.java_agents),
                    "javaVersion": alias.java_version,
                    "mainClass": alias.main_class,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        for alias in aliases {
            match alias.description {
                Some(description) => {
                    println!("{}\t{}\t{}", alias.name, alias.script_ref, description)
                }
                None => println!("{}\t{}", alias.name, alias.script_ref),
            }
        }
    }
    Ok(())
}

fn tools_payload(script: &std::path::Path, output: &juv::BuildOutput) -> serde_json::Value {
    let directives = &output.directives;
    serde_json::json!({
        "originalResource": script.to_string_lossy(),
        "backingResource": script.to_string_lossy(),
        "applicationClassesDir": output.classes_dir.to_string_lossy(),
        "applicationJar": null,
        "mainClass": &output.main_class,
        "dependencies": &directives.deps,
        "repositories": directives.repos.iter().map(|repo| repo_json(repo)).collect::<Vec<_>>(),
        "resolvedDependencies": output.classpath.iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "javaVersion": &directives.java_version,
        "requestedJavaVersion": &directives.java_version,
        "compileOptions": &directives.javac_options,
        "runtimeOptions": &directives.runtime_options,
        "nativeOptions": &directives.native_options,
        "javaAgents": key_values_json(&directives.java_agents),
        "manifestOptions": key_values_json(&directives.manifest_options),
        "files": &directives.files,
        "sources": &directives.sources,
        "description": &directives.description,
        "gav": &directives.gav,
        "module": &directives.module,
        "docs": docs_json(&directives.docs),
        "enablePreview": directives.enable_preview,
        "enableCds": directives.enable_cds,
        "disableIntegrations": directives.disable_integrations,
    })
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Some(Commands::Run(cmd)) => run_java(apply_alias_to_run(RunOptions {
            script: cmd.script,
            script_args: cmd.args,
            extra_deps: split_cli_words(&cmd.deps),
            extra_repos: split_cli_words(&cmd.repos),
            extra_sources: split_cli_words(&cmd.sources),
            extra_files: split_cli_words(&cmd.files),
            classpath: cmd.classpath,
            javac_options: cmd.javac_options,
            runtime_options: cmd.runtime_options,
            java_agents: split_cli_key_values(&cmd.java_agents),
            java_version: cmd.java_version,
            main_class: cmd.main_class,
            cache_dir: cmd.cache_dir,
            trust_remote: cmd.trust,
        })?)?,
        Some(Commands::Build(cmd)) => {
            build_java(apply_alias_to_build(BuildOptions {
                script: cmd.script,
                extra_deps: split_cli_words(&cmd.deps),
                extra_repos: split_cli_words(&cmd.repos),
                extra_sources: split_cli_words(&cmd.sources),
                extra_files: split_cli_words(&cmd.files),
                classpath: cmd.classpath,
                javac_options: cmd.javac_options,
                runtime_options: cmd.runtime_options,
                java_agents: split_cli_key_values(&cmd.java_agents),
                java_version: cmd.java_version,
                main_class: cmd.main_class,
                cache_dir: cmd.cache_dir,
                trust_remote: cmd.trust,
            })?)?;
            0
        }
        Some(Commands::Init(cmd)) => {
            if let Some(template) = &cmd.template {
                if template != "hello" && template != "java" {
                    anyhow::bail!("only the default Java init template is supported for now");
                }
            }
            init_script(InitOptions {
                script: cmd.script,
                deps: cmd
                    .deps
                    .iter()
                    .flat_map(|dep| split_directive_words(dep))
                    .collect(),
                java_version: cmd.java_version,
                force: cmd.force,
            })?;
            0
        }
        Some(Commands::Cache(cmd)) => match cmd.command {
            CacheSubcommand::Clear(clear) => {
                clear_cache(clear.cache_dir.as_deref())?;
                0
            }
            CacheSubcommand::Path(path) => {
                print_cache_path(path.cache_dir)?;
                0
            }
            CacheSubcommand::List(list) => {
                let entries = cache_entries(list.cache_dir.as_deref())?;
                if list.json {
                    let json = entries
                        .iter()
                        .map(|entry| {
                            serde_json::json!({
                                "script": entry.script.to_string_lossy(),
                                "classesDir": entry.classes_dir.to_string_lossy(),
                                "cacheDir": entry.cache_dir.to_string_lossy(),
                            })
                        })
                        .collect::<Vec<_>>();
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    for entry in entries {
                        println!(
                            "{}\t{}\t{}",
                            entry.script.display(),
                            entry.classes_dir.display(),
                            entry.cache_dir.display()
                        );
                    }
                }
                0
            }
        },
        Some(Commands::Trust(cmd)) => match cmd.command {
            TrustSubcommand::Add(cmd) => {
                let hash = trust_add(&cmd.url, cmd.cache_dir.as_deref())?;
                println!("{}\t{}", cmd.url, hash);
                0
            }
            TrustSubcommand::Remove(cmd) => {
                trust_remove(&cmd.url, cmd.cache_dir.as_deref())?;
                0
            }
            TrustSubcommand::List(cmd) => {
                for (url, hash) in trust_entries(cmd.cache_dir.as_deref())? {
                    println!("{url}\t{hash}");
                }
                0
            }
            TrustSubcommand::Clear(cmd) => {
                trust_clear(cmd.cache_dir.as_deref())?;
                0
            }
        },
        Some(Commands::Info(cmd)) => match cmd.command {
            InfoSubcommand::Classpath(cmd) => {
                let output = build_java(BuildOptions {
                    script: cmd.script,
                    extra_deps: split_cli_words(&cmd.deps),
                    extra_repos: split_cli_words(&cmd.repos),
                    extra_sources: split_cli_words(&cmd.sources),
                    extra_files: split_cli_words(&cmd.files),
                    classpath: cmd.classpath,
                    javac_options: cmd.javac_options,
                    runtime_options: cmd.runtime_options,
                    java_agents: split_cli_key_values(&cmd.java_agents),
                    java_version: cmd.java_version,
                    main_class: cmd.main_class,
                    cache_dir: cmd.cache_dir,
                    trust_remote: false,
                })?;
                let mut entries = output.classpath;
                if !cmd.deps_only {
                    entries.insert(0, output.classes_dir);
                }
                println!("{}", std::env::join_paths(entries)?.to_string_lossy());
                0
            }
            InfoSubcommand::Tools(cmd) => {
                let script = std::fs::canonicalize(&cmd.script)?;
                let output = build_java(BuildOptions {
                    script: script.clone(),
                    extra_deps: split_cli_words(&cmd.deps),
                    extra_repos: split_cli_words(&cmd.repos),
                    extra_sources: split_cli_words(&cmd.sources),
                    extra_files: split_cli_words(&cmd.files),
                    classpath: cmd.classpath,
                    javac_options: cmd.javac_options,
                    runtime_options: cmd.runtime_options,
                    java_agents: split_cli_key_values(&cmd.java_agents),
                    java_version: cmd.java_version,
                    main_class: cmd.main_class,
                    cache_dir: cmd.cache_dir,
                    trust_remote: false,
                })?;
                let payload = tools_payload(&script, &output);
                if let Some(field) = cmd.select {
                    let Some(value) = payload.get(&field) else {
                        anyhow::bail!("Cannot return value of unknown field: {field}");
                    };
                    if value.is_null() {
                        anyhow::bail!("field {field} is null");
                    }
                    if let Some(text) = value.as_str() {
                        println!("{text}");
                    } else {
                        println!("{}", serde_json::to_string_pretty(value)?);
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                0
            }
            InfoSubcommand::Docs(cmd) => {
                let source = std::fs::read_to_string(&cmd.script)?;
                let directives = juv::parse_directives(&source);
                if let Some(description) = directives.description {
                    println!("{description}");
                }
                for doc in directives.docs {
                    let (id, target) = match doc.value {
                        Some(value) => (doc.key, value),
                        None => ("main".to_string(), doc.key),
                    };
                    println!("{id}:");
                    println!("  {target}");
                }
                0
            }
            InfoSubcommand::Cache(cmd) => {
                print_cache_path(cmd.cache_dir)?;
                0
            }
            InfoSubcommand::Main(cmd) => {
                let source = fs::read_to_string(&cmd.script)?;
                let main = juv::parse_directives(&source)
                    .main_class
                    .or_else(|| juv::infer_main_class_from_source(&cmd.script, &source));
                print_required(main.as_deref(), "could not infer main class; add //MAIN")?;
                0
            }
            InfoSubcommand::Java(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_required(
                    directives.java_version.as_deref(),
                    "no //JAVA directive found",
                )?;
                0
            }
            InfoSubcommand::Description(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_required(
                    directives.description.as_deref(),
                    "no //DESCRIPTION directive found",
                )?;
                0
            }
            InfoSubcommand::Gav(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_required(directives.gav.as_deref(), "no //GAV directive found")?;
                0
            }
            InfoSubcommand::Module(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_required(directives.module.as_deref(), "no //MODULE directive found")?;
                0
            }
            InfoSubcommand::Deps(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.deps);
                0
            }
            InfoSubcommand::Repos(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.repos);
                0
            }
            InfoSubcommand::Sources(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.sources);
                0
            }
            InfoSubcommand::Files(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.files);
                0
            }
            InfoSubcommand::CompileOptions(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.javac_options);
                0
            }
            InfoSubcommand::RuntimeOptions(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.runtime_options);
                0
            }
            InfoSubcommand::NativeOptions(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_lines(&directives.native_options);
                0
            }
            InfoSubcommand::Javaagents(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_key_values(&directives.java_agents);
                0
            }
            InfoSubcommand::Manifest(cmd) => {
                let directives = parsed_directives(&cmd.script)?;
                print_key_values(&directives.manifest_options);
                0
            }
            InfoSubcommand::Directives(cmd) => {
                let source = std::fs::read_to_string(&cmd.script)?;
                println!("{:#?}", juv::parse_directives(&source));
                0
            }
        },
        Some(Commands::App(cmd)) => match cmd.command {
            AppSubcommand::Install(cmd) => {
                let wrapper = app_install(AppInstallOptions {
                    script: cmd.script,
                    name: cmd.name,
                    force: cmd.force,
                })?;
                println!("Command installed: {}", wrapper.display());
                let bin_dir = app_bin_dir()?;
                eprintln!(
                    "Add {} to your PATH to use installed commands.",
                    bin_dir.display()
                );
                0
            }
            AppSubcommand::Uninstall(cmd) => {
                let removed = app_uninstall(&cmd.name)?;
                if removed {
                    println!("Command uninstalled: {}", cmd.name);
                } else {
                    println!("Command '{}' not found.", cmd.name);
                }
                0
            }
            AppSubcommand::List(_) => {
                let entries = app_list()?;
                if entries.is_empty() {
                    println!("No commands installed.");
                } else {
                    for entry in &entries {
                        println!("{}\t{}", entry.name, entry.target);
                    }
                }
                0
            }
        },
        Some(Commands::Alias(cmd)) => match cmd.command {
            AliasSubcommand::List(cmd) => {
                print_aliases(cmd.json)?;
                0
            }
        },
        Some(Commands::Export(cmd)) => match cmd.command {
            ExportSubcommand::Local(cmd) => {
                let output = export_jar(apply_alias_to_export(export_options(
                    cmd,
                    ExportKind::Local,
                ))?)?;
                println!("Exported to {}", output.display());
                0
            }
            ExportSubcommand::Portable(cmd) => {
                let output = export_jar(apply_alias_to_export(export_options(
                    cmd,
                    ExportKind::Portable,
                ))?)?;
                println!("Exported to {}", output.display());
                0
            }
        },
        Some(Commands::Resolve(cmd)) => {
            let cache_dir = match cmd.cache_dir {
                Some(path) => path,
                None => default_cache_dir()?.join("deps"),
            };
            let mut repos = vec![juv::resolver::Repository::central()];
            for repo in &cmd.repos {
                if repo == "central" || repo == "mavenCentral" {
                    continue; // already included
                }
                if let Some((id, url)) = repo.split_once('=') {
                    repos.push(juv::resolver::Repository {
                        id: id.to_string(),
                        url: url.to_string(),
                    });
                } else {
                    repos.push(juv::resolver::Repository {
                        id: repo.clone(),
                        url: repo.clone(),
                    });
                }
            }
            if cmd.classpath {
                let paths = juv::resolver::resolve_classpath(&cmd.coordinates, &repos, &cache_dir)?;
                println!("{}", std::env::join_paths(paths)?.to_string_lossy());
            } else {
                let artifacts = juv::resolver::resolve(&cmd.coordinates, &repos, &cache_dir)?;
                for artifact in &artifacts {
                    println!("{artifact}");
                }
            }
            0
        }
        Some(Commands::Fetch(cmd)) => {
            let cache_dir = match cmd.cache_dir {
                Some(path) => path,
                None => default_cache_dir()?.join("deps"),
            };
            let mut repos = vec![juv::resolver::Repository::central()];
            for repo in &cmd.repos {
                if repo == "central" || repo == "mavenCentral" {
                    continue; // already included
                }
                if let Some((id, url)) = repo.split_once('=') {
                    repos.push(juv::resolver::Repository {
                        id: id.to_string(),
                        url: url.to_string(),
                    });
                } else {
                    repos.push(juv::resolver::Repository {
                        id: repo.clone(),
                        url: repo.clone(),
                    });
                }
            }
            if cmd.deps_only {
                let artifacts = juv::resolver::resolve(&cmd.coordinates, &repos, &cache_dir)?;
                for artifact in &artifacts {
                    println!("{artifact}");
                }
            } else {
                let paths = juv::resolver::resolve_classpath(&cmd.coordinates, &repos, &cache_dir)?;
                println!("{}", std::env::join_paths(paths)?.to_string_lossy());
            }
            0
        }
        Some(Commands::Jdk(cmd)) => match cmd.command {
            JdkSubcommand::List(_) => {
                let jdks = juv::jdk::list_jdks()?;
                if jdks.is_empty() {
                    println!("No JDKs found.");
                } else {
                    for (major, root) in &jdks {
                        println!("{major}\t{major}.x\t{}", root.display());
                    }
                }
                0
            }
            JdkSubcommand::Install(cmd) => {
                let version = juv::jdk::parse_java_version_directive(&cmd.version)?;
                let jdk_root = juv::jdk::install_jdk(version)?;
                println!("JDK {} installed to {}", version, jdk_root.display());
                0
            }
            JdkSubcommand::Home(cmd) => {
                let version = juv::jdk::parse_java_version_directive(&cmd.version)?;
                let jdk_root = juv::jdk::find_jdk(version, false)?;
                println!("{}", jdk_root.display());
                0
            }
        },
        None => {
            let Some(script) = cli.script else {
                eprintln!("No script specified. Try: juv run Hello.java");
                std::process::exit(2);
            };
            run_java(apply_alias_to_run(RunOptions {
                script,
                script_args: cli.args,
                extra_deps: Vec::new(),
                extra_repos: Vec::new(),
                extra_sources: Vec::new(),
                extra_files: Vec::new(),
                classpath: Vec::new(),
                javac_options: Vec::new(),
                runtime_options: Vec::new(),
                java_agents: Vec::new(),
                java_version: None,
                main_class: None,
                cache_dir: None,
                trust_remote: false,
            })?)?
        }
    };
    std::process::exit(code);
}
