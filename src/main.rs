use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{SystemTime, UNIX_EPOCH},
};

use juv::{
    alias_add, alias_remove, app_bin_dir, app_install, app_list, app_uninstall, build_java,
    cache_entries, catalog_add, catalog_aliases, catalog_refs, catalog_templates, clear_cache,
    default_cache_dir, export_jar, export_native, init_script, juvx, resolve_catalog_alias,
    run_java, split_directive_words, trust_add, trust_clear, trust_entries, trust_remove,
    AliasAddOptions, AliasRemoveOptions, AppInstallOptions, BuildOptions, CatalogAddOptions,
    ExportKind, ExportOptions, InitOptions, KeyValue, NativeExportOptions, RunOptions,
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
    /// Check Java source files with javac diagnostics and Error Prone by default.
    Check(CheckCommand),
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
    /// Manage external catalogs from jbang-catalog.json.
    Catalog(CatalogCommand),
    /// Export runnable JARs.
    Export(ExportCommand),
    /// List init templates.
    Template(TemplateCommand),
    /// Resolve Maven dependencies without running.
    Resolve(ResolveCommand),
    /// Fetch Maven dependency artifacts and print classpath.
    Fetch(FetchCommand),
    /// Run JUnit tests with the standalone console launcher.
    Test(TestCommand),
    /// Format Java source files with Palantir Java Format.
    Fmt(FmtCommand),
    /// Run an executable JAR resolved from Maven coordinates.
    Juvx(JuvxCommand),
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
struct CheckCommand {
    /// Emit structured diagnostics JSON.
    #[arg(long = "json")]
    json: bool,

    /// Disable Error Prone checks and run only javac/-Xlint diagnostics.
    #[arg(long = "no-error-prone")]
    no_error_prone: bool,

    /// Error Prone version to use when Error Prone is enabled.
    #[arg(long = "error-prone-version", default_value = DEFAULT_ERROR_PRONE_VERSION)]
    error_prone_version: String,

    /// Treat javac and Error Prone warnings as errors.
    #[arg(long = "warnings-as-errors", alias = "Werror")]
    warnings_as_errors: bool,

    /// Additional dependency coordinates, same shape as //DEPS.
    #[arg(long = "deps")]
    deps: Vec<String>,

    /// Additional repository, same shape as //REPOS.
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

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

    /// Override requested Java version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Java source files or directories. Defaults to the current directory.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
}

#[derive(Parser, Debug)]
struct TestCommand {
    /// Print converted JUnit XML report as JSON.
    #[arg(long = "json", conflicts_with = "xml")]
    json: bool,

    /// Print the generated JUnit XML report.
    #[arg(long = "xml", conflicts_with = "json")]
    xml: bool,

    /// JUnit Platform Console Standalone version to use.
    ///
    /// Defaults to the cached latest Maven Central release, refreshed periodically.
    #[arg(long = "junit-version")]
    junit_version: Option<String>,

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

    /// Additional java runtime option for the JUnit launcher JVM.
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

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Trust this remote script content hash before testing.
    #[arg(long = "trust")]
    trust: bool,

    /// Java test source file or directory. Defaults to the current directory.
    #[arg(default_value = ".")]
    script: PathBuf,

    /// Extra arguments passed to the JUnit ConsoleLauncher after defaults.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Parser, Debug)]
struct FmtCommand {
    /// Check formatting without rewriting files.
    #[arg(long = "check")]
    check: bool,

    /// Palantir Java Format version to use.
    ///
    /// Defaults to the cached latest Maven Central release, refreshed periodically.
    #[arg(long = "formatter-version")]
    formatter_version: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Java source files or directories. Defaults to the current directory.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
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
    /// Add alias for a script reference.
    Add(Box<AliasAddCommand>),
    /// Remove an existing alias.
    Remove(AliasRemoveCommand),
    /// List aliases from the nearest jbang-catalog.json.
    List(AliasListCommand),
}

#[derive(Parser, Debug)]
struct AliasCatalogOptions {
    /// Use the global user catalog file (~/.jbang/jbang-catalog.json).
    #[arg(long = "global", short = 'g', conflicts_with = "file")]
    global: bool,

    /// Path to the catalog file or directory to use.
    #[arg(long = "file", short = 'f')]
    file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct AliasAddCommand {
    #[command(flatten)]
    catalog: AliasCatalogOptions,

    /// Alias name (defaults to the script filename stem).
    #[arg(long = "name")]
    name: Option<String>,

    /// Description for the alias.
    #[arg(long = "description")]
    description: Option<String>,

    /// Force overwrite of an existing alias.
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
    #[arg(long = "files")]
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

    /// Requested Java version.
    #[arg(long = "java")]
    java_version: Option<String>,

    /// Additional java agent, same shape as //JAVAAGENT.
    #[arg(long = "javaagent")]
    java_agents: Vec<String>,

    /// Main class for the alias.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Documentation reference for the alias.
    #[arg(long = "docs")]
    docs: Vec<String>,

    /// Script path, URL, or alias reference.
    script: String,

    /// Arguments stored in the alias and prepended at run time.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Parser, Debug)]
struct AliasRemoveCommand {
    #[command(flatten)]
    catalog: AliasCatalogOptions,

    /// Alias name to remove.
    name: String,
}

#[derive(Parser, Debug)]
struct AliasListCommand {
    /// Print JSON instead of tab-separated text.
    #[arg(long = "json")]
    json: bool,
}

#[derive(Parser, Debug)]
struct CatalogCommand {
    #[command(subcommand)]
    command: CatalogSubcommand,
}

#[derive(Subcommand, Debug)]
enum CatalogSubcommand {
    /// Add an external catalog reference.
    Add(CatalogAddCommand),
    /// List external catalog references.
    List(CatalogListCommand),
}

#[derive(Parser, Debug)]
struct CatalogAddCommand {
    #[command(flatten)]
    catalog: AliasCatalogOptions,

    /// Catalog name.
    name: String,

    /// Catalog path, URL, or directory.
    catalog_ref: String,

    /// Description for the catalog.
    #[arg(long = "description")]
    description: Option<String>,

    /// Import aliases and templates from this catalog into local lookup.
    #[arg(long = "import")]
    import_items: bool,

    /// Force overwrite of an existing catalog reference.
    #[arg(long = "force")]
    force: bool,
}

#[derive(Parser, Debug)]
struct CatalogListCommand {
    /// Print JSON instead of tab-separated text.
    #[arg(long = "json")]
    json: bool,
}

#[derive(Parser, Debug)]
struct TemplateCommand {
    #[command(subcommand)]
    command: TemplateSubcommand,
}

#[derive(Subcommand, Debug)]
enum TemplateSubcommand {
    /// List built-in init templates.
    List(TemplateListCommand),
}

#[derive(Parser, Debug)]
struct TemplateListCommand {
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
    /// Export a native executable using GraalVM native-image.
    Native(ExportNativeCommand),
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
struct ExportNativeCommand {
    /// Output executable path (defaults to <script> with platform executable suffix).
    #[arg(long = "output", short = 'o')]
    output: Option<PathBuf>,

    /// Force overwrite of existing output files.
    #[arg(long = "force")]
    force: bool,

    /// Path to native-image executable (defaults to JDK bin/native-image or PATH).
    #[arg(long = "native-image")]
    native_image: Option<PathBuf>,

    /// Additional native-image option, same shape as //NATIVE_OPTIONS.
    #[arg(
        long = "native-option",
        alias = "native-options",
        allow_hyphen_values = true
    )]
    native_options: Vec<String>,

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
struct JuvxCommand {
    /// Maven coordinate to resolve and run (groupId:artifactId[:classifier]:version).
    coordinate: String,

    /// Additional repository (id=url format or bare URL).
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Override dependency cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Main class to launch with the resolved classpath instead of java -jar.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Arguments passed to the launched Java tool after `--`.
    #[arg(last = true)]
    args: Vec<String>,
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

fn native_export_options(cmd: ExportNativeCommand) -> NativeExportOptions {
    NativeExportOptions {
        script: cmd.script,
        output: cmd.output,
        force: cmd.force,
        native_image: cmd.native_image,
        extra_native_options: split_cli_words(&cmd.native_options),
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

fn apply_alias_to_native_export(mut options: NativeExportOptions) -> Result<NativeExportOptions> {
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

fn print_catalogs(json: bool) -> Result<()> {
    let catalogs = catalog_refs(&std::env::current_dir()?)?;
    if json {
        let payload = catalogs
            .iter()
            .map(|catalog| {
                serde_json::json!({
                    "name": catalog.name,
                    "catalogRef": catalog.catalog_ref,
                    "catalog": catalog.catalog.to_string_lossy(),
                    "description": catalog.description,
                    "import": catalog.import_items,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        for catalog in catalogs {
            match catalog.description {
                Some(description) => {
                    println!("{}\t{}\t{}", catalog.name, catalog.catalog_ref, description)
                }
                None => println!("{}\t{}", catalog.name, catalog.catalog_ref),
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

fn run_juvx(cmd: JuvxCommand) -> Result<i32> {
    juvx::run(juvx::JuvxOptions {
        coordinate: cmd.coordinate,
        repos: cmd.repos,
        cache_dir: cmd.cache_dir,
        main_class: cmd.main_class,
        args: cmd.args,
    })
}

fn format_cli_java_agent(agent: &KeyValue) -> String {
    match &agent.value {
        Some(value) => format!("-javaagent:{}={}", agent.key, value),
        None => format!("-javaagent:{}", agent.key),
    }
}

const DEFAULT_PALANTIR_JAVA_FORMAT_VERSION: &str = "2.91.0";
const TOOL_VERSION_CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
const PALANTIR_GROUP_PATH: &str = "com/palantir/javaformat";
const PALANTIR_MAIN_CLASS: &str = "com.palantir.javaformat.java.Main";
const COMPACT_WRAPPER_CLASS: &str = "__JuvFormatterWrapper";
const PALANTIR_GROUP_ID: &str = "com.palantir.javaformat";
const PALANTIR_ARTIFACT_ID: &str = "palantir-java-format";

#[derive(Debug, Clone)]
enum FormatterBackend {
    Native(PathBuf),
    Jar {
        java: PathBuf,
        classpath: Vec<PathBuf>,
    },
}

fn run_fmt(cmd: FmtCommand) -> Result<i32> {
    let files = collect_java_files(&cmd.paths)?;
    if files.is_empty() {
        return Ok(0);
    }
    let backend =
        resolve_formatter_backend(cmd.cache_dir.as_deref(), cmd.formatter_version.as_deref())?;
    let mut changed = Vec::new();
    for file in files {
        if format_one_file(&backend, &file, cmd.check)? {
            changed.push(file);
        }
    }
    if cmd.check && !changed.is_empty() {
        for file in &changed {
            eprintln!("would reformat {}", file.display());
        }
        return Ok(1);
    }
    Ok(0)
}

fn collect_java_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            if is_java_file(path) {
                files.push(path.clone());
            }
            continue;
        }
        if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_entry(|entry| {
                    !entry.file_type().is_dir() || !is_ignored_fmt_dir(entry.path())
                })
            {
                let entry = entry.with_context(|| format!("failed to read {}", path.display()))?;
                let entry_path = entry.path();
                if entry.file_type().is_file() && is_java_file(entry_path) {
                    files.push(entry_path.to_path_buf());
                }
            }
            continue;
        }
        return Err(anyhow::anyhow!("fmt path not found: {}", path.display()));
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn is_java_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("java")
}

fn is_ignored_fmt_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "target" | "build" | ".gradle" | ".jbang"))
}

fn resolve_formatter_backend(
    cache_dir: Option<&Path>,
    version: Option<&str>,
) -> Result<FormatterBackend> {
    if version.is_none() {
        if let Ok(path) = which::which("palantir-java-format") {
            return Ok(FormatterBackend::Native(path));
        }
    }
    let version = match version {
        Some(version) => version.to_string(),
        None => latest_cached_tool_version(
            cache_dir,
            PALANTIR_GROUP_ID,
            PALANTIR_ARTIFACT_ID,
            &[juv::resolver::Repository::central()],
        )
        .unwrap_or_else(|err| {
            eprintln!(
                "warning: could not determine latest Palantir Java Format version: {err:#}; using {DEFAULT_PALANTIR_JAVA_FORMAT_VERSION}"
            );
            DEFAULT_PALANTIR_JAVA_FORMAT_VERSION.to_string()
        }),
    };
    if let Some(native) = cached_or_downloaded_native_formatter(cache_dir, &version)? {
        return Ok(FormatterBackend::Native(native));
    }
    cached_or_downloaded_jar_formatter(cache_dir, &version)
}

fn cached_or_downloaded_native_formatter(
    cache_dir: Option<&Path>,
    version: &str,
) -> Result<Option<PathBuf>> {
    let Some(classifier) = native_formatter_classifier() else {
        return Ok(None);
    };
    let root = cache_root(cache_dir)?
        .join("formatters")
        .join("palantir-java-format")
        .join(version);
    let bin = root.join(format!("palantir-java-format-{classifier}"));
    if bin.exists() {
        return Ok(Some(bin));
    }
    fs::create_dir_all(&root)?;
    let base = format!("https://repo1.maven.org/maven2/{PALANTIR_GROUP_PATH}/palantir-java-format-native/{version}");
    let artifact = format!("palantir-java-format-native-{version}-nativeImage-{classifier}.bin");
    let url = format!("{base}/{artifact}");
    let sha_url = format!("{url}.sha256");
    let bytes = match ureq::get(&url).call() {
        Ok(response) => {
            let mut reader = response.into_reader();
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            bytes
        }
        Err(_) => return Ok(None),
    };
    let sha_text = ureq::get(&sha_url)
        .call()
        .with_context(|| format!("failed to fetch checksum for {url}"))?
        .into_string()
        .with_context(|| format!("failed to read checksum for {url}"))?;
    let expected = sha_text
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty checksum response for {url}"))?;
    let actual = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
    if actual != expected {
        return Err(anyhow::anyhow!("checksum mismatch for {url}"));
    }
    fs::write(&bin, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&bin)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions)?;
    }
    Ok(Some(bin))
}

fn native_formatter_classifier() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-glibc_x86-64"),
        ("linux", "aarch64") => Some("linux-glibc_aarch64"),
        ("macos", "aarch64") => Some("macos_aarch64"),
        _ => None,
    }
}

fn cached_or_downloaded_jar_formatter(
    cache_dir: Option<&Path>,
    version: &str,
) -> Result<FormatterBackend> {
    let cache = cache_root(cache_dir)?.join("deps");
    let coordinate = format!("com.palantir.javaformat:palantir-java-format:{version}");
    let repos = vec![juv::resolver::Repository::central()];
    let classpath = juv::resolver::resolve_classpath(&[coordinate], &repos, &cache)?;
    let java = juv::jdk::java_bin_path(&juv::jdk::resolve_jdk(&None, true)?);
    Ok(FormatterBackend::Jar { java, classpath })
}

fn cache_root(cache_dir: Option<&Path>) -> Result<PathBuf> {
    Ok(match cache_dir {
        Some(path) => path.to_path_buf(),
        None => default_cache_dir()?,
    })
}

fn format_one_file(backend: &FormatterBackend, file: &Path, check: bool) -> Result<bool> {
    let source =
        fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))?;
    if is_compact_source(&source) {
        let formatted = format_compact_source(backend, &source, file)?;
        let changed = formatted != source;
        if changed && !check {
            fs::write(file, formatted)
                .with_context(|| format!("failed to write {}", file.display()))?;
        }
        return Ok(changed);
    }
    if check {
        let output = formatter_command(backend)
            .arg("--dry-run")
            .arg("--set-exit-if-changed")
            .arg(file)
            .output()
            .with_context(|| format!("failed to execute formatter for {}", file.display()))?;
        if output.status.success() {
            return Ok(false);
        }
        if output.status.code() == Some(1) {
            return Ok(true);
        }
        return Err(formatter_error(file, output));
    }
    let output = formatter_command(backend)
        .arg("--replace")
        .arg(file)
        .output()
        .with_context(|| format!("failed to execute formatter for {}", file.display()))?;
    if !output.status.success() {
        return Err(formatter_error(file, output));
    }
    let updated =
        fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))?;
    Ok(updated != source)
}

fn formatter_command(backend: &FormatterBackend) -> ProcessCommand {
    match backend {
        FormatterBackend::Native(path) => ProcessCommand::new(path),
        FormatterBackend::Jar { java, classpath } => {
            let mut command = ProcessCommand::new(java);
            command
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.parser=ALL-UNNAMED")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED")
                .arg("-cp")
                .arg(std::env::join_paths(classpath).unwrap_or_default())
                .arg(PALANTIR_MAIN_CLASS);
            command
        }
    }
}

fn formatter_error(file: &Path, output: std::process::Output) -> anyhow::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    anyhow::anyhow!(
        "formatter failed for {} with exit code {}\n{}{}",
        file.display(),
        output.status.code().unwrap_or(1),
        stdout,
        stderr
    )
}

