use anyhow::{Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{SystemTime, UNIX_EPOCH},
};

use jbx::{
    alias_add, alias_remove, app_bin_dir, app_install, app_list, app_uninstall, build_java,
    cache_entries, catalog_add, catalog_aliases, catalog_refs, catalog_templates, clear_cache,
    default_cache_dir, export_jar, export_native, init_script, maven_tool, resolve_catalog_alias,
    run_java, split_directive_words, trust_add, trust_clear, trust_entries, trust_remove,
    AliasAddOptions, AliasRemoveOptions, AppInstallOptions, BuildOptions, CatalogAddOptions,
    ExportKind, ExportOptions, InitOptions, KeyValue, NativeExportOptions, RunOptions,
};

#[derive(Parser, Debug)]
#[command(
    name = "jbx",
    version,
    about = "jbx: one-stop Java toolbox for scripts, tools, and agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Additional repository for Maven executable shorthand (id=url format or bare URL).
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Override dependency cache directory for Maven executable shorthand.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Main class for Maven executable shorthand instead of java -jar.
    #[arg(long = "main")]
    main_class: Option<String>,

    /// Script to run, or Maven coordinates to launch as a Java tool.
    script: Option<PathBuf>,

    /// Arguments passed to the script/tool when no subcommand is given.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compile and run a Java source file.
    Run(RunCommand),
    /// Compile and store script in the cache without running it.
    Build(BuildCommand),
    /// Prepare Maven Central publishing artifacts.
    Publish(PublishCommand),
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
struct PublishCommand {
    /// Java source file to publish. Defaults to jbx.json main when --file is used.
    script: Option<PathBuf>,

    /// jbx descriptor file. Defaults to ./jbx.json when present.
    #[arg(long = "file")]
    file: Option<PathBuf>,

    /// Override version from jbx.json or //GAV.
    #[arg(long = "version")]
    version: Option<String>,

    /// Output Maven Central bundle ZIP path.
    #[arg(long = "output", short = 'o')]
    output: Option<PathBuf>,

    /// Working directory for staged publish artifacts.
    #[arg(long = "target-dir")]
    target_dir: Option<PathBuf>,

    /// Override package used when staging default-package sources.
    #[arg(long = "package")]
    package_name: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,

    /// Prepare and verify artifacts without uploading.
    #[arg(long = "dry-run", conflicts_with = "publish")]
    dry_run: bool,

    /// Allow unsigned dry-run bundles for local inspection.
    #[arg(long = "skip-signing")]
    skip_signing: bool,

    /// GPG key ID/email to use for detached ASCII signatures.
    #[arg(long = "gpg-key")]
    gpg_key: Option<String>,

    /// Upload to Maven Central and publish after validation.
    #[arg(long = "publish")]
    publish: bool,

    /// Maven Central Portal publishing type for the upload.
    #[arg(long = "publishing-type", default_value = "automatic")]
    publishing_type: CentralPublishingType,

    /// Base URL for Maven Central Portal API.
    #[arg(long = "central-url", hide = true)]
    central_url: Option<String>,

    /// Do not poll Central after uploading the deployment bundle.
    #[arg(long = "no-wait")]
    no_wait: bool,

    /// Seconds to wait between Central deployment status checks.
    #[arg(long = "poll-interval", default_value_t = 5, hide = true)]
    poll_interval: u64,

    /// Maximum seconds to wait for Maven Central publication before exiting.
    #[arg(long = "max-wait-seconds", default_value_t = 600)]
    max_wait_seconds: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CentralPublishingType {
    Automatic,
    UserManaged,
}

impl CentralPublishingType {
    fn as_query_value(self) -> &'static str {
        match self {
            CentralPublishingType::Automatic => "AUTOMATIC",
            CentralPublishingType::UserManaged => "USER_MANAGED",
        }
    }
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
    /// Clear the jbx cache directory.
    Clear(CacheClearCommand),
    /// Print the effective jbx cache directory.
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
    /// Print the effective jbx cache directory.
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

fn key_values_json(values: &[jbx::KeyValue]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|kv| serde_json::json!({ "key": kv.key, "value": kv.value }))
            .collect(),
    )
}

