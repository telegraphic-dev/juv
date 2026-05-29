use anyhow::{Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
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
    /// Install the current project into a Maven repository layout.
    Install(InstallCommand),
    /// Print agent-friendly documentation for source, directories, or Maven artifacts.
    Docs(DocsCommand),
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
struct DocsCommand {
    /// Maven GAV, Java source file, docs sidecar, or directory to document.
    target: String,

    /// Print JSON instead of Markdown.
    #[arg(long = "json")]
    json: bool,

    /// Additional repository for remote Maven docs sidecars (id=url format or bare URL).
    #[arg(long = "repo", alias = "repos")]
    repos: Vec<String>,

    /// Limit structured output to matching type names. Repeatable; accepts simple or fully-qualified names.
    #[arg(long = "type", alias = "types")]
    types: Vec<String>,

    /// Override remote docs cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
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

    /// Serve a local Maven repository containing the artifact on the given port.
    #[arg(long = "serve", conflicts_with_all = ["dry_run", "publish"])]
    serve: Option<u16>,

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

#[derive(Parser, Debug)]
struct InstallCommand {
    /// Java source file to install. Defaults to jbx.json main when --file is used.
    script: Option<PathBuf>,

    /// jbx descriptor file. Defaults to ./jbx.json when present.
    #[arg(long = "file")]
    file: Option<PathBuf>,

    /// Override version from jbx.json or //GAV.
    #[arg(long = "version")]
    version: Option<String>,

    /// Destination Maven repository root. Defaults to ~/.m2/repository.
    #[arg(long = "destination", alias = "to")]
    destination: Option<PathBuf>,

    /// Working directory for staged install artifacts.
    #[arg(long = "target-dir")]
    target_dir: Option<PathBuf>,

    /// Override package used when staging default-package sources.
    #[arg(long = "package")]
    package_name: Option<String>,

    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
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

#[derive(Debug, Clone)]
struct DocsCoordinate {
    group: String,
    id: String,
    version: String,
}

fn run_docs(cmd: DocsCommand) -> Result<i32> {
    let target_path = PathBuf::from(&cmd.target);
    let output = if target_path.exists() {
        render_local_docs(&target_path, cmd.json, &cmd.types)?
    } else if looks_like_docs_coordinate(&cmd.target) {
        fetch_remote_docs(&cmd)?
    } else {
        anyhow::bail!("docs target not found: {}", cmd.target);
    };
    print!("{output}");
    Ok(0)
}

fn looks_like_docs_coordinate(value: &str) -> bool {
    let part_count = value.split(':').count();
    !value.starts_with("http://")
        && !value.starts_with("https://")
        && (part_count == 2 || part_count == 3)
        && value.split(':').all(|part| !part.is_empty())
}

fn parse_docs_coordinate(
    value: &str,
    repos: &[jbx::resolver::Repository],
) -> Result<DocsCoordinate> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 2 && parts.len() != 3 {
        anyhow::bail!(
            "docs requires Maven coordinates as group:artifact or group:artifact:version"
        );
    }
    let group = parts[0].to_string();
    let id = parts[1].to_string();
    validate_group(&group)?;
    validate_path_safe_coordinate_part(&id, "id")?;
    let version = match parts.get(2) {
        Some(version) => {
            validate_path_safe_coordinate_part(version, "version")?;
            (*version).to_string()
        }
        None => jbx::resolver::resolve_latest_version(
            &jbx::resolver::Module {
                org: group.clone(),
                name: id.clone(),
            },
            repos,
        )?,
    };
    Ok(DocsCoordinate { group, id, version })
}

fn render_local_docs(path: &Path, json: bool, type_filters: &[String]) -> Result<String> {
    if is_jar_file(path) {
        return render_jar_docs(path, json, type_filters);
    }
    let sources = collect_java_files(&[path.to_path_buf()])?;
    if sources.is_empty() {
        anyhow::bail!(
            "docs target contains no Java source files: {}",
            path.display()
        );
    }
    let docs = sources
        .iter()
        .map(|source| docs_source_json(source, None))
        .collect::<Result<Vec<_>>>()?;
    let types = filter_docs_types(extract_docs_types_from_sources(&sources)?, type_filters);
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("docs"));
    if json {
        Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "https://telegraphic.dev/schemas/jbx-docs/v1.json",
                "target": path.to_string_lossy(),
                "sources": docs,
                "types": types,
                "generatedFrom": {
                    "source": "jbx-directives",
                    "jbxVersion": env!("CARGO_PKG_VERSION"),
                }
            }))?
        ))
    } else {
        render_docs_markdown(title, None, &docs, &types)
    }
}

fn docs_source_json(source: &Path, artifact: Option<&DocsCoordinate>) -> Result<serde_json::Value> {
    let directives = parsed_directives(&source.to_path_buf())?;
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("source");
    let docs = directives
        .docs
        .iter()
        .map(|doc| match &doc.value {
            Some(value) => serde_json::json!({"key": doc.key, "value": value}),
            None => serde_json::json!({"key": doc.key}),
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "path": source.to_string_lossy(),
        "name": name,
        "description": directives.description,
        "docs": docs,
        "dependencies": directives.deps,
        "repositories": directives.repos,
        "sources": directives.sources,
        "java": directives.java_version,
        "main": directives.main_class,
        "artifact": artifact.map(|coordinate| serde_json::json!({
            "group": coordinate.group,
            "id": coordinate.id,
            "version": coordinate.version,
            "coordinate": format!("{}:{}:{}", coordinate.group, coordinate.id, coordinate.version),
        })),
    }))
}

fn render_docs_markdown(
    title: &str,
    artifact: Option<&DocsCoordinate>,
    sources: &[serde_json::Value],
    types: &[serde_json::Value],
) -> Result<String> {
    let mut out = String::new();
    out.push_str(&format!("# {title}\n\n"));
    if let Some(coordinate) = artifact {
        out.push_str(&format!(
            "Artifact: `{}:{}:{}`\n\n",
            coordinate.group, coordinate.id, coordinate.version
        ));
    }
    for source in sources {
        let name = source
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("source");
        if sources.len() > 1 || artifact.is_some() {
            out.push_str(&format!("## {name}\n\n"));
        }
        if let Some(description) = source.get("description").and_then(|value| value.as_str()) {
            out.push_str(description);
            out.push_str("\n\n");
        }
        if let Some(docs) = source.get("docs").and_then(|value| value.as_array()) {
            for doc in docs {
                let key = doc
                    .get("key")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                match doc.get("value").and_then(|value| value.as_str()) {
                    Some(value) => out.push_str(&format!("- {key}: {value}\n")),
                    None => out.push_str(&format!("- {key}\n")),
                }
            }
            if !docs.is_empty() {
                out.push('\n');
            }
        }
        if let Some(deps) = source
            .get("dependencies")
            .and_then(|value| value.as_array())
        {
            if !deps.is_empty() {
                out.push_str("Dependencies:\n");
                for dep in deps.iter().filter_map(|value| value.as_str()) {
                    out.push_str(&format!("- `{dep}`\n"));
                }
                out.push('\n');
            }
        }
    }
    for ty in types {
        if let Some(qualified_name) = ty.get("qualifiedName").and_then(|value| value.as_str()) {
            out.push_str(&format!("## `{qualified_name}`\n\n"));
            if let Some(kind) = ty.get("kind").and_then(|value| value.as_str()) {
                out.push_str(&format!("Kind: {kind}\n\n"));
            }
            if let Some(description) = ty.get("description").and_then(|value| value.as_str()) {
                if !description.is_empty() {
                    out.push_str(description);
                    out.push_str("\n\n");
                }
            }
            render_examples_section(&mut out, ty.get("examples"));
            render_member_section(&mut out, "Fields", ty.get("fields"), render_field_signature);
            render_member_section(
                &mut out,
                "Constructors",
                ty.get("constructors"),
                render_constructor_signature,
            );
            render_member_section(
                &mut out,
                "Methods",
                ty.get("methods"),
                render_method_signature,
            );
        }
    }
    Ok(out)
}