fn is_compact_source(source: &str) -> bool {
    let mut brace_depth = 0usize;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let at_top_level = brace_depth == 0;
        let starts_type = starts_with_java_type_declaration(trimmed);
        if at_top_level && !starts_type && trimmed.starts_with("void main(") {
            return true;
        }
        brace_depth = update_brace_depth(brace_depth, line);
    }
    false
}

fn update_brace_depth(mut depth: usize, line: &str) -> usize {
    for ch in line.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn starts_with_java_type_declaration(trimmed: &str) -> bool {
    const TYPE_DECLARATION_PREFIXES: &[&str] = &[
        "class ",
        "abstract class ",
        "sealed class ",
        "non-sealed class ",
        "final class ",
        "public class ",
        "public abstract class ",
        "public sealed class ",
        "public non-sealed class ",
        "public final class ",
        "record ",
        "public record ",
        "interface ",
        "public interface ",
        "enum ",
        "public enum ",
        "@interface ",
        "public @interface ",
    ];
    TYPE_DECLARATION_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn format_compact_source(backend: &FormatterBackend, source: &str, file: &Path) -> Result<String> {
    let (prefix, body) = split_compact_prefix(source);
    let indented_body = body
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let wrapped = format!("{prefix}class {COMPACT_WRAPPER_CLASS} {{\n{indented_body}\n}}\n");
    let output = formatter_command(backend)
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(wrapped.as_bytes())?;
            }
            child.wait_with_output()
        })
        .with_context(|| format!("failed to execute formatter for {}", file.display()))?;
    if !output.status.success() {
        return Err(formatter_error(file, output));
    }
    let formatted =
        String::from_utf8(output.stdout).context("formatter emitted non-UTF-8 output")?;
    unwrap_compact_source(&formatted)
}