fn docs_json(values: &[jbx::KeyValue]) -> serde_json::Value {
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

fn parsed_directives(script: &PathBuf) -> Result<jbx::Directives> {
    let source = fs::read_to_string(script)?;
    Ok(jbx::parse_directives(&source))
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

fn alias_for_script(script: &Path) -> Result<Option<jbx::CatalogAlias>> {
    let name = script.to_string_lossy().to_string();
    if script.exists() || name.starts_with("http://") || name.starts_with("https://") {
        return Ok(None);
    }
    resolve_catalog_alias(&name, &std::env::current_dir()?)
}

#[allow(clippy::too_many_arguments)]
fn merge_alias_common(
    alias: &jbx::CatalogAlias,
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

fn tools_payload(script: &std::path::Path, output: &jbx::BuildOutput) -> serde_json::Value {
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
            &[jbx::resolver::Repository::central()],
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
    let repos = vec![jbx::resolver::Repository::central()];
    let classpath = jbx::resolver::resolve_classpath(&[coordinate], &repos, &cache)?;
    let java = jbx::jdk::java_bin_path(&jbx::jdk::resolve_jdk(&None, true)?);
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

#[derive(Debug, Clone)]
struct PublishCoordinates {
    group: String,
    id: String,
    version: String,
}

#[derive(Debug, Clone)]
struct PublishLicense {
    name: String,
    url: String,
}

#[derive(Debug, Clone)]
struct PublishDeveloper {
    name: String,
    email: Option<String>,
    organization: Option<String>,
    organization_url: Option<String>,
}

#[derive(Debug, Clone)]
struct PublishScm {
    connection: String,
    developer_connection: Option<String>,
    url: String,
}

#[derive(Debug, Clone)]
struct PublishDescriptor {
    script: PathBuf,
    descriptor_dir: PathBuf,
    coordinates: PublishCoordinates,
    package_name: Option<String>,
    name: Option<String>,
    description: Option<String>,
    url: Option<String>,
    licenses: Vec<PublishLicense>,
    developers: Vec<PublishDeveloper>,
    scm: Option<PublishScm>,
    java_version: Option<String>,
    deps: Vec<String>,
    sources: Vec<String>,
    auto_discover_sources: bool,
    repos: Vec<String>,
}

fn run_publish(cmd: PublishCommand) -> Result<i32> {
    if cmd.publish && cmd.dry_run {
        anyhow::bail!("--dry-run and --publish are mutually exclusive; dry-run never uploads");
    }
    if !cmd.publish && !cmd.dry_run {
        anyhow::bail!(
            "publish requires --dry-run for local inspection or --publish for Maven Central upload"
        );
    }
    if cmd.publish && cmd.skip_signing {
        anyhow::bail!("--publish requires signed artifacts; remove --skip-signing or use --dry-run for local inspection");
    }
    let descriptor = load_publish_descriptor(&cmd)?;
    let bundle = prepare_publish_bundle(&descriptor, &cmd)?;
    if !cmd.publish {
        println!(
            "prepared Maven Central dry run bundle for {}:{}:{} at {}",
            descriptor.coordinates.group,
            descriptor.coordinates.id,
            descriptor.coordinates.version,
            bundle.display()
        );
        return Ok(0);
    }
    let client = CentralClient::from_command(&cmd)?;
    let deployment_name = format!(
        "{}-{}",
        descriptor.coordinates.id, descriptor.coordinates.version
    );
    let deployment_id = client.upload_bundle(&bundle, &deployment_name, cmd.publishing_type)?;
    println!(
        "uploaded Maven Central deployment {deployment_id} for {}:{}:{}",
        descriptor.coordinates.group, descriptor.coordinates.id, descriptor.coordinates.version
    );
    if cmd.no_wait {
        println!(
            "deployment status polling skipped; check Maven Central Portal for {deployment_id}"
        );
        return Ok(0);
    }
    client.wait_for_publication(
        &deployment_id,
        cmd.publishing_type,
        cmd.poll_interval,
        cmd.max_wait_seconds,
    )?;
    Ok(0)
}

struct CentralClient {
    base_url: String,
    authorization: String,
}

impl CentralClient {
    fn from_command(cmd: &PublishCommand) -> Result<Self> {
        let base_url = cmd
            .central_url
            .clone()
            .or_else(|| std::env::var("CENTRAL_PORTAL_URL").ok())
            .unwrap_or_else(|| "https://central.sonatype.com".to_string());
        let token = central_bearer_token(cmd)?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            authorization: format!("Bearer {token}"),
        })
    }

    fn upload_bundle(
        &self,
        bundle: &Path,
        deployment_name: &str,
        publishing_type: CentralPublishingType,
    ) -> Result<String> {
        let filename = bundle
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("central-bundle.zip");
        let boundary = multipart_boundary();
        let body = central_multipart_body(&boundary, filename, &fs::read(bundle)?)?;
        let url = format!(
            "{}/api/v1/publisher/upload?name={}&publishingType={}",
            self.base_url,
            url_encode(deployment_name),
            publishing_type.as_query_value()
        );
        let response = ureq::post(&url)
            .set("Authorization", &self.authorization)
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .send_bytes(&body);
        let text = central_response_text(response, "upload deployment bundle")?;
        let deployment_id = text.trim();
        if deployment_id.is_empty() {
            anyhow::bail!("Maven Central upload succeeded but returned an empty deployment id");
        }
        Ok(deployment_id.to_string())
    }

    fn deployment_status(&self, deployment_id: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{}/api/v1/publisher/status?id={}",
            self.base_url,
            url_encode(deployment_id)
        );
        let text = central_response_text(
            ureq::post(&url)
                .set("Authorization", &self.authorization)
                .call(),
            "read deployment status",
        )?;
        serde_json::from_str(&text)
            .with_context(|| format!("invalid Maven Central status response: {text}"))
    }

    fn publish_deployment(&self, deployment_id: &str) -> Result<()> {
        let url = format!(
            "{}/api/v1/publisher/deployment/{}",
            self.base_url,
            url_encode(deployment_id)
        );
        central_response_text(
            ureq::post(&url)
                .set("Authorization", &self.authorization)
                .call(),
            "publish validated deployment",
        )?;
        Ok(())
    }

    fn wait_for_publication(
        &self,
        deployment_id: &str,
        publishing_type: CentralPublishingType,
        poll_interval: u64,
        max_wait_seconds: u64,
    ) -> Result<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(max_wait_seconds);
        let mut manual_publish_started = publishing_type == CentralPublishingType::Automatic;
        loop {
            let status = self.deployment_status(deployment_id)?;
            let state = status
                .get("deploymentState")
                .and_then(|value| value.as_str())
                .unwrap_or("UNKNOWN");
            println!("Maven Central deployment {deployment_id}: {state}");
            match state {
                "PUBLISHED" => {
                    if let Some(purls) = status.get("purls").and_then(|value| value.as_array()) {
                        for purl in purls.iter().filter_map(|value| value.as_str()) {
                            println!("published {purl}");
                        }
                    }
                    return Ok(());
                }
                "FAILED" => {
                    anyhow::bail!(
                        "Maven Central deployment {deployment_id} failed: {}",
                        status
                    );
                }
                "VALIDATED" if !manual_publish_started => {
                    self.publish_deployment(deployment_id)?;
                    manual_publish_started = true;
                }
                "PENDING" | "VALIDATING" | "VALIDATED" | "PUBLISHING" => {}
                _ => anyhow::bail!(
                    "unknown Maven Central deployment state for {deployment_id}: {state}"
                ),
            }
            if std::time::Instant::now() >= deadline {
                anyhow::bail!("timed out waiting for Maven Central deployment {deployment_id} after {max_wait_seconds}s");
            }
            if poll_interval > 0 {
                std::thread::sleep(std::time::Duration::from_secs(poll_interval));
            }
        }
    }
}