fn render_examples_section(out: &mut String, value: Option<&serde_json::Value>) {
    let Some(examples) = value.and_then(|value| value.as_array()) else {
        return;
    };
    let examples = examples
        .iter()
        .filter_map(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    if examples.is_empty() {
        return;
    }
    out.push_str("### Examples\n\n");
    for example in examples {
        out.push_str("```java\n");
        out.push_str(example.trim());
        out.push_str("\n```\n\n");
    }
}

fn render_member_section(
    out: &mut String,
    title: &str,
    value: Option<&serde_json::Value>,
    render: fn(&serde_json::Value) -> Option<String>,
) {
    let Some(members) = value.and_then(|value| value.as_array()) else {
        return;
    };
    if members.iter().filter_map(render).next().is_none() {
        return;
    }
    out.push_str(&format!("### {title}\n\n"));
    for member in members {
        let Some(signature) = render(member) else {
            continue;
        };
        out.push_str(&format!("- `{signature}`"));
        if let Some(description) = member.get("description").and_then(|value| value.as_str()) {
            if !description.trim().is_empty() {
                out.push_str(&format!(" — {}", inline_markdown(description)));
            }
        }
        out.push('\n');
        render_parameter_descriptions(out, member);
        if let Some(description) = member
            .get("returnDescription")
            .and_then(|value| value.as_str())
        {
            if !description.trim().is_empty() {
                out.push_str(&format!("  - Returns: {}\n", inline_markdown(description)));
            }
        }
    }
    out.push('\n');
}

fn render_parameter_descriptions(out: &mut String, member: &serde_json::Value) {
    let Some(parameters) = member.get("parameters").and_then(|value| value.as_array()) else {
        return;
    };
    for parameter in parameters {
        let Some(description) = parameter
            .get("description")
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        if description.trim().is_empty() {
            continue;
        }
        let name = parameter
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("parameter");
        out.push_str(&format!("  - `{name}`: {}\n", inline_markdown(description)));
    }
}

fn inline_markdown(markdown: &str) -> String {
    markdown.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn render_field_signature(field: &serde_json::Value) -> Option<String> {
    let name = field.get("name")?.as_str()?;
    let ty = field.get("type")?.as_str()?;
    Some(join_signature_parts(&[
        member_modifiers(field),
        ty.to_string(),
        name.to_string(),
    ]))
}

fn render_constructor_signature(constructor: &serde_json::Value) -> Option<String> {
    let name = constructor.get("name")?.as_str()?;
    Some(render_callable_signature(None, name, constructor))
}

fn render_method_signature(method: &serde_json::Value) -> Option<String> {
    let name = method.get("name")?.as_str()?;
    let return_type = method.get("returnType")?.as_str().unwrap_or("void");
    Some(render_callable_signature(Some(return_type), name, method))
}

fn render_callable_signature(
    return_type: Option<&str>,
    name: &str,
    member: &serde_json::Value,
) -> String {
    let mut head = Vec::new();
    if let Some(return_type) = return_type {
        head.push(return_type.to_string());
    }
    head.push(format!(
        "{name}({})",
        render_parameters(member.get("parameters"))
    ));
    let mut signature = join_signature_parts(&head);
    if let Some(throws) = render_throws(member.get("throws")) {
        signature.push_str(" throws ");
        signature.push_str(&throws);
    }
    signature
}

fn member_modifiers(member: &serde_json::Value) -> String {
    member
        .get("modifiers")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn render_parameters(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|value| value.as_array())
        .map(|params| {
            params
                .iter()
                .filter_map(|param| {
                    Some(format!(
                        "{} {}",
                        param.get("type")?.as_str()?,
                        param.get("name")?.as_str()?
                    ))
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn render_throws(value: Option<&serde_json::Value>) -> Option<String> {
    let throws = value
        .and_then(|value| value.as_array())?
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    (!throws.is_empty()).then(|| throws.join(", "))
}

fn join_signature_parts(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}
fn is_jar_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
}

fn render_jar_docs(path: &Path, json: bool, type_filters: &[String]) -> Result<String> {
    let docs_source = if find_javadoc_jar(path).is_some() {
        "javadoc"
    } else {
        "javap"
    };
    let types = filter_docs_types(extract_docs_types_from_jar(path)?, type_filters);
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("jar"));
    if json {
        Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "https://telegraphic.dev/schemas/jbx-docs/v1.json",
                "target": path.to_string_lossy(),
                "types": types,
                "generatedFrom": {
                    "source": docs_source,
                    "jbxVersion": env!("CARGO_PKG_VERSION"),
                }
            }))?
        ))
    } else {
        render_docs_markdown(title, None, &[], &types)
    }
}

fn extract_docs_types_from_sources(sources: &[PathBuf]) -> Result<Vec<serde_json::Value>> {
    let mut types = Vec::new();
    for source in sources {
        let content = fs::read_to_string(source)
            .with_context(|| format!("failed to read Java source {}", source.display()))?;
        types.extend(extract_docs_types_from_source(&content));
    }
    Ok(types)
}

fn extract_docs_types_from_source(content: &str) -> Vec<serde_json::Value> {
    let package = parse_java_package(content).unwrap_or_default();
    let mut types = Vec::new();
    let mut pending_annotations = Vec::new();
    let mut current: Option<DocsTypeBuilder> = None;
    let mut brace_depth = 0_i32;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
        {
            continue;
        }
        if line.starts_with('@') {
            pending_annotations.push(parse_annotation(line, &package));
            continue;
        }
        if current.is_none() {
            if let Some(builder) =
                parse_type_declaration(line, &package, std::mem::take(&mut pending_annotations))
            {
                current = Some(builder);
                brace_depth += count_char(line, '{') - count_char(line, '}');
            } else {
                pending_annotations.clear();
            }
            continue;
        }
        brace_depth += count_char(line, '{') - count_char(line, '}');
        if let Some(builder) = current.as_mut() {
            if let Some(member) = parse_member_declaration(
                line,
                &package,
                &builder.name,
                std::mem::take(&mut pending_annotations),
            ) {
                builder.push_member(member);
            } else if !line.starts_with('@') {
                pending_annotations.clear();
            }
        }
        if brace_depth <= 0 {
            if let Some(builder) = current.take() {
                types.push(builder.into_json());
            }
            pending_annotations.clear();
            brace_depth = 0;
        }
    }
    if let Some(builder) = current {
        types.push(builder.into_json());
    }
    types
}

#[derive(Debug)]
struct DocsTypeBuilder {
    kind: String,
    name: String,
    qualified_name: String,
    package: String,
    visibility: String,
    modifiers: Vec<String>,
    annotations: Vec<serde_json::Value>,
    description: Option<String>,
    examples: Vec<String>,
    extends: Option<String>,
    implements: Vec<String>,
    fields: Vec<serde_json::Value>,
    constructors: Vec<serde_json::Value>,
    methods: Vec<serde_json::Value>,
}

impl DocsTypeBuilder {
    fn push_member(&mut self, member: DocsMember) {
        match member {
            DocsMember::Field(value) => self.fields.push(value),
            DocsMember::Constructor(value) => self.constructors.push(value),
            DocsMember::Method(value) => self.methods.push(value),
        }
    }

    fn into_json(self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.kind,
            "name": self.name,
            "qualifiedName": self.qualified_name,
            "package": self.package,
            "visibility": self.visibility,
            "modifiers": self.modifiers,
            "annotations": self.annotations,
            "description": self.description,
            "examples": self.examples,
            "extends": self.extends,
            "implements": self.implements,
            "fields": self.fields,
            "constructors": self.constructors,
            "methods": self.methods,
        })
    }
}

enum DocsMember {
    Field(serde_json::Value),
    Constructor(serde_json::Value),
    Method(serde_json::Value),
}

fn parse_java_package(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("package ")
            .and_then(|rest| rest.strip_suffix(';'))
            .map(|package| package.trim().to_string())
    })
}