fn split_compact_prefix(source: &str) -> (String, String) {
    let mut prefix = String::new();
    let mut body = String::new();
    let mut in_prefix = true;
    let mut in_block_comment = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if in_prefix {
            let prefix_line = in_block_comment
                || trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("#!")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("/*");
            if prefix_line {
                prefix.push_str(line);
                prefix.push('\n');
                if in_block_comment || trimmed.starts_with("/*") {
                    in_block_comment = !trimmed.contains("*/");
                }
                continue;
            }
            in_prefix = false;
        }
        body.push_str(line);
        body.push('\n');
    }
    (prefix, body.trim_end().to_string())
}

fn unwrap_compact_source(formatted: &str) -> Result<String> {
    let lines = formatted.lines().collect::<Vec<_>>();
    let wrapper_index = lines
        .iter()
        .position(|line| line.trim() == format!("class {COMPACT_WRAPPER_CLASS} {{"))
        .ok_or_else(|| anyhow::anyhow!("formatter output did not contain compact wrapper"))?;
    let wrapper_end = lines
        .iter()
        .enumerate()
        .skip(wrapper_index + 1)
        .filter_map(|(index, line)| (line.trim() == "}").then_some(index))
        .next_back()
        .ok_or_else(|| anyhow::anyhow!("formatter output did not contain compact wrapper end"))?;

    let mut out = String::new();
    for line in &lines[..wrapper_index] {
        out.push_str(line);
        out.push('\n');
    }
    let body_lines = &lines[wrapper_index + 1..wrapper_end];
    let indent = body_lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            (!trimmed.is_empty()).then_some(line.len() - trimmed.len())
        })
        .min()
        .unwrap_or(0);
    for line in body_lines {
        if line.len() >= indent {
            out.push_str(&line[indent..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    Ok(out)
}

const DEFAULT_JUNIT_PLATFORM_VERSION: &str = "6.1.0";

fn run_check(cmd: CheckCommand) -> Result<i32> {
    let files = collect_java_files(&cmd.paths)?;
    if files.is_empty() {
        if cmd.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "files": [],
                    "diagnostics": [],
                    "errorProne": !cmd.no_error_prone,
                }))?
            );
        }
        return Ok(0);
    }

    let jdk_root = juv::jdk::resolve_jdk(&cmd.java_version, true)?;
    let javac = juv::jdk::javac_bin_path(&jdk_root);
    let java = juv::jdk::java_bin_path(&jdk_root);
    let root = cache_root(cmd.cache_dir.as_deref())?.join("check");
    let wrapper_dir = root.join("compiler-wrapper");
    fs::create_dir_all(&wrapper_dir)?;
    let wrapper_source = wrapper_dir.join("JuvCheckCompiler.java");
    let wrapper_class = wrapper_dir.join("JuvCheckCompiler.class");
    fs::write(&wrapper_source, CHECK_COMPILER_SOURCE)?;
    let wrapper_needs_compile = !wrapper_class.exists()
        || fs::metadata(&wrapper_source)?.modified()? > fs::metadata(&wrapper_class)?.modified()?;
    if wrapper_needs_compile {
        let status = ProcessCommand::new(&javac)
            .arg(&wrapper_source)
            .status()
            .with_context(|| format!("failed to execute {}", javac.display()))?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "failed to compile juv check compiler wrapper with exit code {}",
                status.code().unwrap_or(1)
            ));
        }
    }

    let mut compiler_options = vec!["-Xlint:all".to_string(), "-proc:none".to_string()];
    let classes_dir = root.join("classes");
    if classes_dir.exists() {
        fs::remove_dir_all(&classes_dir)?;
    }
    fs::create_dir_all(&classes_dir)?;
    compiler_options.push("-d".to_string());
    compiler_options.push(classes_dir.to_string_lossy().to_string());

    let dep_coordinates = split_cli_words(&cmd.deps);
    let mut classpath = cmd.classpath;
    if !dep_coordinates.is_empty() {
        let repos = juvx::maven_repositories(&split_cli_words(&cmd.repos));
        let cache_dir = cache_root(cmd.cache_dir.as_deref())?.join("deps");
        classpath.extend(juv::resolver::resolve_classpath(
            &dep_coordinates,
            &repos,
            &cache_dir,
        )?);
    }
    if !classpath.is_empty() {
        compiler_options.push("-classpath".to_string());
        compiler_options.push(
            std::env::join_paths(&classpath)?
                .to_string_lossy()
                .to_string(),
        );
    }
    compiler_options.extend(cmd.javac_options);
    if cmd.warnings_as_errors {
        compiler_options.push("-Werror".to_string());
    }

    let mut wrapper_classpath = vec![wrapper_dir.clone()];
    if !cmd.no_error_prone {
        let repos = juvx::maven_repositories(&split_cli_words(&cmd.repos));
        let cache_dir = cache_root(cmd.cache_dir.as_deref())?.join("deps");
        let error_prone_coordinate = format!(
            "{ERROR_PRONE_GROUP_ID}:{ERROR_PRONE_ARTIFACT_ID}:{}",
            cmd.error_prone_version
        );
        let error_prone_cp =
            juv::resolver::resolve_classpath(&[error_prone_coordinate], &repos, &cache_dir)?;
        wrapper_classpath.extend(error_prone_cp);
        compiler_options.push("-XDcompilePolicy=simple".to_string());
        compiler_options.push("--should-stop=ifError=FLOW".to_string());
        compiler_options.push("-Xplugin:ErrorProne".to_string());
    }

    let output = check_java_command(&java, &wrapper_classpath, &compiler_options, &files)?
        .output()
        .with_context(|| format!("failed to execute {}", java.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if cmd.json {
        print!("{stdout}");
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        return Ok(output.status.code().unwrap_or(1));
    }

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("invalid juv check wrapper output: {stdout}"))?;
    print_check_human(&payload)?;
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output.status.code().unwrap_or(1))
}