fn central_bearer_token(_cmd: &PublishCommand) -> Result<String> {
    if let Some(token) = first_env(&[
        "CENTRAL_PORTAL_TOKEN",
        "CENTRAL_TOKEN",
        "MAVEN_CENTRAL_TOKEN",
        "SONATYPE_TOKEN",
    ]) {
        return Ok(token);
    }
    let username = first_env(&[
        "CENTRAL_TOKEN_USERNAME",
        "CENTRAL_PORTAL_USERNAME",
        "CENTRAL_USERNAME",
        "MAVEN_CENTRAL_USERNAME",
        "SONATYPE_USERNAME",
    ]);
    let password = first_env(&[
        "CENTRAL_TOKEN_PASSWORD",
        "CENTRAL_PORTAL_PASSWORD",
        "CENTRAL_PASSWORD",
        "MAVEN_CENTRAL_PASSWORD",
        "SONATYPE_PASSWORD",
    ]);
    match (username, password) {
        (Some(username), Some(password)) => Ok(base64::engine::general_purpose::STANDARD
            .encode(format!("{username}:{password}"))),
        _ => anyhow::bail!(
            "Maven Central publishing requires CENTRAL_PORTAL_TOKEN or CENTRAL_TOKEN_USERNAME/CENTRAL_TOKEN_PASSWORD"
        ),
    }
}

fn first_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| std::env::var(name).ok())
}

fn central_response_text(
    response: std::result::Result<ureq::Response, ureq::Error>,
    operation: &str,
) -> Result<String> {
    match response {
        Ok(response) => response
            .into_string()
            .with_context(|| format!("failed to read Maven Central response for {operation}")),
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            anyhow::bail!("Maven Central {operation} failed with HTTP {code}: {body}");
        }
        Err(err) => Err(anyhow::anyhow!("Maven Central {operation} failed: {err}")),
    }
}

fn multipart_boundary() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("jbx-central-{}-{nanos}", std::process::id())
}

fn central_multipart_body(boundary: &str, filename: &str, bundle: &[u8]) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    write!(body, "--{boundary}\r\n")?;
    write!(
        body,
        "Content-Disposition: form-data; name=\"bundle\"; filename=\"{}\"\r\n",
        filename.replace('\"', "%22")
    )?;
    write!(body, "Content-Type: application/octet-stream\r\n\r\n")?;
    body.extend_from_slice(bundle);
    write!(body, "\r\n--{boundary}--\r\n")?;
    Ok(body)
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn load_publish_descriptor(cmd: &PublishCommand) -> Result<PublishDescriptor> {
    let descriptor_path = match &cmd.file {
        Some(path) => Some(path.clone()),
        None => {
            let candidate = PathBuf::from("jbx.json");
            candidate.exists().then_some(candidate)
        }
    };

    let mut script = cmd.script.clone();
    let mut descriptor_dir = PathBuf::from(".");
    let mut coordinates = None;
    let mut package_name = None;
    let mut name = None;
    let mut description = None;
    let mut url = None;
    let mut licenses = Vec::new();
    let mut developers = Vec::new();
    let mut scm = None;
    let mut java_version = None;
    let mut deps = Vec::new();
    let mut sources = Vec::new();
    let mut descriptor_sources_present = false;
    let mut repos = Vec::new();

    if let Some(path) = descriptor_path {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read descriptor {}", path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse descriptor {}", path.display()))?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        descriptor_dir = base_dir.to_path_buf();
        if script.is_none() {
            if let Some(main) = json.get("main").and_then(|value| value.as_str()) {
                script = Some(resolve_publish_main_path(base_dir, main));
            }
        }
        if json.get("group").is_some() || json.get("id").is_some() || json.get("version").is_some()
        {
            coordinates = Some(parse_descriptor_coordinates(&json)?);
        }
        package_name = json
            .get("package")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        name = json
            .get("name")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        description = json
            .get("description")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        url = json
            .get("url")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        licenses = parse_descriptor_licenses(&json)?;
        developers = parse_descriptor_developers(&json)?;
        scm = parse_descriptor_scm(&json)?;
        java_version = json
            .get("java")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        deps = string_array(&json, "dependencies")?;
        descriptor_sources_present = json.get("sources").is_some();
        sources = string_array(&json, "sources")?;
        repos = string_array(&json, "repositories")?;
    }

    let script =
        script.ok_or_else(|| anyhow::anyhow!("publish requires a script or jbx.json main"))?;
    if !script.exists() {
        anyhow::bail!(
            "publish main source not found: {}{}",
            script.display(),
            publish_main_hint(&script)
        );
    }
    let directives = parsed_directives(&script)
        .with_context(|| format!("failed to read publish main source {}", script.display()))?;
    if coordinates.is_none() {
        if let Some(raw) = directives.gav.as_deref() {
            coordinates = Some(parse_gav_directive(raw)?);
        }
    }
    if description.is_none() {
        description = directives.description.clone();
    }
    let github = infer_github_publish_metadata();
    if url.is_none() {
        url = github.as_ref().and_then(|metadata| metadata.url.clone());
    }
    if licenses.is_empty() {
        licenses = github
            .as_ref()
            .and_then(|metadata| metadata.license.clone())
            .into_iter()
            .collect();
    }
    if developers.is_empty() {
        developers = github
            .as_ref()
            .and_then(|metadata| metadata.developer.clone())
            .into_iter()
            .collect();
    }
    if scm.is_none() {
        scm = github.and_then(|metadata| metadata.scm);
    }
    if java_version.is_none() {
        java_version = directives.java_version.clone();
    }
    if deps.is_empty() {
        deps = directives.deps.clone();
    }
    if sources.is_empty() {
        sources = directives.sources.clone();
    }
    if repos.is_empty() {
        repos = directives.repos.clone();
    }
    let mut coordinates = coordinates
        .ok_or_else(|| anyhow::anyhow!("publish requires group, id, and version metadata"))?;
    if let Some(version) = &cmd.version {
        coordinates.version = version.clone();
    }
    if let Some(package_name_override) = &cmd.package_name {
        package_name = Some(package_name_override.clone());
    }
    validate_group(&coordinates.group)?;
    validate_path_safe_coordinate_part(&coordinates.id, "id")?;
    validate_path_safe_coordinate_part(&coordinates.version, "version")?;
    if let Some(package_name) = package_name.as_deref() {
        validate_package_name(package_name)?;
    }
    let name = name.or_else(|| Some(format!("{}:{}", coordinates.group, coordinates.id)));
    if coordinates.version.ends_with("-SNAPSHOT") {
        anyhow::bail!("Maven Central does not accept -SNAPSHOT versions");
    }
    if description.is_none() {
        description = Some(format!("{} published with jbx", coordinates.id));
    }
    require_publish_metadata("url", url.as_deref())?;
    if licenses.is_empty() {
        anyhow::bail!("publish requires at least one license for Maven Central metadata");
    }
    if developers.is_empty() {
        anyhow::bail!("publish requires at least one developer for Maven Central metadata");
    }
    if scm.is_none() {
        anyhow::bail!("publish requires scm metadata for Maven Central");
    }
    Ok(PublishDescriptor {
        script,
        descriptor_dir,
        coordinates,
        package_name,
        name,
        description,
        url,
        licenses,
        developers,
        scm,
        java_version,
        deps,
        sources,
        auto_discover_sources: !descriptor_sources_present,
        repos,
    })
}

