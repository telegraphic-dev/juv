use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use doj::{
    build_java, clear_cache, init_script, run_java, split_directive_words, BuildOptions,
    InitOptions, RunOptions,
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

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(long = "javac-option")]
    javac_options: Vec<String>,

    /// Additional java runtime option.
    #[arg(long = "runtime-option")]
    runtime_options: Vec<String>,

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

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(long = "javac-option")]
    javac_options: Vec<String>,

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
}

#[derive(Parser, Debug)]
struct CacheClearCommand {
    /// Override cache directory.
    #[arg(long = "cache-dir")]
    cache_dir: Option<PathBuf>,
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

    /// Additional classpath entries.
    #[arg(long = "class-path", alias = "cp")]
    classpath: Vec<PathBuf>,

    /// Additional javac option.
    #[arg(long = "javac-option")]
    javac_options: Vec<String>,

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
struct InfoDirectivesCommand {
    /// Java source file.
    script: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Some(Commands::Run(cmd)) => run_java(RunOptions {
            script: cmd.script,
            script_args: cmd.args,
            extra_deps: cmd.deps,
            classpath: cmd.classpath,
            javac_options: cmd.javac_options,
            runtime_options: cmd.runtime_options,
            main_class: cmd.main_class,
            cache_dir: cmd.cache_dir,
        })?,
        Some(Commands::Build(cmd)) => {
            build_java(BuildOptions {
                script: cmd.script,
                extra_deps: cmd.deps,
                classpath: cmd.classpath,
                javac_options: cmd.javac_options,
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
        },
        Some(Commands::Info(cmd)) => match cmd.command {
            InfoSubcommand::Classpath(cmd) => {
                let output = build_java(BuildOptions {
                    script: cmd.script,
                    extra_deps: cmd.deps,
                    classpath: cmd.classpath,
                    javac_options: cmd.javac_options,
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
                classpath: Vec::new(),
                javac_options: Vec::new(),
                runtime_options: Vec::new(),
                main_class: None,
                cache_dir: None,
            })?
        }
    };
    std::process::exit(code);
}