fn check_java_command<'a>(
    java: &'a Path,
    wrapper_classpath: &'a [PathBuf],
    compiler_options: &'a [String],
    files: &'a [PathBuf],
) -> Result<ProcessCommand> {
    let mut command = ProcessCommand::new(java);
    command.args(error_prone_jdk_flags());
    command.arg("-cp").arg(
        std::env::join_paths(wrapper_classpath)
            .context("failed to build juv check compiler wrapper classpath")?,
    );
    command.arg("JuvCheckCompiler");
    command.args(compiler_options);
    command.arg("--");
    command.args(files);
    Ok(command)
}

fn error_prone_jdk_flags() -> [&'static str; 10] {
    [
        "--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.main=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.model=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.parser=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.processing=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED",
        "--add-opens=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED",
        "--add-opens=jdk.compiler/com.sun.tools.javac.comp=ALL-UNNAMED",
    ]
}

fn print_check_human(payload: &serde_json::Value) -> Result<()> {
    let diagnostics = payload
        .get("diagnostics")
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("check output did not contain diagnostics array"))?;
    for diagnostic in diagnostics {
        let kind = diagnostic
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or("UNKNOWN")
            .to_ascii_lowercase();
        let file = diagnostic
            .get("file")
            .and_then(|value| value.as_str())
            .unwrap_or("<compiler>");
        let line = diagnostic
            .get("line")
            .and_then(|value| value.as_i64())
            .unwrap_or(-1);
        let column = diagnostic
            .get("column")
            .and_then(|value| value.as_i64())
            .unwrap_or(-1);
        let message = diagnostic
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if line > 0 && column > 0 {
            println!("{file}:{line}:{column}: {kind}: {message}");
        } else {
            println!("{file}: {kind}: {message}");
        }
    }
    if diagnostics.is_empty() {
        println!("check passed");
    }
    Ok(())
}