fn resolve_publish_main_path(base_dir: &Path, main: &str) -> PathBuf {
    let raw = Path::new(main);
    let exact = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    if exact.exists() || raw.extension().is_some() {
        return exact;
    }
    for extension in ["java", "jsh", "jav"] {
        let candidate = exact.with_extension(extension);
        if candidate.exists() {
            return candidate;
        }
    }
    exact
}

fn publish_main_hint(path: &Path) -> String {
    if path.extension().is_some() {
        String::new()
    } else {
        format!(
            " (also checked {}.java, {}.jsh, and {}.jav)",
            path.display(),
            path.display(),
            path.display()
        )
    }
}

fn parse_descriptor_coordinates(json: &serde_json::Value) -> Result<PublishCoordinates> {
    let field = |name: &str| -> Result<String> {
        json.get(name)
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow::anyhow!("{name} is required"))
    };
    Ok(PublishCoordinates {
        group: field("group")?,
        id: field("id")?,
        version: field("version")?,
    })
}

fn parse_gav_directive(raw: &str) -> Result<PublishCoordinates> {
    let parts = raw.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        anyhow::bail!("//GAV must have group:artifact:version");
    }
    Ok(PublishCoordinates {
        group: parts[0].to_string(),
        id: parts[1].to_string(),
        version: parts[2].to_string(),
    })
}

fn string_array(json: &serde_json::Value, name: &str) -> Result<Vec<String>> {
    let Some(value) = json.get(name) else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{name} must be an array of strings"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow::anyhow!("{name} must be an array of strings"))
        })
        .collect()
}

fn required_object_string(object: &serde_json::Value, name: &str) -> Result<String> {
    object
        .get(name)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

fn optional_object_string(object: &serde_json::Value, name: &str) -> Option<String> {
    object
        .get(name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_descriptor_licenses(json: &serde_json::Value) -> Result<Vec<PublishLicense>> {
    let Some(value) = json.get("licenses") else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("licenses must be an array of objects"))?;
    array
        .iter()
        .map(|license| {
            Ok(PublishLicense {
                name: required_object_string(license, "name")?,
                url: required_object_string(license, "url")?,
            })
        })
        .collect()
}

fn parse_descriptor_developers(json: &serde_json::Value) -> Result<Vec<PublishDeveloper>> {
    let Some(value) = json.get("developers") else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("developers must be an array of objects"))?;
    array
        .iter()
        .map(|developer| {
            Ok(PublishDeveloper {
                name: required_object_string(developer, "name")?,
                email: optional_object_string(developer, "email"),
                organization: optional_object_string(developer, "organization"),
                organization_url: optional_object_string(developer, "organizationUrl"),
            })
        })
        .collect()
}

fn parse_descriptor_scm(json: &serde_json::Value) -> Result<Option<PublishScm>> {
    let Some(value) = json.get("scm") else {
        return Ok(None);
    };
    Ok(Some(PublishScm {
        connection: required_object_string(value, "connection")?,
        developer_connection: optional_object_string(value, "developerConnection"),
        url: required_object_string(value, "url")?,
    }))
}

fn require_publish_metadata(name: &str, value: Option<&str>) -> Result<()> {
    if value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        anyhow::bail!("publish requires {name} for Maven Central metadata");
    }
    Ok(())
}

#[derive(Debug, Default)]
struct InferredPublishMetadata {
    url: Option<String>,
    license: Option<PublishLicense>,
    developer: Option<PublishDeveloper>,
    scm: Option<PublishScm>,
}

fn infer_github_publish_metadata() -> Option<InferredPublishMetadata> {
    let remote = ProcessCommand::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())?;
    let repo = github_repo_slug(remote.trim())?;
    let mut metadata = InferredPublishMetadata {
        url: Some(format!("https://github.com/{repo}")),
        scm: Some(PublishScm {
            connection: format!("scm:git:https://github.com/{repo}.git"),
            developer_connection: Some(format!("scm:git:ssh://git@github.com/{repo}.git")),
            url: format!("https://github.com/{repo}"),
        }),
        ..InferredPublishMetadata::default()
    };
    if let Some(json) = gh_repo_view(&repo) {
        metadata.url = json
            .get("url")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .or(metadata.url);
        metadata.license = json.get("licenseInfo").and_then(github_license_from_json);
        metadata.developer = json.get("owner").and_then(github_developer_from_owner);
    }
    Some(metadata)
}

fn github_repo_slug(remote: &str) -> Option<String> {
    let without_suffix = remote.strip_suffix(".git").unwrap_or(remote);
    if let Some(rest) = without_suffix.strip_prefix("git@github.com:") {
        return Some(rest.to_string());
    }
    without_suffix
        .strip_prefix("https://github.com/")
        .map(ToOwned::to_owned)
}

fn gh_repo_view(repo: &str) -> Option<serde_json::Value> {
    let output = ProcessCommand::new("gh")
        .args(["repo", "view", repo, "--json", "url,licenseInfo,owner"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn github_license_from_json(value: &serde_json::Value) -> Option<PublishLicense> {
    let name = value
        .get("name")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())?;
    let spdx = value
        .get("spdxId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    Some(PublishLicense {
        name: name.to_string(),
        url: spdx
            .map(|spdx| format!("https://spdx.org/licenses/{spdx}.html"))
            .unwrap_or_else(|| "https://opensource.org/licenses".to_string()),
    })
}

fn github_developer_from_owner(value: &serde_json::Value) -> Option<PublishDeveloper> {
    let login = value
        .get("login")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())?;
    Some(PublishDeveloper {
        name: login.to_string(),
        email: None,
        organization: None,
        organization_url: Some(format!("https://github.com/{login}")),
    })
}