fn parse_type_declaration(
    line: &str,
    package: &str,
    annotations: Vec<serde_json::Value>,
) -> Option<DocsTypeBuilder> {
    let header = line.split('{').next().unwrap_or(line).trim();
    let tokens = split_java_words(header);
    let kind_index = tokens
        .iter()
        .position(|token| matches!(token.as_str(), "class" | "interface" | "enum" | "record"))?;
    let kind = tokens.get(kind_index)?.to_string();
    let name = tokens
        .get(kind_index + 1)?
        .trim_end_matches('(')
        .to_string();
    let visibility = parse_visibility(&tokens);
    let modifiers = parse_modifiers(&tokens[..kind_index]);
    let qualified_name = qualify_name(package, &name);
    let extends = tokens
        .iter()
        .position(|token| token == "extends")
        .and_then(|index| tokens.get(index + 1))
        .map(|name| qualify_type(package, name.trim_end_matches(',')));
    let implements = tokens
        .iter()
        .position(|token| token == "implements")
        .map(|index| {
            tokens[index + 1..]
                .iter()
                .map(|token| token.trim_end_matches(','))
                .filter(|token| !token.is_empty())
                .map(|token| qualify_type(package, token))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(DocsTypeBuilder {
        kind,
        name,
        qualified_name,
        package: package.to_string(),
        visibility,
        modifiers,
        annotations,
        description: None,
        examples: Vec::new(),
        extends,
        implements,
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: Vec::new(),
    })
}

fn parse_member_declaration(
    line: &str,
    package: &str,
    type_name: &str,
    annotations: Vec<serde_json::Value>,
) -> Option<DocsMember> {
    let declaration = line
        .split('{')
        .next()
        .unwrap_or(line)
        .split('=')
        .next()
        .unwrap_or(line)
        .trim()
        .trim_end_matches(';')
        .trim();
    if declaration.is_empty() || declaration.starts_with("return ") {
        return None;
    }
    if declaration.contains('(') && declaration.contains(')') {
        parse_method_or_constructor(declaration, package, type_name, annotations)
    } else {
        parse_field(declaration, package, type_name, annotations).map(DocsMember::Field)
    }
}

fn parse_field(
    declaration: &str,
    package: &str,
    type_name: &str,
    annotations: Vec<serde_json::Value>,
) -> Option<serde_json::Value> {
    let (head, name) = split_type_and_name(declaration)?;
    let type_token = strip_leading_modifiers(&head);
    let tokens = split_java_words(declaration);
    let visibility = parse_visibility(&tokens);
    let modifiers = parse_modifiers(&tokens);
    Some(serde_json::json!({
        "name": name,
        "qualifiedName": format!("{}.{}.{}", package, type_name, name),
        "visibility": visibility,
        "modifiers": modifiers,
        "annotations": annotations,
        "type": qualify_type(package, &type_token),
    }))
}

fn parse_method_or_constructor(
    declaration: &str,
    package: &str,
    type_name: &str,
    annotations: Vec<serde_json::Value>,
) -> Option<DocsMember> {
    let open = declaration.find('(')?;
    let close = declaration.rfind(')')?;
    let before = declaration[..open].trim();
    let params = &declaration[open + 1..close];
    let after = declaration[close + 1..].trim();
    let (head, name) = split_type_and_name(before)?;
    let tokens = split_java_words(before);
    let visibility = parse_visibility(&tokens);
    let modifiers = parse_modifiers(&tokens);
    let parameters = parse_parameters(params, package);
    let throws = parse_throws(after, package);
    if name == type_name {
        return Some(DocsMember::Constructor(serde_json::json!({
            "name": name,
            "qualifiedName": format!("{}.{}.{}", package, type_name, name),
            "visibility": visibility,
            "modifiers": modifiers,
            "annotations": annotations,
            "parameters": parameters,
            "throws": throws,
        })));
    }
    let return_type = strip_method_type_parameters(&strip_leading_modifiers(&head));
    Some(DocsMember::Method(serde_json::json!({
        "name": name,
        "qualifiedName": format!("{}.{}.{}", package, type_name, name),
        "visibility": visibility,
        "modifiers": modifiers,
        "annotations": annotations,
        "parameters": parameters,
        "returnType": qualify_type(package, &return_type),
        "throws": throws,
    })))
}

fn split_type_and_name(input: &str) -> Option<(String, String)> {
    let split_at = input
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_whitespace())?
        .0;
    let head = input[..split_at].trim();
    let name = input[split_at..].trim();
    if head.is_empty() || name.is_empty() {
        return None;
    }
    Some((head.to_string(), name.to_string()))
}

fn strip_leading_modifiers(input: &str) -> String {
    let mut rest = input.trim();
    while let Some((candidate, tail)) = rest.split_once(char::is_whitespace) {
        if !is_java_modifier(candidate) {
            break;
        }
        rest = tail.trim_start();
    }
    rest.to_string()
}

fn strip_method_type_parameters(input: &str) -> String {
    let input = input.trim();
    if !input.starts_with('<') {
        return input.to_string();
    }
    let mut depth = 0_i32;
    for (index, ch) in input.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return input[index + ch.len_utf8()..].trim().to_string();
                }
            }
            _ => {}
        }
    }
    input.to_string()
}

fn parse_parameters(params: &str, package: &str) -> Vec<serde_json::Value> {
    if params.trim().is_empty() {
        return Vec::new();
    }
    split_java_commas(params)
        .into_iter()
        .enumerate()
        .filter_map(|(index, param)| {
            let param = param.trim();
            if param.is_empty() {
                return None;
            }
            let (type_name, name) = split_parameter_type_and_name(param)
                .unwrap_or_else(|| (param.to_string(), format!("arg{index}")));
            Some(serde_json::json!({
                "name": name,
                "type": qualify_type(package, &type_name),
            }))
        })
        .collect()
}

fn split_parameter_type_and_name(param: &str) -> Option<(String, String)> {
    let split_at = param
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_whitespace())?
        .0;
    let type_name = param[..split_at].trim();
    let name = param[split_at..].trim();
    if type_name.is_empty() || name.is_empty() || name.contains('.') {
        return None;
    }
    let type_name = type_name.strip_prefix("final ").unwrap_or(type_name).trim();
    Some((type_name.to_string(), name.to_string()))
}