const ERROR_PRONE_GROUP_ID: &str = "com.google.errorprone";
const ERROR_PRONE_ARTIFACT_ID: &str = "error_prone_core";
const DEFAULT_ERROR_PRONE_VERSION: &str = "2.39.0";

const CHECK_COMPILER_SOURCE: &str = r#"
import javax.tools.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

public class JuvCheckCompiler {
  public static void main(String[] args) throws Exception {
    List<String> options = new ArrayList<>();
    List<String> files = new ArrayList<>();
    boolean afterSeparator = false;
    for (String arg : args) {
      if (arg.equals("--")) {
        afterSeparator = true;
      } else if (afterSeparator) {
        files.add(arg);
      } else {
        options.add(arg);
      }
    }

    JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
    if (compiler == null) {
      System.err.println("No system Java compiler available. Run with a JDK, not a JRE.");
      System.exit(2);
    }

    DiagnosticCollector<JavaFileObject> diagnostics = new DiagnosticCollector<>();
    StandardJavaFileManager fm = compiler.getStandardFileManager(diagnostics, Locale.ROOT, StandardCharsets.UTF_8);
    Iterable<? extends JavaFileObject> units = fm.getJavaFileObjectsFromStrings(files);
    StringWriter compilerOut = new StringWriter();
    Boolean ok = compiler.getTask(compilerOut, fm, diagnostics, options, null, units).call();

    StringBuilder sb = new StringBuilder();
    sb.append("{\n  \"ok\": ").append(Boolean.TRUE.equals(ok)).append(",\n  \"diagnostics\": [\n");
    List<Diagnostic<? extends JavaFileObject>> ds = diagnostics.getDiagnostics();
    for (int i = 0; i < ds.size(); i++) {
      Diagnostic<? extends JavaFileObject> d = ds.get(i);
      sb.append("    {");
      field(sb, "kind", d.getKind().toString()); sb.append(",");
      field(sb, "code", d.getCode()); sb.append(",");
      field(sb, "file", d.getSource() == null ? null : new File(d.getSource().toUri()).getPath()); sb.append(",");
      sb.append("\"line\": ").append(d.getLineNumber()).append(",");
      sb.append("\"column\": ").append(d.getColumnNumber()).append(",");
      field(sb, "message", d.getMessage(Locale.ROOT));
      sb.append("}");
      if (i + 1 < ds.size()) sb.append(",");
      sb.append("\n");
    }
    sb.append("  ],\n");
    field(sb, "compilerOutput", compilerOut.toString());
    sb.append("\n}\n");
    System.out.print(sb);
    fm.close();
    System.exit(Boolean.TRUE.equals(ok) ? 0 : 1);
  }