fn validate_group(value: &str) -> Result<()> {
    validate_coordinate_part(value, "group")?;
    if value
        .split('.')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        anyhow::bail!("invalid group: {value}");
    }
    Ok(())
}

fn validate_path_safe_coordinate_part(value: &str, name: &str) -> Result<()> {
    validate_coordinate_part(value, name)?;
    if value == "." || value == ".." {
        anyhow::bail!("invalid {name}: {value}");
    }
    Ok(())
}

fn validate_coordinate_part(value: &str, name: &str) -> Result<()> {
    if value.is_empty()
        || value
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
    {
        anyhow::bail!("invalid {name}: {value}");
    }
    Ok(())
}

fn validate_package_name(value: &str) -> Result<()> {
    if value
        .split('.')
        .any(|part| part.is_empty() || !is_java_identifier(part))
    {
        anyhow::bail!("invalid package name: {value}");
    }
    Ok(())
}

fn is_java_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first == '$' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

fn prepare_publish_bundle(descriptor: &PublishDescriptor, cmd: &PublishCommand) -> Result<PathBuf> {
    let target_dir = cmd
        .target_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/jbx-publish"));
    let staging_dir = target_dir.join("staging");
    let repo_dir = target_dir.join("repository");
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir)?;
    }
    if repo_dir.exists() {
        fs::remove_dir_all(&repo_dir)?;
    }
    fs::create_dir_all(&staging_dir)?;
    fs::create_dir_all(&repo_dir)?;

    let staged = stage_publish_sources(descriptor, &staging_dir)?;
    let build = build_java(BuildOptions {
        script: staged.script.clone(),
        extra_deps: descriptor.deps.clone(),
        extra_repos: descriptor.repos.clone(),
        extra_sources: staged.extra_sources.clone(),
        extra_files: Vec::new(),
        classpath: Vec::new(),
        javac_options: Vec::new(),
        runtime_options: Vec::new(),
        java_agents: Vec::new(),
        java_version: descriptor.java_version.clone(),
        main_class: None,
        cache_dir: cmd.cache_dir.clone(),
        trust_remote: false,
    })?;

    let base_rel = PathBuf::from(descriptor.coordinates.group.replace('.', "/"))
        .join(&descriptor.coordinates.id)
        .join(&descriptor.coordinates.version);
    let artifact_dir = repo_dir.join(&base_rel);
    fs::create_dir_all(&artifact_dir)?;
    let prefix = format!(
        "{}-{}",
        descriptor.coordinates.id, descriptor.coordinates.version
    );
    let jar = artifact_dir.join(format!("{prefix}.jar"));
    write_directory_jar(&build.classes_dir, &jar)?;
    let sources_jar = artifact_dir.join(format!("{prefix}-sources.jar"));
    write_directory_jar(&staging_dir, &sources_jar)?;
    let javadoc_jar = artifact_dir.join(format!("{prefix}-javadoc.jar"));
    write_javadoc_jar(
        descriptor,
        &staged.all_sources,
        &build.classpath,
        &target_dir,
        &javadoc_jar,
    )?;
    let pom = artifact_dir.join(format!("{prefix}.pom"));
    fs::write(&pom, render_pom(descriptor)?)?;
    for file in [&jar, &sources_jar, &javadoc_jar, &pom] {
        write_checksums(file)?;
        if !cmd.skip_signing {
            write_gpg_signature(file, cmd.gpg_key.as_deref())?;
        }
    }
    let bundle = cmd
        .output
        .clone()
        .unwrap_or_else(|| target_dir.join(format!("{prefix}-central-bundle.zip")));
    if let Some(parent) = bundle.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    zip_directory(&repo_dir, &bundle)?;
    Ok(bundle)
}

struct StagedPublishSources {
    script: PathBuf,
    extra_sources: Vec<String>,
    all_sources: Vec<PathBuf>,
}

fn stage_publish_sources(
    descriptor: &PublishDescriptor,
    staging_dir: &Path,
) -> Result<StagedPublishSources> {
    let script = stage_publish_source_file(&descriptor.script, descriptor, staging_dir)?;
    let mut extra_sources = Vec::new();
    let mut all_sources = vec![script.clone()];
    let mut source_paths = descriptor
        .sources
        .iter()
        .map(|source| resolve_descriptor_relative_path(&descriptor.descriptor_dir, source))
        .collect::<Vec<_>>();
    if descriptor.auto_discover_sources {
        source_paths.extend(discover_publish_source_files(
            &descriptor.descriptor_dir,
            &descriptor.script,
        )?);
    }
    source_paths.sort();
    source_paths.dedup();
    let script_canonical = descriptor.script.canonicalize().ok();
    for source_path in source_paths {
        if script_canonical
            .as_ref()
            .is_some_and(|script| source_path.canonicalize().ok().as_ref() == Some(script))
        {
            continue;
        }
        let staged = stage_publish_source_file(&source_path, descriptor, staging_dir)
            .with_context(|| format!("failed to stage source {}", source_path.display()))?;
        let absolute = staged.to_string_lossy().to_string();
        extra_sources.push(absolute);
        all_sources.push(staged);
    }
    Ok(StagedPublishSources {
        script,
        extra_sources,
        all_sources,
    })
}

fn discover_publish_source_files(base_dir: &Path, main_source: &Path) -> Result<Vec<PathBuf>> {
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }
    let main_canonical = main_source.canonicalize().ok();
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(base_dir)
        .into_iter()
        .filter_entry(|entry| {
            !entry.file_type().is_dir() || !is_ignored_publish_source_dir(entry.path(), base_dir)
        })
    {
        let entry = entry.with_context(|| format!("failed to scan {}", base_dir.display()))?;
        let path = entry.path();
        if !entry.file_type().is_file() || !is_java_file(path) {
            continue;
        }
        if main_canonical
            .as_ref()
            .is_some_and(|main| path.canonicalize().ok().as_ref() == Some(main))
        {
            continue;
        }
        files.push(path.to_path_buf());
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn is_ignored_publish_source_dir(path: &Path, base_dir: &Path) -> bool {
    if path == base_dir {
        return false;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with('.') || matches!(name, "target" | "build" | "out" | "classes")
        })
}