fn parse_throws(after: &str, package: &str) -> Vec<String> {
    after
        .strip_prefix("throws ")
        .map(|throws| {
            split_java_commas(throws)
                .into_iter()
                .map(|name| qualify_type(package, name.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_annotation(line: &str, package: &str) -> serde_json::Value {
    let body = line.trim().trim_start_matches('@');
    let name = body.split(['(', ' ', '\t']).next().unwrap_or(body).trim();
    let qualified_name = if name.contains('.') {
        name.to_string()
    } else if is_java_lang_type(name) {
        format!("java.lang.{name}")
    } else {
        qualify_name(package, name)
    };
    serde_json::json!({
        "qualifiedName": qualified_name,
        "values": {},
    })
}

fn extract_docs_types_from_jar(path: &Path) -> Result<Vec<serde_json::Value>> {
    if let Some(javadocs) = find_javadoc_jar(path) {
        let types = extract_docs_types_from_javadoc_jar(&javadocs)?;
        if !types.is_empty() {
            return Ok(types);
        }
    }
    extract_docs_types_from_bytecode(path)
}

fn find_javadoc_jar(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let parent = path.parent()?;
    let candidates = [
        parent.join(format!("{stem}-javadoc.jar")),
        parent.join(format!("{stem}-javadocs.jar")),
    ];
    candidates.into_iter().find(|candidate| candidate.exists())
}

fn extract_docs_types_from_javadoc_jar(path: &Path) -> Result<Vec<serde_json::Value>> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to open javadoc jar {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read javadoc jar {}", path.display()))?;
    let mut types = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        if !is_javadoc_type_page(&name) {
            continue;
        }
        let mut html = String::new();
        entry.read_to_string(&mut html)?;
        if let Some(ty) = parse_javadoc_type_page(&name, &html) {
            types.push(ty);
        }
    }
    types.sort_by_key(|value| {
        value
            .get("qualifiedName")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    });
    Ok(types)
}

fn is_javadoc_type_page(name: &str) -> bool {
    name.ends_with(".html")
        && !name.contains('-')
        && !name.ends_with("/package-summary.html")
        && !name.ends_with("/module-summary.html")
        && !name.ends_with("/overview-summary.html")
        && !name.ends_with("/index.html")
        && name.rsplit('/').next().is_some_and(|file| {
            file.chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        })
}

fn parse_javadoc_type_page(path: &str, html: &str) -> Option<serde_json::Value> {
    let qualified_name = path.trim_end_matches(".html").replace('/', ".");
    let (package, name) = qualified_name
        .rsplit_once('.')
        .map(|(package, name)| (package.to_string(), name.to_string()))
        .unwrap_or_else(|| (String::new(), qualified_name.clone()));
    let text = normalize_doc_text(&strip_html_tags(html));
    let kind = if text.contains(&format!("interface {name}")) {
        "interface"
    } else if text.contains(&format!("enum {name}")) {
        "enum"
    } else if text.contains(&format!("record {name}")) {
        "record"
    } else {
        "class"
    };
    let member_docs = extract_javadoc_member_docs(html);
    let description = extract_javadoc_type_description(html);
    let examples = extract_javadoc_examples(html);
    let mut builder = DocsTypeBuilder {
        kind: kind.to_string(),
        name: name.clone(),
        qualified_name: qualified_name.clone(),
        package: package.clone(),
        visibility: "public".to_string(),
        modifiers: vec!["public".to_string()],
        annotations: Vec::new(),
        description,
        examples,
        extends: None,
        implements: Vec::new(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: Vec::new(),
    };
    for member_doc in member_docs {
        if let Some(mut member) =
            parse_member_declaration(&member_doc.signature, &package, &name, Vec::new())
        {
            enrich_member_with_javadoc(&mut member, &member_doc);
            builder.push_member(member);
        }
    }
    Some(builder.into_json())
}

#[derive(Debug, Clone, Default)]
struct JavadocMemberDoc {
    signature: String,
    description: Option<String>,
    parameter_descriptions: Vec<(String, String)>,
    return_description: Option<String>,
}

fn extract_javadoc_member_docs(html: &str) -> Vec<JavadocMemberDoc> {
    let detail_re =
        regex::Regex::new(r#"(?s)<section class=\"detail\"[^>]*>(.*?)</section>"#).unwrap();
    let signature_re =
        regex::Regex::new(r#"(?s)<div class=\"member-signature\">(.*?)</div>"#).unwrap();
    let mut members = Vec::new();
    for detail in detail_re
        .captures_iter(html)
        .filter_map(|captures| captures.get(1).map(|value| value.as_str()))
    {
        let Some(signature_html) = signature_re
            .captures(detail)
            .and_then(|captures| captures.get(1).map(|value| value.as_str()))
        else {
            continue;
        };
        let signature = normalize_doc_text(&strip_html_tags(signature_html));
        if signature.is_empty() {
            continue;
        }
        members.push(JavadocMemberDoc {
            signature,
            description: extract_javadoc_member_description(detail),
            parameter_descriptions: extract_javadoc_parameter_descriptions(detail),
            return_description: extract_javadoc_return_description(detail),
        });
    }
    if members.is_empty() {
        members.extend(
            extract_javadoc_signatures(html)
                .into_iter()
                .map(|signature| JavadocMemberDoc {
                    signature,
                    ..JavadocMemberDoc::default()
                }),
        );
    }
    members
}

fn extract_javadoc_member_description(detail_html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"(?s)<div class=\"block\">(.*?)</div>"#).unwrap();
    re.captures(detail_html)
        .and_then(|captures| captures.get(1))
        .map(|value| html_fragment_to_markdown(value.as_str()))
        .filter(|value| !value.is_empty())
}

fn extract_javadoc_parameter_descriptions(detail_html: &str) -> Vec<(String, String)> {
    let re = regex::Regex::new(r#"(?s)<dd>\s*<code>(.*?)</code>\s*-\s*(.*?)</dd>"#).unwrap();
    re.captures_iter(detail_html)
        .filter_map(|captures| {
            let name = captures
                .get(1)
                .map(|value| normalize_doc_text(&strip_html_tags(value.as_str())))?;
            let description = captures
                .get(2)
                .map(|value| html_fragment_to_markdown(value.as_str()))?;
            if name.is_empty() || description.is_empty() {
                None
            } else {
                Some((name, description))
            }
        })
        .collect()
}

fn extract_javadoc_return_description(detail_html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"(?s)<dt>Returns:</dt>\s*<dd>(.*?)</dd>"#).unwrap();
    re.captures(detail_html)
        .and_then(|captures| captures.get(1))
        .map(|value| html_fragment_to_markdown(value.as_str()))
        .filter(|value| !value.is_empty())
}

fn enrich_member_with_javadoc(member: &mut DocsMember, doc: &JavadocMemberDoc) {
    let value = match member {
        DocsMember::Field(value) | DocsMember::Constructor(value) | DocsMember::Method(value) => {
            value
        }
    };
    if let Some(description) = &doc.description {
        value["description"] = serde_json::Value::String(description.clone());
    }
    if let Some(return_description) = &doc.return_description {
        value["returnDescription"] = serde_json::Value::String(return_description.clone());
    }
    if let Some(parameters) = value
        .get_mut("parameters")
        .and_then(|value| value.as_array_mut())
    {
        for parameter in parameters {
            let Some(parameter_name) = parameter.get("name").and_then(|value| value.as_str())
            else {
                continue;
            };
            if let Some((_, description)) = doc
                .parameter_descriptions
                .iter()
                .find(|(name, _)| name == parameter_name)
            {
                parameter["description"] = serde_json::Value::String(description.clone());
            }
        }
    }
}

fn extract_javadoc_signatures(html: &str) -> Vec<String> {
    let mut signatures = Vec::new();
    let re = regex::Regex::new(r#"(?s)<div class="member-signature">(.*?)</div>"#).unwrap();
    for captures in re.captures_iter(html) {
        if let Some(signature) = captures.get(1) {
            let text = normalize_doc_text(&strip_html_tags(signature.as_str()));
            if !text.is_empty() {
                signatures.push(text);
            }
        }
    }
    if signatures.is_empty() {
        let pre = regex::Regex::new(r#"(?s)<pre[^>]*>(.*?)</pre>"#).unwrap();
        for captures in pre.captures_iter(html) {
            if let Some(signature) = captures.get(1) {
                let text = normalize_doc_text(&strip_html_tags(signature.as_str()))
                    .trim_end_matches(';')
                    .to_string();
                if is_javadoc_member_signature(&text) {
                    signatures.push(text);
                }
            }
        }
    }
    signatures
}

fn is_javadoc_member_signature(text: &str) -> bool {
    let text = strip_leading_annotation_lines(text);
    let tokens = split_java_words(text);
    if tokens.is_empty() {
        return false;
    }
    if !matches!(tokens[0].as_str(), "public" | "protected" | "private") {
        return false;
    }
    !tokens
        .iter()
        .any(|token| matches!(token.as_str(), "class" | "interface" | "enum" | "record"))
}

fn is_javadoc_type_signature(text: &str) -> bool {
    split_java_words(strip_leading_annotation_lines(text))
        .iter()
        .any(|token| matches!(token.as_str(), "class" | "interface" | "enum" | "record"))
}

fn strip_leading_annotation_lines(text: &str) -> &str {
    let mut rest = text.trim();
    while rest.starts_with('@') {
        let Some((_, tail)) = rest.split_once(char::is_whitespace) else {
            return "";
        };
        rest = tail.trim_start();
    }
    rest
}

fn extract_javadoc_examples(html: &str) -> Vec<String> {
    let pre = regex::Regex::new(r#"(?s)<pre[^>]*>(.*?)</pre>"#).unwrap();
    let mut examples = Vec::new();
    for example in pre
        .captures_iter(html)
        .filter_map(|captures| captures.get(1))
        .map(|example| normalize_code_block(&strip_html_tags(example.as_str())))
    {
        let normalized = normalize_doc_text(&example);
        if !example.is_empty()
            && !is_javadoc_member_signature(&normalized)
            && !is_javadoc_type_signature(&normalized)
            && !examples.contains(&example)
        {
            examples.push(example);
        }
    }
    examples
}

fn normalize_code_block(input: &str) -> String {
    input
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn extract_javadoc_type_description(html: &str) -> Option<String> {
    let patterns = [
        r#"(?s)<section[^>]*class="[^"]*(?:class|interface|enum|record)-description[^"]*"[^>]*>.*?<div class="block">(.*?)</div>"#,
        r#"(?s)<div class="description">.*?<div class="block">(.*?)</div>"#,
    ];
    patterns.iter().find_map(|pattern| {
        let re = regex::Regex::new(pattern).ok()?;
        let html = re.captures(html)?.get(1)?.as_str();
        let markdown = html_fragment_to_markdown(html);
        (!markdown.is_empty()).then_some(markdown)
    })
}

fn html_fragment_to_markdown(input: &str) -> String {
    let fragment = html_unescape(input);
    let markdown = quick_html2md::html_to_markdown(&format!("<div>{fragment}</div>"));
    normalize_markdown_text(&markdown)
}

fn normalize_markdown_text(input: &str) -> String {
    let mut out = String::new();
    let mut blank_lines = 0;
    let mut in_fence = false;
    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(trimmed);
            out.push('\n');
            in_fence = !in_fence;
            blank_lines = 0;
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if trimmed.is_empty() {
            blank_lines += 1;
            if blank_lines <= 1 && !out.trim().is_empty() && !out.ends_with("\n\n") {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            continue;
        }
        blank_lines = 0;
        if is_markdown_block_line(trimmed) {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(trimmed);
            out.push('\n');
        } else {
            if !out.is_empty() && !out.ends_with(['\n', ' ']) {
                out.push(' ');
            }
            out.push_str(trimmed);
        }
    }
    out.trim().to_string()
}

fn is_markdown_block_line(line: &str) -> bool {
    line.starts_with("#")
        || line.starts_with(">")
        || line.starts_with("- ")
        || line.starts_with("* ")
        || regex::Regex::new(r#"^\d+\.\s"#).unwrap().is_match(line)
}

fn strip_html_tags(input: &str) -> String {
    let tags = regex::Regex::new(r#"(?s)<[^>]+>"#).unwrap();
    html_unescape(&tags.replace_all(input, " "))
}

fn html_unescape(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn normalize_doc_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_docs_types_from_bytecode(path: &Path) -> Result<Vec<serde_json::Value>> {
    let output = ProcessCommand::new("jar")
        .arg("tf")
        .arg(path)
        .output()
        .with_context(|| "failed to run jar tf for docs extraction")?;
    if !output.status.success() {
        anyhow::bail!(
            "jar tf failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let class_names = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.ends_with(".class") && !line.contains('$'))
        .map(|line| line.trim_end_matches(".class").replace('/', "."))
        .collect::<Vec<_>>();
    let mut types = Vec::new();
    for class_name in class_names {
        let output = ProcessCommand::new("javap")
            .arg("-classpath")
            .arg(path)
            .arg("-protected")
            .arg("-v")
            .arg(&class_name)
            .output()
            .with_context(|| format!("failed to run javap for {class_name}"))?;
        if output.status.success() {
            if let Some(ty) = parse_javap_type(&String::from_utf8_lossy(&output.stdout)) {
                types.push(ty);
            }
        }
    }
    Ok(types)
}

fn parse_javap_type(output: &str) -> Option<serde_json::Value> {
    let signature_lines = output
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.ends_with(';')
                || line.ends_with('{')
                || line.starts_with("public class ")
                || line.starts_with("protected class ")
                || line.starts_with("class ")
                || line.starts_with("public interface ")
                || line.starts_with("public enum ")
        })
        .filter(|line| {
            !line.starts_with("descriptor:")
                && !line.starts_with("flags:")
                && !line.starts_with('#')
                && !line.starts_with("Classfile ")
                && !line.starts_with("Last modified ")
                && !line.starts_with("SHA-256 ")
                && !line.starts_with("Compiled from ")
        })
        .collect::<Vec<_>>();
    let type_line = signature_lines.iter().find(|line| {
        line.contains(" class ") || line.contains(" interface ") || line.contains(" enum ")
    })?;
    let header = type_line.trim_end_matches('{').trim();
    let tokens = split_java_words(header);
    let kind_index = tokens
        .iter()
        .position(|token| matches!(token.as_str(), "class" | "interface" | "enum"))?;
    let kind = tokens[kind_index].clone();
    let qualified_name = tokens.get(kind_index + 1)?.to_string();
    let package = qualified_name
        .rsplit_once('.')
        .map(|(package, _)| package.to_string())
        .unwrap_or_default();
    let name = qualified_name
        .rsplit_once('.')
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| qualified_name.clone());
    let mut builder = DocsTypeBuilder {
        kind,
        name: name.clone(),
        qualified_name: qualified_name.clone(),
        package: package.clone(),
        visibility: parse_visibility(&tokens),
        modifiers: parse_modifiers(&tokens[..kind_index]),
        annotations: Vec::new(),
        description: None,
        examples: Vec::new(),
        extends: None,
        implements: Vec::new(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: Vec::new(),
    };
    let parameter_names = javap_parameter_names(output);
    for line in signature_lines {
        let line = line.trim_end_matches(';').trim();
        if line == header || line.ends_with('{') {
            continue;
        }
        if line.contains('(') && line.contains(')') {
            if let Some(mut member) = parse_method_or_constructor(line, &package, &name, Vec::new())
            {
                if let DocsMember::Method(value) | DocsMember::Constructor(value) = &mut member {
                    if let Some(names) = value
                        .get("name")
                        .and_then(|value| value.as_str())
                        .and_then(|method_name| parameter_names.get(method_name))
                    {
                        apply_parameter_names(value, names);
                    }
                }
                builder.push_member(member);
            }
        } else if let Some(field) = parse_field(line, &package, &name, Vec::new()) {
            builder.fields.push(field);
        }
    }
    Some(builder.into_json())
}

fn javap_parameter_names(output: &str) -> std::collections::HashMap<String, Vec<String>> {
    let lines = output.lines().collect::<Vec<_>>();
    let mut names_by_method = std::collections::HashMap::new();
    let mut current_method: Option<String> = None;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if line.ends_with(';')
            && line.contains('(')
            && !line.starts_with("descriptor:")
            && !line.starts_with('#')
        {
            current_method = line
                .split('(')
                .next()
                .and_then(|before| split_java_words(before).last().cloned());
        } else if line == "MethodParameters:" {
            if let Some(method) = current_method.clone() {
                let mut names = Vec::new();
                index += 2;
                while index < lines.len() {
                    let candidate = lines[index].trim();
                    if candidate.is_empty() || candidate.ends_with(':') || candidate.contains(';') {
                        break;
                    }
                    if let Some(name) = candidate.split_whitespace().next() {
                        names.push(name.to_string());
                    }
                    index += 1;
                }
                names_by_method.insert(method, names);
            }
        }
        index += 1;
    }
    names_by_method
}

fn apply_parameter_names(value: &mut serde_json::Value, names: &[String]) {
    if let Some(params) = value
        .get_mut("parameters")
        .and_then(|value| value.as_array_mut())
    {
        for (param, name) in params.iter_mut().zip(names) {
            param["name"] = serde_json::Value::String(name.clone());
        }
    }
}

fn split_java_words(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|token| token.trim_matches(',').to_string())
        .filter(|token| !token.is_empty())
        .collect()
}

fn split_java_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut angle_depth = 0_i32;
    let mut paren_depth = 0_i32;
    for (index, ch) in input.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if angle_depth == 0 && paren_depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts.into_iter().filter(|part| !part.is_empty()).collect()
}

fn parse_visibility(tokens: &[String]) -> String {
    if tokens.iter().any(|token| token == "public") {
        "public".to_string()
    } else if tokens.iter().any(|token| token == "protected") {
        "protected".to_string()
    } else if tokens.iter().any(|token| token == "private") {
        "private".to_string()
    } else {
        "package".to_string()
    }
}

fn parse_modifiers(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| is_java_modifier(token))
        .cloned()
        .collect()
}

fn is_java_modifier(token: &str) -> bool {
    matches!(
        token,
        "public"
            | "protected"
            | "private"
            | "static"
            | "final"
            | "abstract"
            | "default"
            | "sealed"
            | "non-sealed"
            | "synchronized"
            | "native"
            | "strictfp"
    )
}

fn qualify_name(package: &str, name: &str) -> String {
    if package.is_empty() || name.contains('.') {
        name.to_string()
    } else {
        format!("{package}.{name}")
    }
}

fn qualify_type(package: &str, name: &str) -> String {
    let name = normalize_java_type_spacing(name.trim().trim_end_matches(','));
    if let Some(simple) = name.strip_prefix("java.lang.") {
        return simple.to_string();
    }
    let base = name
        .trim_end_matches("...")
        .split(['<', '['])
        .next()
        .unwrap_or(&name)
        .trim();
    if name.is_empty()
        || name == "void"
        || is_type_variable(&name)
        || is_primitive_type(base)
        || is_java_lang_type(base)
        || is_common_jdk_simple_type(base)
        || is_unqualified_exception_or_error(base)
        || name.contains('.')
        || name.contains('<')
    {
        name.to_string()
    } else {
        qualify_name(package, &name)
    }
}

fn normalize_java_type_spacing(input: &str) -> String {
    let mut out = String::new();
    let mut previous_was_space = false;
    for ch in input.chars() {
        match ch {
            '<' | '>' | '[' | ']' => {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push(ch);
                previous_was_space = false;
            }
            ',' => {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push(ch);
                out.push(' ');
                previous_was_space = true;
            }
            ch if ch.is_whitespace() => {
                if !out.is_empty()
                    && !previous_was_space
                    && !out.ends_with('<')
                    && !out.ends_with('[')
                {
                    out.push(' ');
                    previous_was_space = true;
                }
            }
            _ => {
                out.push(ch);
                previous_was_space = false;
            }
        }
    }
    out.trim().to_string()
}

fn is_type_variable(name: &str) -> bool {
    let name = name.trim();
    name.len() == 1 && name.chars().all(|ch| ch.is_ascii_uppercase())
}

fn is_unqualified_exception_or_error(name: &str) -> bool {
    !name.contains('.') && (name.ends_with("Exception") || name.ends_with("Error"))
}

fn is_common_jdk_simple_type(name: &str) -> bool {
    matches!(
        name,
        "File"
            | "InputStream"
            | "OutputStream"
            | "Reader"
            | "Writer"
            | "DataInput"
            | "DataOutput"
            | "IOException"
            | "URL"
            | "URI"
            | "List"
            | "Set"
            | "Map"
            | "Collection"
            | "Iterable"
            | "Iterator"
            | "ConcurrentHashMap"
    )
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "boolean" | "byte" | "char" | "short" | "int" | "long" | "float" | "double"
    )
}

fn is_java_lang_type(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "Object"
            | "Class"
            | "Integer"
            | "Long"
            | "Boolean"
            | "Double"
            | "Float"
            | "Short"
            | "Byte"
            | "Character"
            | "ClassLoader"
            | "Throwable"
            | "Exception"
            | "RuntimeException"
            | "IllegalArgumentException"
            | "Deprecated"
            | "Override"
            | "SuppressWarnings"
            | "FunctionalInterface"
    )
}

fn count_char(input: &str, needle: char) -> i32 {
    input.chars().filter(|ch| *ch == needle).count() as i32
}

fn filter_docs_types(types: Vec<serde_json::Value>, filters: &[String]) -> Vec<serde_json::Value> {
    if filters.is_empty() {
        return types;
    }
    types
        .into_iter()
        .filter(|ty| docs_type_matches_filters(ty, filters))
        .collect()
}

fn docs_type_matches_filters(ty: &serde_json::Value, filters: &[String]) -> bool {
    let name = ty
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let qualified_name = ty
        .get("qualifiedName")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    filters.iter().any(|filter| {
        let filter = filter.trim();
        filter == name
            || filter == qualified_name
            || qualified_name.ends_with(&format!(".{filter}"))
            || simple_glob_match(filter, name)
            || simple_glob_match(filter, qualified_name)
    })
}

fn filter_remote_docs_text(text: String, json: bool, type_filters: &[String]) -> Result<String> {
    if !json || type_filters.is_empty() {
        return Ok(text);
    }
    let mut value: serde_json::Value = serde_json::from_str(&text)?;
    if let Some(types) = value
        .get_mut("types")
        .and_then(|value| value.as_array_mut())
    {
        let filtered = filter_docs_types(std::mem::take(types), type_filters);
        *types = filtered;
    }
    Ok(format!("{}\n", serde_json::to_string_pretty(&value)?))
}

fn simple_glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    match (pattern.strip_prefix('*'), pattern.strip_suffix('*')) {
        (Some(suffix), _) => value.ends_with(suffix),
        (_, Some(prefix)) => value.starts_with(prefix),
        _ => false,
    }
}

fn fetch_remote_docs(cmd: &DocsCommand) -> Result<String> {
    let repos = docs_repositories(&cmd.repos);
    let coordinate = parse_docs_coordinate(&cmd.target, &repos)?;
    let extension = if cmd.json { "json" } else { "md" };
    let filename = format!(
        "{}-{}-jbx-docs.{extension}",
        coordinate.id, coordinate.version
    );
    let cache_root = cmd
        .cache_dir
        .clone()
        .unwrap_or(default_cache_dir()?)
        .join("docs");
    let cache_path = cache_root
        .join(coordinate.group.replace('.', "/"))
        .join(&coordinate.id)
        .join(&coordinate.version)
        .join(&filename);
    if cache_path.exists() {
        let text = fs::read_to_string(&cache_path)
            .with_context(|| format!("failed to read cached docs {}", cache_path.display()))?;
        return filter_remote_docs_text(text, cmd.json, &cmd.types);
    }
    for repo in repos {
        let url = docs_artifact_url(&repo, &coordinate, &filename);
        match ureq::get(&url).call() {
            Ok(response) => {
                let text = response
                    .into_string()
                    .with_context(|| format!("failed to read docs sidecar from {url}"))?;
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&cache_path, &text)?;
                return filter_remote_docs_text(text, cmd.json, &cmd.types);
            }
            Err(ureq::Error::Status(404, _)) => continue,
            Err(_) => continue,
        }
    }
    fetch_remote_javadoc_docs(cmd, &coordinate, &cache_root)
}

fn fetch_remote_javadoc_docs(
    cmd: &DocsCommand,
    coordinate: &DocsCoordinate,
    cache_root: &Path,
) -> Result<String> {
    let filename = format!("{}-{}-javadoc.jar", coordinate.id, coordinate.version);
    let cache_path = cache_root
        .join(coordinate.group.replace('.', "/"))
        .join(&coordinate.id)
        .join(&coordinate.version)
        .join(&filename);
    if !cache_path.exists() {
        let mut found = false;
        for repo in docs_repositories(&cmd.repos) {
            let url = docs_artifact_url(&repo, coordinate, &filename);
            match ureq::get(&url).call() {
                Ok(response) => {
                    let mut bytes = Vec::new();
                    response
                        .into_reader()
                        .read_to_end(&mut bytes)
                        .with_context(|| format!("failed to read javadoc jar from {url}"))?;
                    if let Some(parent) = cache_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&cache_path, bytes)?;
                    found = true;
                    break;
                }
                Err(ureq::Error::Status(404, _)) => continue,
                Err(_) => continue,
            }
        }
        if !found {
            anyhow::bail!(
                "jbx docs sidecar or javadoc jar not found for {}:{}:{}",
                coordinate.group,
                coordinate.id,
                coordinate.version
            );
        }
    }
    let types = filter_docs_types(
        extract_docs_types_from_javadoc_jar(&cache_path)?,
        &cmd.types,
    );
    if cmd.json {
        Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "https://telegraphic.dev/schemas/jbx-docs/v1.json",
                "artifact": {
                    "group": coordinate.group,
                    "id": coordinate.id,
                    "version": coordinate.version,
                    "coordinate": format!("{}:{}:{}", coordinate.group, coordinate.id, coordinate.version),
                },
                "types": types,
                "generatedFrom": {
                    "source": "javadoc",
                    "jbxVersion": env!("CARGO_PKG_VERSION"),
                }
            }))?
        ))
    } else {
        render_docs_markdown(&coordinate.id, Some(coordinate), &[], &types)
    }
}

fn docs_repositories(repo_args: &[String]) -> Vec<jbx::resolver::Repository> {
    let mut repos = maven_tool::maven_repositories(repo_args);
    repos.sort_by_key(|repo| if repo.id == "central" { 1 } else { 0 });
    repos
}

fn docs_artifact_url(
    repo: &jbx::resolver::Repository,
    coordinate: &DocsCoordinate,
    filename: &str,
) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        repo.url.trim_end_matches('/'),
        coordinate.group.replace('.', "/"),
        coordinate.id,
        coordinate.version,
        filename
    )
}