  private static void field(StringBuilder sb, String name, String value) {
    sb.append("\"").append(esc(name)).append("\": ");
    if (value == null) {
      sb.append("null");
    } else {
      sb.append("\"").append(esc(value)).append("\"");
    }
  }

  private static String esc(String s) {
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < s.length(); i++) {
      char c = s.charAt(i);
      switch (c) {
        case '\\': out.append("\\\\"); break;
        case '"': out.append("\\\""); break;
        case '\n': out.append("\\n"); break;
        case '\r': out.append("\\r"); break;
        case '\t': out.append("\\t"); break;
        default:
          if (c < 0x20) {
            out.append(String.format("\\u%04x", (int)c));
          } else {
            out.append(c);
          }
      }
    }
    return out.toString();
  }
}
"#;

const JUNIT_GROUP_ID: &str = "org.junit.platform";
const JUNIT_ARTIFACT_ID: &str = "junit-platform-console-standalone";

fn run_tests(cmd: TestCommand) -> Result<i32> {
    let junit_version = match cmd.junit_version.clone() {
        Some(version) => version,
        None => latest_cached_tool_version(
            cmd.cache_dir.as_deref(),
            JUNIT_GROUP_ID,
            JUNIT_ARTIFACT_ID,
            &[juv::resolver::Repository::central()],
        )
        .unwrap_or_else(|err| {
            eprintln!(
                "warning: could not determine latest JUnit Platform Console Standalone version: {err:#}; using {DEFAULT_JUNIT_PLATFORM_VERSION}"
            );
            DEFAULT_JUNIT_PLATFORM_VERSION.to_string()
        }),
    };
    let launcher_coordinate =
        format!("org.junit.platform:junit-platform-console-standalone:{junit_version}");
    let mut deps = split_cli_words(&cmd.deps);
    deps.push(launcher_coordinate);

    let (script, inferred_directory_sources) = expand_test_target(&cmd.script)?;
    let mut extra_sources = split_cli_words(&cmd.sources);
    extra_sources.extend(inferred_directory_sources);
    extra_sources.extend(infer_test_companion_sources(&script));
    dedupe_strings(&mut extra_sources);

    let build = build_java(BuildOptions {
        script,
        extra_deps: deps,
        extra_repos: split_cli_words(&cmd.repos),
        extra_sources,
        extra_files: split_cli_words(&cmd.files),
        classpath: cmd.classpath,
        javac_options: cmd.javac_options,
        runtime_options: Vec::new(),
        java_agents: split_cli_key_values(&cmd.java_agents),
        java_version: cmd.java_version,
        main_class: None,
        cache_dir: cmd.cache_dir,
        trust_remote: cmd.trust,
    })?;

    let launcher = build
        .classpath
        .iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("junit-platform-console-standalone-"))
        })
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("could not resolve junit-platform-console-standalone"))?;

    let reports_dir = junit_reports_dir()?;
    fs::create_dir_all(&reports_dir)?;
    let mut runtime_cp = vec![build.classes_dir.clone()];
    runtime_cp.extend(build.classpath.clone());

    let jdk_root = juv::jdk::resolve_jdk(&build.directives.java_version, true)?;
    let java = juv::jdk::java_bin_path(&jdk_root).display().to_string();
    let mut java_cmd = ProcessCommand::new(&java);
    for agent in &build.directives.java_agents {
        java_cmd.arg(format_cli_java_agent(agent));
    }
    java_cmd.args(&build.directives.runtime_options);
    java_cmd.args(&cmd.runtime_options);
    java_cmd
        .arg("-jar")
        .arg(&launcher)
        .arg("execute")
        .arg("--class-path")
        .arg(std::env::join_paths(&runtime_cp)?)
        .arg("--scan-class-path")
        .arg("--reports-dir")
        .arg(&reports_dir);
    if cmd.json || cmd.xml {
        java_cmd.arg("--details=none").arg("--disable-banner");
    }
    java_cmd.args(&cmd.args);

    let output = java_cmd
        .output()
        .with_context(|| format!("failed to execute {java}"))?;
    let code = output.status.code().unwrap_or(1);
    let xml = read_junit_xml_reports(&reports_dir)?;
    let _ = fs::remove_dir_all(&reports_dir);

    if (cmd.json || cmd.xml) && !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if cmd.json {
        let payload = junit_xml_to_json(&xml)?;
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if cmd.xml {
        print!("{xml}");
    } else {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(code)
}

fn junit_reports_dir() -> Result<PathBuf> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!("juv-junit-{}-{nanos}", std::process::id())))
}

fn read_junit_xml_reports(reports_dir: &Path) -> Result<String> {
    let mut reports = fs::read_dir(reports_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "xml"))
        .collect::<Vec<_>>();
    reports.sort();
    if reports.is_empty() {
        return Ok(String::new());
    }
    if reports.len() == 1 {
        return Ok(fs::read_to_string(&reports[0])?);
    }
    let mut xml = String::from("<testsuites>\n");
    for report in reports {
        let report_xml = fs::read_to_string(report)?;
        xml.push_str(strip_xml_declaration(report_xml.trim_start()));
        xml.push('\n');
    }
    xml.push_str("</testsuites>\n");
    Ok(xml)
}

fn strip_xml_declaration(xml: &str) -> &str {
    let Some(rest) = xml.strip_prefix("<?xml") else {
        return xml;
    };
    rest.find("?>")
        .map(|end| rest[end + 2..].trim_start())
        .unwrap_or(xml)
}