fn resolve_descriptor_relative_path(base_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn stage_publish_source_file(
    source_path: &Path,
    descriptor: &PublishDescriptor,
    staging_dir: &Path,
) -> Result<PathBuf> {
    let source = fs::read_to_string(source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("invalid source path: {}", source_path.display()))?;
    if let Some(package_name) = package_name_in_source(&source) {
        let package_dir = staging_dir.join(package_name.replace('.', "/"));
        fs::create_dir_all(&package_dir)?;
        let target = package_dir.join(file_name);
        fs::write(&target, source)?;
        return Ok(target);
    }
    let package_name = descriptor.package_name.clone().unwrap_or_else(|| {
        format!(
            "{}.{}",
            descriptor.coordinates.group,
            descriptor.coordinates.id.replace('-', "")
        )
    });
    if looks_like_compact_source(&source) {
        let target = staging_dir.join(file_name);
        fs::write(&target, source)?;
        return Ok(target);
    }
    validate_package_name(&package_name)?;
    let package_dir = staging_dir.join(package_name.replace('.', "/"));
    fs::create_dir_all(&package_dir)?;
    let target = package_dir.join(file_name);
    fs::write(&target, format!("package {package_name};\n\n{source}"))?;
    Ok(target)
}

fn package_name_in_source(source: &str) -> Option<String> {
    let package_re = regex::Regex::new(
        r"(?m)^\s*package\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*;",
    )
    .expect("valid package regex");
    package_re
        .captures(source)
        .and_then(|captures| captures.get(1))
        .map(|package| package.as_str().to_string())
}

fn looks_like_compact_source(source: &str) -> bool {
    let has_type_declaration = source.contains(" class ")
        || source.contains(" public class ")
        || source.contains(" record ")
        || source.contains(" interface ")
        || source.contains(" enum ");
    !has_type_declaration && source.contains("void main(")
}

fn render_pom(descriptor: &PublishDescriptor) -> Result<String> {
    let name = descriptor
        .name
        .as_deref()
        .unwrap_or(&descriptor.coordinates.id);
    let description = descriptor
        .description
        .as_deref()
        .expect("publish metadata was validated");
    let url = descriptor
        .url
        .as_deref()
        .expect("publish metadata was validated");
    let dependencies = render_pom_dependencies(&descriptor.deps)?;
    let licenses = render_pom_licenses(&descriptor.licenses);
    let developers = render_pom_developers(&descriptor.developers);
    let scm = render_pom_scm(
        descriptor
            .scm
            .as_ref()
            .expect("publish metadata was validated"),
    );
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>{}</groupId>
  <artifactId>{}</artifactId>
  <version>{}</version>
  <packaging>jar</packaging>
  <name>{}</name>
  <description>{}</description>
  <url>{}</url>{}{}{}{}
</project>
"#,
        xml_escape(&descriptor.coordinates.group),
        xml_escape(&descriptor.coordinates.id),
        xml_escape(&descriptor.coordinates.version),
        xml_escape(name),
        xml_escape(description),
        xml_escape(url),
        licenses,
        developers,
        scm,
        dependencies
    ))
}

fn render_pom_licenses(licenses: &[PublishLicense]) -> String {
    let mut out = String::from("\n  <licenses>");
    for license in licenses {
        out.push_str("\n    <license>");
        out.push_str(&format!(
            "\n      <name>{}</name>",
            xml_escape(&license.name)
        ));
        out.push_str(&format!("\n      <url>{}</url>", xml_escape(&license.url)));
        out.push_str("\n    </license>");
    }
    out.push_str("\n  </licenses>");
    out
}

fn render_pom_developers(developers: &[PublishDeveloper]) -> String {
    let mut out = String::from("\n  <developers>");
    for developer in developers {
        out.push_str("\n    <developer>");
        out.push_str(&format!(
            "\n      <name>{}</name>",
            xml_escape(&developer.name)
        ));
        if let Some(email) = developer.email.as_deref() {
            out.push_str(&format!("\n      <email>{}</email>", xml_escape(email)));
        }
        if let Some(organization) = developer.organization.as_deref() {
            out.push_str(&format!(
                "\n      <organization>{}</organization>",
                xml_escape(organization)
            ));
        }
        if let Some(organization_url) = developer.organization_url.as_deref() {
            out.push_str(&format!(
                "\n      <organizationUrl>{}</organizationUrl>",
                xml_escape(organization_url)
            ));
        }
        out.push_str("\n    </developer>");
    }
    out.push_str("\n  </developers>");
    out
}

fn render_pom_scm(scm: &PublishScm) -> String {
    let mut out = String::from("\n  <scm>");
    out.push_str(&format!(
        "\n    <connection>{}</connection>",
        xml_escape(&scm.connection)
    ));
    if let Some(developer_connection) = scm.developer_connection.as_deref() {
        out.push_str(&format!(
            "\n    <developerConnection>{}</developerConnection>",
            xml_escape(developer_connection)
        ));
    }
    out.push_str(&format!("\n    <url>{}</url>", xml_escape(&scm.url)));
    out.push_str("\n  </scm>");
    out
}

fn render_pom_dependencies(deps: &[String]) -> Result<String> {
    let parsed = deps
        .iter()
        .filter_map(|dep| jbx::resolver::parse_coordinate(dep).ok())
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        return Ok(String::new());
    }
    let mut out = String::from("\n  <dependencies>");
    for dep in parsed {
        out.push_str("\n    <dependency>");
        out.push_str(&format!(
            "\n      <groupId>{}</groupId>",
            xml_escape(&dep.module.org)
        ));
        out.push_str(&format!(
            "\n      <artifactId>{}</artifactId>",
            xml_escape(&dep.module.name)
        ));
        out.push_str(&format!(
            "\n      <version>{}</version>",
            xml_escape(&dep.version)
        ));
        if let Some(classifier) = dep.classifier.as_deref() {
            out.push_str(&format!(
                "\n      <classifier>{}</classifier>",
                xml_escape(classifier)
            ));
        }
        out.push_str("\n    </dependency>");
    }
    out.push_str("\n  </dependencies>");
    Ok(out)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn write_directory_jar(source_dir: &Path, jar: &Path) -> Result<()> {
    let file = fs::File::create(jar)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for entry in walkdir::WalkDir::new(source_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(source_dir)?
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(rel, options)?;
        zip.write_all(&fs::read(entry.path())?)?;
    }
    zip.finish()?;
    Ok(())
}