fn publish_docs_outputs(
    descriptor: &PublishDescriptor,
    staged: &StagedPublishSources,
) -> Result<(String, String)> {
    let coordinate = DocsCoordinate {
        group: descriptor.coordinates.group.clone(),
        id: descriptor.coordinates.id.clone(),
        version: descriptor.coordinates.version.clone(),
    };
    let sources = staged
        .all_sources
        .iter()
        .map(|source| docs_source_json(source, Some(&coordinate)))
        .collect::<Result<Vec<_>>>()?;
    let title = descriptor
        .name
        .as_deref()
        .unwrap_or(&descriptor.coordinates.id);
    let types = extract_docs_types_from_sources(&staged.all_sources)?;
    let markdown = render_docs_markdown(title, Some(&coordinate), &sources, &types)?;
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "schema": "https://telegraphic.dev/schemas/jbx-docs/v1.json",
        "artifact": {
            "group": coordinate.group,
            "id": coordinate.id,
            "version": coordinate.version,
            "coordinate": format!("{}:{}:{}", coordinate.group, coordinate.id, coordinate.version),
        },
        "summary": descriptor.description,
        "sources": sources,
        "types": types,
        "generatedFrom": {
            "source": "jbx publish",
            "jbxVersion": env!("CARGO_PKG_VERSION"),
        }
    }))?;
    Ok((markdown, format!("{json}\n")))
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