fn junit_xml_to_json(xml: &str) -> Result<serde_json::Value> {
    let suite_re = regex::Regex::new(r#"<testsuite\b([^>]*)>"#)?;
    let case_re = regex::Regex::new(r#"(?s)<testcase\b([^>]*?)(?:/>|>(.*?)</testcase>)"#)?;
    let mut tests = 0_u64;
    let mut failures = 0_u64;
    let mut errors = 0_u64;
    let mut skipped = 0_u64;
    let mut test_cases = Vec::new();

    for captures in suite_re.captures_iter(xml) {
        let attrs = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        tests += xml_attr(attrs, "tests")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        failures += xml_attr(attrs, "failures")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        errors += xml_attr(attrs, "errors")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        skipped += xml_attr(attrs, "skipped")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
    }

    for captures in case_re.captures_iter(xml) {
        let attrs = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        let body = captures.get(2).map(|m| m.as_str()).unwrap_or_default();
        let status = if body.contains("<failure") {
            "failed"
        } else if body.contains("<error") {
            "errored"
        } else if body.contains("<skipped") {
            "skipped"
        } else {
            "passed"
        };
        test_cases.push(serde_json::json!({
            "className": xml_attr(attrs, "classname").unwrap_or_default(),
            "name": xml_attr(attrs, "name").unwrap_or_default(),
            "time": xml_attr(attrs, "time").unwrap_or_default(),
            "status": status,
        }));
    }

    Ok(serde_json::json!({
        "tests": tests,
        "failures": failures,
        "errors": errors,
        "skipped": skipped,
        "testCases": test_cases,
    }))
}

fn xml_attr(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs.trim_start();
    while !rest.is_empty() {
        let eq = rest.find('=')?;
        let attr_name = rest[..eq].trim();
        let after_eq = rest[eq + 1..].trim_start();
        let quote = after_eq.chars().next()?;
        let (value, next) = if quote == '"' || quote == '\'' {
            let value_start = quote.len_utf8();
            match after_eq[value_start..].find(quote) {
                Some(end) => {
                    let value_end = value_start + end;
                    (
                        &after_eq[value_start..value_end],
                        &after_eq[value_end + quote.len_utf8()..],
                    )
                }
                None => return None,
            }
        } else {
            let end = after_eq.find(char::is_whitespace).unwrap_or(after_eq.len());
            (&after_eq[..end], &after_eq[end..])
        };
        if attr_name == name {
            return Some(value.to_string());
        }
        rest = next.trim_start();
    }
    None
}

fn latest_cached_tool_version(
    cache_dir: Option<&Path>,
    group_id: &str,
    artifact_id: &str,
    repos: &[juv::resolver::Repository],
) -> Result<String> {
    let metadata_dir = cache_root(cache_dir)?.join("metadata");
    let cache_file = metadata_dir.join(format!("{group_id}.{artifact_id}.version"));
    if let Ok(metadata) = fs::metadata(&cache_file) {
        if let Ok(modified) = metadata.modified() {
            if SystemTime::now()
                .duration_since(modified)
                .map(|age| age.as_secs() < TOOL_VERSION_CACHE_MAX_AGE_SECS)
                .unwrap_or(false)
            {
                let cached = fs::read_to_string(&cache_file)?.trim().to_string();
                if !cached.is_empty() {
                    return Ok(cached);
                }
            }
        }
    }
    let version = latest_tool_version(group_id, artifact_id, repos)?;
    fs::create_dir_all(&metadata_dir)?;
    fs::write(&cache_file, format!("{version}\n"))?;
    Ok(version)
}

fn latest_tool_version(
    group_id: &str,
    artifact_id: &str,
    repos: &[juv::resolver::Repository],
) -> Result<String> {
    juv::resolver::resolve_latest_version(
        &juv::resolver::Module {
            org: group_id.to_string(),
            name: artifact_id.to_string(),
        },
        repos,
    )
}

fn expand_test_target(path: &Path) -> Result<(PathBuf, Vec<String>)> {
    if !path.is_dir() {
        return Ok((path.to_path_buf(), Vec::new()));
    }

    let mut java_files = fs::read_dir(path)
        .with_context(|| format!("failed to read test directory {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|entry_path| entry_path.extension().is_some_and(|ext| ext == "java"))
        .collect::<Vec<_>>();
    java_files.sort();

    let script = java_files
        .iter()
        .find(|entry_path| is_test_source(entry_path))
        .or_else(|| java_files.first())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no Java source files found in {}", path.display()))?;

    let extra_sources = java_files
        .into_iter()
        .filter(|entry_path| entry_path != &script)
        .filter_map(|entry_path| {
            entry_path
                .strip_prefix(path)
                .ok()
                .map(|entry_path| entry_path.to_string_lossy().to_string())
        })
        .collect();

    Ok((script, extra_sources))
}

fn is_test_source(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            stem.ends_with("Test") || stem.ends_with("Tests") || stem.ends_with("IT")
        })
}

fn infer_test_companion_sources(script: &Path) -> Vec<String> {
    let Some(parent) = script.parent() else {
        return Vec::new();
    };
    let Some(stem) = script.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let candidates = [
        stem.strip_suffix("Tests"),
        stem.strip_suffix("Test"),
        stem.strip_suffix("IT"),
    ];
    candidates
        .into_iter()
        .flatten()
        .filter(|name| !name.is_empty())
        .map(|name| parent.join(format!("{name}.java")))
        .filter(|path| path.exists() && path != script)
        .filter_map(|path| {
            path.strip_prefix(parent)
                .ok()
                .map(|path| path.to_string_lossy().to_string())
        })
        .collect()
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
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
            init_script(InitOptions {
                script: cmd.script,
                deps: cmd
                    .deps
                    .iter()
                    .flat_map(|dep| split_directive_words(dep))
                    .collect(),
                java_version: cmd.java_version,
                template: cmd.template,
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
            AliasSubcommand::Add(cmd) => {
                let catalog = alias_add(
                    AliasAddOptions {
                        script_ref: cmd.script,
                        name: cmd.name,
                        description: cmd.description,
                        arguments: cmd.args,
                        deps: split_cli_words(&cmd.deps),
                        repos: split_cli_words(&cmd.repos),
                        sources: split_cli_words(&cmd.sources),
                        files: split_cli_words(&cmd.files),
                        classpaths: cmd.classpath,
                        javac_options: cmd.javac_options,
                        runtime_options: cmd.runtime_options,
                        java_agents: split_cli_key_values(&cmd.java_agents),
                        docs: split_cli_key_values(&cmd.docs),
                        java_version: cmd.java_version,
                        main_class: cmd.main_class,
                        force: cmd.force,
                        catalog_file: cmd.catalog.file,
                        global: cmd.catalog.global,
                    },
                    &std::env::current_dir()?,
                )?;
                println!("Alias added to {}", catalog.display());
                0
            }
            AliasSubcommand::Remove(cmd) => {
                let removed = alias_remove(
                    AliasRemoveOptions {
                        name: cmd.name.clone(),
                        catalog_file: cmd.catalog.file,
                        global: cmd.catalog.global,
                    },
                    &std::env::current_dir()?,
                )?;
                if removed {
                    println!("Alias removed: {}", cmd.name);
                } else {
                    println!("Alias '{}' not found.", cmd.name);
                }
                0
            }
            AliasSubcommand::List(cmd) => {
                print_aliases(cmd.json)?;
                0
            }
        },
        Some(Commands::Catalog(cmd)) => match cmd.command {
            CatalogSubcommand::Add(cmd) => {
                let catalog = catalog_add(
                    CatalogAddOptions {
                        name: cmd.name,
                        catalog_ref: cmd.catalog_ref,
                        description: cmd.description,
                        import_items: cmd.import_items,
                        force: cmd.force,
                        catalog_file: cmd.catalog.file,
                        global: cmd.catalog.global,
                    },
                    &std::env::current_dir()?,
                )?;
                println!("Catalog added to {}", catalog.display());
                0
            }
            CatalogSubcommand::List(cmd) => {
                print_catalogs(cmd.json)?;
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
            ExportSubcommand::Native(cmd) => {
                let output =
                    export_native(apply_alias_to_native_export(native_export_options(cmd))?)?;
                println!("Exported to {}", output.display());
                0
            }
        },
        Some(Commands::Template(cmd)) => match cmd.command {
            TemplateSubcommand::List(cmd) => {
                let templates = catalog_templates(&std::env::current_dir()?)?;
                if cmd.json {
                    let payload = templates
                        .iter()
                        .map(|template| {
                            serde_json::json!({
                                "name": template.name,
                                "description": template.description,
                            })
                        })
                        .collect::<Vec<_>>();
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    for template in templates {
                        let description = template.description.unwrap_or_default();
                        println!("{}\t{}", template.name, description);
                    }
                }
                0
            }
        },
        Some(Commands::Resolve(cmd)) => {
            let cache_dir = match cmd.cache_dir {
                Some(path) => path,
                None => default_cache_dir()?.join("deps"),
            };
            let repos = juvx::maven_repositories(&cmd.repos);
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
            let repos = juvx::maven_repositories(&cmd.repos);
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
        Some(Commands::Check(cmd)) => run_check(cmd)?,
        Some(Commands::Test(cmd)) => run_tests(cmd)?,
        Some(Commands::Fmt(cmd)) => run_fmt(cmd)?,
        Some(Commands::Juvx(cmd)) => run_juvx(cmd)?,
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

#[cfg(test)]
mod test_command_unit_tests {
    use super::*;

    #[test]
    fn xml_attr_preserves_quoted_values_with_spaces() {
        let attrs = r#"classname="ExampleTest" name="[1] display name with spaces" time="0.001""#;
        assert_eq!(
            xml_attr(attrs, "name").as_deref(),
            Some("[1] display name with spaces")
        );
        assert_eq!(xml_attr(attrs, "classname").as_deref(), Some("ExampleTest"));
    }

    #[test]
    fn junit_default_version_uses_resolver_latest_metadata() {
        let (repo, handle) = metadata_repo(
            "org/junit/platform/junit-platform-console-standalone/maven-metadata.xml",
            "6.2.0",
        );
        let version = latest_tool_version(
            "org.junit.platform",
            "junit-platform-console-standalone",
            &[repo],
        )
        .unwrap();
        handle.join().unwrap();
        assert_eq!(version, "6.2.0");
    }

    #[test]
    fn palantir_default_version_uses_resolver_latest_metadata() {
        let (repo, handle) = metadata_repo(
            "com/palantir/javaformat/palantir-java-format/maven-metadata.xml",
            "2.92.0",
        );
        let version =
            latest_tool_version("com.palantir.javaformat", "palantir-java-format", &[repo])
                .unwrap();
        handle.join().unwrap();
        assert_eq!(version, "2.92.0");
    }

    #[test]
    fn tool_latest_version_is_cached_temporarily() {
        let tmp = tempfile::tempdir().unwrap();
        let (repo, handle) = metadata_repo(
            "org/junit/platform/junit-platform-console-standalone/maven-metadata.xml",
            "6.3.0",
        );
        let first = latest_cached_tool_version(
            Some(tmp.path()),
            "org.junit.platform",
            "junit-platform-console-standalone",
            std::slice::from_ref(&repo),
        )
        .unwrap();
        handle.join().unwrap();

        let second = latest_cached_tool_version(
            Some(tmp.path()),
            "org.junit.platform",
            "junit-platform-console-standalone",
            &[juv::resolver::Repository {
                id: "offline".to_string(),
                url: "http://127.0.0.1:9".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(first, "6.3.0");
        assert_eq!(second, "6.3.0");
    }

    fn metadata_repo(
        expected_path: &'static str,
        release: &'static str,
    ) -> (juv::resolver::Repository, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let read = std::io::Read::read(&mut stream, &mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(
                request.starts_with(&format!("GET /{expected_path} ")),
                "{request}"
            );
            let body = format!(
                r#"<metadata><versioning><latest>{release}</latest><release>{release}</release><versions><version>{release}</version></versions></versioning></metadata>"#
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/xml\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
        });
        (
            juv::resolver::Repository {
                id: "test".to_string(),
                url: format!("http://{addr}"),
            },
            handle,
        )
    }
}
