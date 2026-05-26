use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

use doj::{
    build_java, cache_entries, clear_cache, default_cache_dir, init_script, run_java,
    split_directive_words, BuildOptions, InitOptions, KeyValue, RunOptions,
};

#[derive(Parser, Debug)]
#[command(name = "doj", version, about = "do Java: a Rust port of JBang")]
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
    /// Print parsed JBang directives.
    Info(InfoCommand),
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
    /// Clear the doj cache directory.
    Clear(CacheClearCommand),
    /// Print the effective doj cache directory.
    Path(CachePathCommand),
    /// List cached script entries.
    List(CacheListCommand),
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
    /// Print the effective doj cache directory.
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

fn key_values_json(values: &[doj::KeyValue]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|kv| serde_json::json!({ "key": kv.key, "value": kv.value }))
            .collect(),
    )
}

fn docs_json(values: &[doj::KeyValue]) -> serde_json::Value {
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

fn print_required(value: Option<&str>, missing: &str) -> Result<()> {
    let Some(value) = value else {
        anyhow::bail!("{missing}");
    };
    println!("{value}");
    Ok(())
}

fn parsed_directives(script: &PathBuf) -> Result<doj::Directives> {
    let source = fs::read_to_string(script)?;
    Ok(doj::parse_directives(&source))
}

fn print_cache_path(cache_dir: Option<PathBuf>) -> Result<()> {
    let cache_dir = match cache_dir {
        Some(path) => path,
        None => default_cache_dir()?,
    };
    println!("{}", cache_dir.display());
    Ok(())
}

fn tools_payload(script: &std::path::Path, output: &doj::BuildOutput) -> serde_json::Value {
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
        Some(Commands::Run(cmd)) => run_java(RunOptions {
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
        })?,
        Some(Commands::Build(cmd)) => {
            build_java(BuildOptions {
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
            })?;
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
                let directives = doj::parse_directives(&source);
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
                let main = doj::parse_directives(&source)
                    .main_class
                    .or_else(|| doj::infer_main_class_from_source(&cmd.script, &source));
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
                println!("{:#?}", doj::parse_directives(&source));
                0
            }
        },
        None => {
            let Some(script) = cli.script else {
                eprintln!("No script specified. Try: doj run Hello.java");
                std::process::exit(2);
            };
            run_java(RunOptions {
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
            })?
        }
    };
    std::process::exit(code);
}