fn run_publish(mut cmd: PublishCommand) -> Result<i32> {
    if cmd.publish && cmd.dry_run {
        anyhow::bail!("--dry-run and --publish are mutually exclusive; dry-run never uploads");
    }
    if !cmd.publish && !cmd.dry_run && cmd.serve.is_none() {
        anyhow::bail!(
            "publish requires --dry-run for local inspection, --publish for Maven Central upload, or --serve <port> for a local Maven repository server"
        );
    }
    if cmd.publish && cmd.skip_signing {
        anyhow::bail!("--publish requires signed artifacts; remove --skip-signing or use --dry-run for local inspection");
    }
    let descriptor = load_publish_descriptor(&cmd)?;
    if let Some(port) = cmd.serve {
        cmd.skip_signing = true;
        let repository = prepare_publish_repository(&descriptor, &cmd)?;
        write_maven_metadata(&repository, &descriptor, "maven-metadata.xml", true)?;
        serve_maven_repository(&repository, port)?;
        return Ok(0);
    }
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

fn run_install(cmd: InstallCommand) -> Result<i32> {
    let destination_arg = cmd.destination.clone();
    let publish_cmd = PublishCommand {
        script: cmd.script,
        file: cmd.file,
        version: cmd.version,
        output: None,
        target_dir: cmd.target_dir,
        package_name: cmd.package_name,
        cache_dir: cmd.cache_dir,
        dry_run: false,
        skip_signing: true,
        gpg_key: None,
        publish: false,
        serve: None,
        publishing_type: CentralPublishingType::Automatic,
        central_url: None,
        no_wait: false,
        poll_interval: 5,
        max_wait_seconds: 600,
    };
    let descriptor = load_publish_descriptor(&publish_cmd)?;
    let repository = prepare_publish_repository(&descriptor, &publish_cmd)?;
    let destination = cmd_destination_or_maven_local(destination_arg)?;
    copy_dir_contents(&repository, &destination)?;
    write_maven_metadata(&destination, &descriptor, "maven-metadata-local.xml", false)?;
    let installed = destination
        .join(descriptor.coordinates.group.replace('.', "/"))
        .join(&descriptor.coordinates.id)
        .join(&descriptor.coordinates.version);
    println!(
        "installed {}:{}:{} to {}",
        descriptor.coordinates.group,
        descriptor.coordinates.id,
        descriptor.coordinates.version,
        installed.display()
    );
    Ok(0)
}

fn cmd_destination_or_maven_local(destination: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(destination) = destination {
        return Ok(destination);
    }
    dirs::home_dir()
        .map(|home| home.join(".m2").join("repository"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not determine home directory for Maven local repository; pass --destination"
            )
        })
}