fn publish_join_classpath(paths: &[PathBuf]) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    paths
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(sep)
}

fn write_javadoc_jar(
    descriptor: &PublishDescriptor,
    sources: &[PathBuf],
    classpath: &[PathBuf],
    target_dir: &Path,
    jar: &Path,
) -> Result<()> {
    let javadoc_dir = target_dir.join("javadoc");
    if javadoc_dir.exists() {
        fs::remove_dir_all(&javadoc_dir)?;
    }
    fs::create_dir_all(&javadoc_dir)?;

    let jdk_root = jbx::jdk::resolve_jdk(&descriptor.java_version, true)?;
    let javadoc = jbx::jdk::javadoc_bin_path(&jdk_root);
    let mut cmd = ProcessCommand::new(&javadoc);
    cmd.arg("-quiet")
        .arg("-d")
        .arg(&javadoc_dir)
        .arg("-sourcepath")
        .arg(target_dir.join("staging"));
    if !classpath.is_empty() {
        cmd.arg("-classpath").arg(publish_join_classpath(classpath));
    }
    cmd.args(sources);
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute {}", javadoc.display()))?;
    if !status.success() {
        anyhow::bail!(
            "javadoc failed with exit code {}",
            status.code().unwrap_or(1)
        );
    }
    write_directory_jar(&javadoc_dir, jar)
}