fn copy_dir_contents(source: &Path, destination: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn write_maven_metadata(
    repository: &Path,
    descriptor: &PublishDescriptor,
    file_name: &str,
    checksums: bool,
) -> Result<()> {
    let artifact_dir = repository
        .join(descriptor.coordinates.group.replace('.', "/"))
        .join(&descriptor.coordinates.id);
    fs::create_dir_all(&artifact_dir)?;
    let metadata_path = artifact_dir.join(file_name);
    let mut versions = metadata_path
        .exists()
        .then(|| fs::read_to_string(&metadata_path))
        .transpose()?
        .map(|text| maven_metadata_versions(&text))
        .unwrap_or_default();
    versions.insert(descriptor.coordinates.version.clone());
    let metadata = render_maven_metadata(descriptor, &versions)?;
    fs::write(&metadata_path, metadata)?;
    if checksums {
        write_checksums(&metadata_path)?;
    }
    Ok(())
}

fn maven_metadata_versions(text: &str) -> BTreeSet<String> {
    let mut versions = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("<version>") {
        rest = &rest[start + "<version>".len()..];
        let Some(end) = rest.find("</version>") else {
            break;
        };
        let version = rest[..end].trim();
        if !version.is_empty() {
            versions.insert(version.to_string());
        }
        rest = &rest[end + "</version>".len()..];
    }
    versions
}

fn render_maven_metadata(
    descriptor: &PublishDescriptor,
    versions: &BTreeSet<String>,
) -> Result<String> {
    let version = &descriptor.coordinates.version;
    let last_updated = maven_last_updated_timestamp()?;
    let mut rendered_versions = String::new();
    for version in versions {
        rendered_versions.push_str(&format!(
            "\n      <version>{}</version>",
            xml_escape(version)
        ));
    }
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>{}</groupId>
  <artifactId>{}</artifactId>
  <versioning>
    <latest>{}</latest>
    <release>{}</release>
    <versions>{}
    </versions>
    <lastUpdated>{}</lastUpdated>
  </versioning>
</metadata>
"#,
        xml_escape(&descriptor.coordinates.group),
        xml_escape(&descriptor.coordinates.id),
        xml_escape(version),
        xml_escape(version),
        rendered_versions,
        last_updated
    ))
}

fn maven_last_updated_timestamp() -> Result<String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    Ok(format!(
        "{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}"
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m as u32, d as u32)
}

fn serve_maven_repository(repository: &Path, port: u16) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("failed to bind Maven repository server on port {port}"))?;
    let address = listener.local_addr()?;
    println!(
        "serving Maven repository at http://{}/ from {}",
        address,
        repository.display()
    );
    std::io::stdout().flush()?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_maven_repository_request(stream, repository) {
                    eprintln!("Maven repository request failed: {err}");
                }
            }
            Err(err) => eprintln!("Maven repository connection failed: {err}"),
        }
    }
    Ok(())
}

fn handle_maven_repository_request(mut stream: TcpStream, repository: &Path) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or("/");
    if method != "GET" && method != "HEAD" {
        write_http_response(&mut stream, 405, "Method Not Allowed", b"")?;
        return Ok(());
    }
    let Some(path) = maven_repository_request_path(repository, target) else {
        write_http_response(&mut stream, 404, "Not Found", b"")?;
        return Ok(());
    };
    if !path.is_file() {
        write_http_response(&mut stream, 404, "Not Found", b"")?;
        return Ok(());
    }
    let body = if method == "HEAD" {
        Vec::new()
    } else {
        fs::read(&path)?
    };
    write_http_response(&mut stream, 200, "OK", &body)?;
    Ok(())
}

fn maven_repository_request_path(repository: &Path, target: &str) -> Option<PathBuf> {
    let path = target.split('?').next().unwrap_or(target);
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        return None;
    }
    let mut relative = PathBuf::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains('\\') {
            return None;
        }
        relative.push(segment);
    }
    Some(repository.join(relative))
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &[u8],
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
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
    if cmd.publish || cmd.dry_run {
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
    if exact.exists() {
        return exact;
    }
    if let Some(candidate) = resolve_publish_main_fqn(base_dir, main) {
        return candidate;
    }
    if raw.extension().is_some() {
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

fn resolve_publish_main_fqn(base_dir: &Path, main: &str) -> Option<PathBuf> {
    if !is_java_fqn(main) {
        return None;
    }
    let (package_name, class_name) = main.rsplit_once('.')?;
    let package_declaration = format!("package {package_name};");
    let class_declaration = format!("class {class_name}");
    let public_class_declaration = format!("public class {class_name}");
    for entry in walkdir::WalkDir::new(base_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("java") {
            continue;
        }
        if path.file_stem().and_then(|stem| stem.to_str()) != Some(class_name) {
            continue;
        }
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        if source.contains(&package_declaration)
            && (source.contains(&public_class_declaration) || source.contains(&class_declaration))
        {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn is_java_fqn(value: &str) -> bool {
    value.split('.').filter(|part| !part.is_empty()).count() >= 2
        && value.split('.').all(is_java_identifier)
}

fn publish_main_hint(path: &Path) -> String {
    if path.extension().is_some() {
        String::new()
    } else {
        format!(
            " (also checked {}.java, {}.jsh, {}.jav, and Java FQN matches under the descriptor directory)",
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
    let repo_dir = prepare_publish_repository(descriptor, cmd)?;
    let target_dir = cmd
        .target_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/jbx-publish"));
    let prefix = format!(
        "{}-{}",
        descriptor.coordinates.id, descriptor.coordinates.version
    );
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

fn prepare_publish_repository(
    descriptor: &PublishDescriptor,
    cmd: &PublishCommand,
) -> Result<PathBuf> {
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
    let (docs_markdown, docs_json) = publish_docs_outputs(descriptor, &staged)?;
    let docs_md = artifact_dir.join(format!("{prefix}-jbx-docs.md"));
    fs::write(&docs_md, docs_markdown)?;
    let docs_json_path = artifact_dir.join(format!("{prefix}-jbx-docs.json"));
    fs::write(&docs_json_path, docs_json)?;
    for file in [
        &jar,
        &sources_jar,
        &javadoc_jar,
        &pom,
        &docs_md,
        &docs_json_path,
    ] {
        write_checksums(file)?;
        if !cmd.skip_signing {
            write_gpg_signature(file, cmd.gpg_key.as_deref())?;
        }
    }
    Ok(repo_dir)
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
        .map(|description| format!("\n  <description>{}</description>", xml_escape(description)))
        .unwrap_or_default();
    let url = descriptor
        .url
        .as_deref()
        .map(|url| format!("\n  <url>{}</url>", xml_escape(url)))
        .unwrap_or_default();
    let dependencies = render_pom_dependencies(&descriptor.deps)?;
    let licenses = render_pom_licenses(&descriptor.licenses);
    let developers = render_pom_developers(&descriptor.developers);
    let scm = descriptor
        .scm
        .as_ref()
        .map(render_pom_scm)
        .unwrap_or_default();
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>{}</groupId>
  <artifactId>{}</artifactId>
  <version>{}</version>
  <packaging>jar</packaging>
  <name>{}</name>{}{}{}{}{}{}
</project>
"#,
        xml_escape(&descriptor.coordinates.group),
        xml_escape(&descriptor.coordinates.id),
        xml_escape(&descriptor.coordinates.version),
        xml_escape(name),
        description,
        url,
        licenses,
        developers,
        scm,
        dependencies
    ))
}

fn render_pom_licenses(licenses: &[PublishLicense]) -> String {
    if licenses.is_empty() {
        return String::new();
    }
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
    if developers.is_empty() {
        return String::new();
    }
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
    let java = jbx::jdk::java_bin_path(&jdk_root);
    let root = cache_root(cmd.cache_dir.as_deref())?.join("check");

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

    let repos = maven_tool::maven_repositories(&directives.repos);
    let cache_dir = cache_root(cmd.cache_dir.as_deref())?.join("deps");
    let mut wrapper_classpath = jbx::resolver::resolve_classpath(
        &[JBX_CHECK_COMPILER_COORDINATE.to_string()],
        &repos,
        &cache_dir,
    )?;
    if wrapper_classpath.is_empty() {
        anyhow::bail!("no JARs resolved for {JBX_CHECK_COMPILER_COORDINATE}");
    }
    if !cmd.no_error_prone {
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
    command.arg(JBX_CHECK_COMPILER_MAIN_CLASS);
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
const JBX_CHECK_COMPILER_COORDINATE: &str = "dev.telegraphic.jbx:jbx-check:0.1.0";
const JBX_CHECK_COMPILER_MAIN_CLASS: &str = "dev.telegraphic.jbx.check.JbxCheckCompiler";

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
        Some(Commands::Install(cmd)) => run_install(cmd)?,
        Some(Commands::Docs(cmd)) => run_docs(cmd)?,
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
    fn javadoc_type_description_is_extracted_and_rendered() {
        let html = r#"
            <div class="description">
              <pre>public class <span class="typeNameLabel">Example</span></pre>
              <div class="block"><p>Example reads and writes JSON.
                It supports <code>tree</code> values.</p></div>
            </div>
            <div class="member-signature">public void run()</div>
        "#;
        let ty = parse_javadoc_type_page("com/example/Example.html", html).unwrap();
        assert_eq!(
            ty["description"],
            "Example reads and writes JSON. It supports `tree` values."
        );

        let markdown = render_docs_markdown("example", None, &[], &[ty]).unwrap();
        assert!(
            markdown.contains("Example reads and writes JSON. It supports `tree` values."),
            "{markdown}"
        );
    }

    #[test]
    fn javadoc_full_signatures_parse_fields_constructors_methods_and_parameters() {
        let html = r#"
            <div class="description">
              <pre>public class <span class="typeNameLabel">Example</span></pre>
              <div class="block">Useful example.
                <pre>Example example = new Example("demo", Map.of());
String value = example.readValue("{}", String.class);</pre>
              </div>
            </div>
            <ul class="blockList">
              <li class="blockList">
                <h4>COUNT</h4>
                <pre>public static final&nbsp;int&nbsp;COUNT</pre>
              </li>
              <li class="blockList">
                <h4>Example</h4>
                <pre>public&nbsp;Example( String &nbsp;name,
       java.util.Map&lt;String, Object&gt;&nbsp;options)
        throws IOException</pre>
              </li>
              <li class="blockList">
                <h4>readValue</h4>
                <pre>public final&nbsp;&lt;T&gt;&nbsp;T&nbsp;readValue( String &nbsp;content,
       Class &lt;T&gt;&nbsp;valueType,
       String...&nbsp;features)
        throws IOException,
               IllegalArgumentException</pre>
              </li>
            </ul>
        "#;
        let ty = parse_javadoc_type_page("com/example/Example.html", html).unwrap();

        assert_eq!(ty["fields"][0]["name"], "COUNT");
        assert_eq!(ty["fields"][0]["type"], "int");
        assert!(ty["description"].as_str().unwrap().contains("```"));
        assert!(ty["description"]
            .as_str()
            .unwrap()
            .contains("example.readValue"));
        assert_eq!(ty["examples"].as_array().unwrap().len(), 1);
        assert!(ty["examples"][0]
            .as_str()
            .unwrap()
            .contains("example.readValue"));

        assert_eq!(ty["constructors"][0]["name"], "Example");
        assert_eq!(ty["constructors"][0]["parameters"][0]["name"], "name");
        assert_eq!(ty["constructors"][0]["parameters"][0]["type"], "String");
        assert_eq!(ty["constructors"][0]["parameters"][1]["name"], "options");
        assert_eq!(
            ty["constructors"][0]["parameters"][1]["type"],
            "java.util.Map<String, Object>"
        );
        assert_eq!(ty["constructors"][0]["throws"][0], "IOException");

        assert_eq!(ty["methods"][0]["name"], "readValue");
        assert_eq!(ty["methods"][0]["modifiers"][0], "public");
        assert_eq!(ty["methods"][0]["modifiers"][1], "final");
        assert_eq!(ty["methods"][0]["returnType"], "T");
        assert_eq!(ty["methods"][0]["parameters"][0]["name"], "content");
        assert_eq!(ty["methods"][0]["parameters"][1]["name"], "valueType");
        assert_eq!(ty["methods"][0]["parameters"][1]["type"], "Class<T>");
        assert_eq!(ty["methods"][0]["parameters"][2]["name"], "features");
        assert_eq!(ty["methods"][0]["parameters"][2]["type"], "String...");
        assert_eq!(ty["methods"][0]["throws"][1], "IllegalArgumentException");
    }

    #[test]
    fn markdown_renders_type_members_for_agent_context() {
        let ty = serde_json::json!({
            "kind": "class",
            "name": "Example",
            "qualifiedName": "com.example.Example",
            "description": "Useful example.",
            "examples": ["Example example = new Example(\"demo\");\nString value = example.readValue(\"{}\");"],
            "fields": [{"name": "COUNT", "type": "int", "modifiers": ["public", "static", "final"]}],
            "constructors": [{"name": "Example", "parameters": [{"name": "name", "type": "String"}], "throws": ["IOException"]}],
            "methods": [{"name": "readValue", "returnType": "T", "parameters": [{"name": "content", "type": "String"}], "throws": ["IOException"]}]
        });
        let markdown = render_docs_markdown("example", None, &[], &[ty]).unwrap();

        assert!(markdown.contains("### Examples"), "{markdown}");
        assert!(
            markdown.contains("String value = example.readValue"),
            "{markdown}"
        );
        assert!(markdown.contains("### Fields"), "{markdown}");
        assert!(
            markdown.contains("- `public static final int COUNT`"),
            "{markdown}"
        );
        assert!(markdown.contains("### Constructors"), "{markdown}");
        assert!(
            markdown.contains("- `Example(String name) throws IOException`"),
            "{markdown}"
        );
        assert!(markdown.contains("### Methods"), "{markdown}");
        assert!(
            markdown.contains("- `T readValue(String content) throws IOException`"),
            "{markdown}"
        );
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