fn write_checksums(path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    let md5 = {
        use md5::Digest;
        let mut hasher = md5::Md5::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    let sha1 = {
        use sha1::Digest;
        let mut hasher = sha1::Sha1::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    let sha256 = {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    let sha512 = {
        use sha2::Digest;
        let mut hasher = sha2::Sha512::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    fs::write(
        path.with_extension(format!("{}md5", extension_with_dot(path))),
        md5,
    )?;
    fs::write(
        path.with_extension(format!("{}sha1", extension_with_dot(path))),
        sha1,
    )?;
    fs::write(
        path.with_extension(format!("{}sha256", extension_with_dot(path))),
        sha256,
    )?;
    fs::write(
        path.with_extension(format!("{}sha512", extension_with_dot(path))),
        sha512,
    )?;
    Ok(())
}

fn write_gpg_signature(path: &Path, gpg_key: Option<&str>) -> Result<()> {
    let signature = path.with_extension(format!("{}asc", extension_with_dot(path)));
    let mut cmd = ProcessCommand::new("gpg");
    cmd.arg("--batch")
        .arg("--yes")
        .arg("--armor")
        .arg("--detach-sign")
        .arg("--output")
        .arg(&signature);
    if let Some(key) = gpg_key {
        cmd.arg("--local-user").arg(key);
    }
    cmd.arg(path);
    let output = cmd
        .output()
        .with_context(|| "failed to execute gpg for Maven Central signatures")?;
    if !output.status.success() {
        anyhow::bail!(
            "gpg signing failed for {}; configure a signing key, pass --gpg-key, or use --skip-signing for unsigned dry-run inspection: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn extension_with_dot(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!("{ext}."))
        .unwrap_or_default()
}

fn zip_directory(source_dir: &Path, output: &Path) -> Result<()> {
    let file = fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for entry in walkdir::WalkDir::new(source_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(source_dir)?
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(rel, options)?;
        zip.write_all(&fs::read(entry.path())?)?;
    }
    zip.finish()?;
    Ok(())
}

const DEFAULT_JUNIT_PLATFORM_VERSION: &str = "6.1.0";

fn collect_check_directives(files: &[PathBuf]) -> Result<jbx::Directives> {
    let mut directives = jbx::Directives::default();
    for file in files {
        let source = fs::read_to_string(file)
            .with_context(|| format!("failed to read Java source {}", file.display()))?;
        let parsed = jbx::parse_directives(&source);
        directives.deps.extend(parsed.deps);
        directives.repos.extend(parsed.repos);
        directives.javac_options.extend(parsed.javac_options);
        if directives.java_version.is_none() {
            directives.java_version = parsed.java_version;
        }
        directives.enable_preview |= parsed.enable_preview;
    }
    Ok(directives)
}

fn has_source_or_release_option(options: &[String]) -> bool {
    options.iter().any(|option| {
        matches!(
            option.as_str(),
            "--source" | "-source" | "--release" | "-release"
        ) || option.starts_with("--source=")
            || option.starts_with("--release=")
    })
}

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

    let mut directives = collect_check_directives(&files)?;
    directives.deps.extend(split_cli_words(&cmd.deps));
    directives.repos.extend(split_cli_words(&cmd.repos));
    directives.javac_options.extend(cmd.javac_options);
    if cmd.java_version.is_some() {
        directives.java_version = cmd.java_version;
    }

    let jdk_root = jbx::jdk::resolve_jdk(&directives.java_version, true)?;
    let javac = jbx::jdk::javac_bin_path(&jdk_root);
    let java = jbx::jdk::java_bin_path(&jdk_root);
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
                "failed to compile jbx check compiler wrapper with exit code {}",
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

    let dep_coordinates = directives.deps;
    let mut classpath = cmd.classpath;
    if !dep_coordinates.is_empty() {
        let repos = maven_tool::maven_repositories(&directives.repos);
        let cache_dir = cache_root(cmd.cache_dir.as_deref())?.join("deps");
        classpath.extend(jbx::resolver::resolve_classpath(
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
    compiler_options.extend(directives.javac_options);
    if directives.enable_preview {
        if !compiler_options
            .iter()
            .any(|option| option == "--enable-preview")
        {
            compiler_options.push("--enable-preview".to_string());
        }
        if !has_source_or_release_option(&compiler_options) {
            let release_version =
                jbx::jdk::detect_jdk_major_version(&jdk_root).with_context(|| {
                    format!("could not determine JDK version at {}", jdk_root.display())
                })?;
            compiler_options.push("--release".to_string());
            compiler_options.push(release_version.to_string());
        }
    }
    if cmd.warnings_as_errors {
        compiler_options.push("-Werror".to_string());
    }

    let mut wrapper_classpath = vec![wrapper_dir.clone()];
    if !cmd.no_error_prone {
        let repos = maven_tool::maven_repositories(&split_cli_words(&cmd.repos));
        let cache_dir = cache_root(cmd.cache_dir.as_deref())?.join("deps");
        let error_prone_coordinate = format!(
            "{ERROR_PRONE_GROUP_ID}:{ERROR_PRONE_ARTIFACT_ID}:{}",
            cmd.error_prone_version
        );
        let error_prone_cp =
            jbx::resolver::resolve_classpath(&[error_prone_coordinate], &repos, &cache_dir)?;
        wrapper_classpath.extend(error_prone_cp);
        compiler_options.push("-XDcompilePolicy=simple".to_string());
        compiler_options.push("--should-stop=ifError=FLOW".to_string());
        compiler_options.push("-Xplugin:ErrorProne -Xep:DefaultPackage:OFF".to_string());
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
        .with_context(|| format!("invalid jbx check wrapper output: {stdout}"))?;
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
            .context("failed to build jbx check compiler wrapper classpath")?,
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
            &[jbx::resolver::Repository::central()],
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

    let (script, inferred_directory_sources) = expand_test_target(&cmd.script)?;
    let mut extra_sources = split_cli_words(&cmd.sources);
    extra_sources.extend(inferred_directory_sources);
    extra_sources.extend(infer_test_companion_sources(&script));
    dedupe_strings(&mut extra_sources);

    let mut directive_files = vec![script.clone()];
    let base_dir = script.parent().unwrap_or_else(|| Path::new("."));
    directive_files.extend(extra_sources.iter().map(|source| base_dir.join(source)));
    let source_directives = collect_check_directives(&directive_files)?;

    let mut deps = source_directives.deps;
    deps.extend(split_cli_words(&cmd.deps));
    deps.push(launcher_coordinate);
    let mut repos = source_directives.repos;
    repos.extend(split_cli_words(&cmd.repos));
    let mut javac_options = source_directives.javac_options;
    javac_options.extend(cmd.javac_options);
    let java_version = cmd.java_version.or(source_directives.java_version);

    let build = build_java(BuildOptions {
        script,
        extra_deps: deps,
        extra_repos: repos,
        extra_sources,
        extra_files: split_cli_words(&cmd.files),
        classpath: cmd.classpath,
        javac_options,
        runtime_options: Vec::new(),
        java_agents: split_cli_key_values(&cmd.java_agents),
        java_version,
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

    let jdk_root = jbx::jdk::resolve_jdk(&build.directives.java_version, true)?;
    let java = jbx::jdk::java_bin_path(&jdk_root).display().to_string();
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
    Ok(std::env::temp_dir().join(format!("jbx-junit-{}-{nanos}", std::process::id())))
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
    repos: &[jbx::resolver::Repository],
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
    repos: &[jbx::resolver::Repository],
) -> Result<String> {
    jbx::resolver::resolve_latest_version(
        &jbx::resolver::Module {
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

fn should_run_as_maven_tool_shorthand(script: &Path) -> bool {
    if script.exists() {
        return false;
    }
    let value = script.to_string_lossy();
    if value.starts_with("http://") || value.starts_with("https://") {
        return false;
    }
    let parts: Vec<&str> = value.split(':').collect();
    matches!(parts.len(), 2..=4) && parts.iter().all(|part| !part.is_empty())
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
                let directives = jbx::parse_directives(&source);
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
                let main = jbx::parse_directives(&source)
                    .main_class
                    .or_else(|| jbx::infer_main_class_from_source(&cmd.script, &source));
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
                println!("{:#?}", jbx::parse_directives(&source));
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
            let repos = maven_tool::maven_repositories(&cmd.repos);
            if cmd.classpath {
                let paths = jbx::resolver::resolve_classpath(&cmd.coordinates, &repos, &cache_dir)?;
                println!("{}", std::env::join_paths(paths)?.to_string_lossy());
            } else {
                let artifacts = jbx::resolver::resolve(&cmd.coordinates, &repos, &cache_dir)?;
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
            let repos = maven_tool::maven_repositories(&cmd.repos);
            if cmd.deps_only {
                let artifacts = jbx::resolver::resolve(&cmd.coordinates, &repos, &cache_dir)?;
                for artifact in &artifacts {
                    println!("{artifact}");
                }
            } else {
                let paths = jbx::resolver::resolve_classpath(&cmd.coordinates, &repos, &cache_dir)?;
                println!("{}", std::env::join_paths(paths)?.to_string_lossy());
            }
            0
        }
        Some(Commands::Publish(cmd)) => run_publish(cmd)?,
        Some(Commands::Check(cmd)) => run_check(cmd)?,
        Some(Commands::Test(cmd)) => run_tests(cmd)?,
        Some(Commands::Fmt(cmd)) => run_fmt(cmd)?,
        Some(Commands::Jdk(cmd)) => match cmd.command {
            JdkSubcommand::List(_) => {
                let jdks = jbx::jdk::list_jdks()?;
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
                let version = jbx::jdk::parse_java_version_directive(&cmd.version)?;
                let jdk_root = jbx::jdk::install_jdk(version)?;
                println!("JDK {} installed to {}", version, jdk_root.display());
                0
            }
            JdkSubcommand::Home(cmd) => {
                let version = jbx::jdk::parse_java_version_directive(&cmd.version)?;
                let jdk_root = jbx::jdk::find_jdk(version, false)?;
                println!("{}", jdk_root.display());
                0
            }
        },
        None => {
            let Some(script) = cli.script else {
                eprintln!("No script or Maven coordinate specified. Try: jbx run Hello.java");
                std::process::exit(2);
            };
            if should_run_as_maven_tool_shorthand(&script) {
                maven_tool::run(maven_tool::MavenToolOptions {
                    coordinate: script.to_string_lossy().into_owned(),
                    repos: cli.repos,
                    cache_dir: cli.cache_dir,
                    main_class: cli.main_class,
                    args: cli.args,
                })?
            } else {
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
            &[jbx::resolver::Repository {
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
    ) -> (jbx::resolver::Repository, std::thread::JoinHandle<()>) {
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
            jbx::resolver::Repository {
                id: "test".to_string(),
                url: format!("http://{addr}"),
            },
            handle,
        )
    }
}
