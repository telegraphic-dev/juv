use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

#[derive(Debug, Clone)]
pub struct JuvxOptions {
    pub coordinate: String,
    pub repos: Vec<String>,
    pub cache_dir: Option<PathBuf>,
    pub main_class: Option<String>,
    pub args: Vec<String>,
}

pub fn maven_repositories(repo_args: &[String]) -> Vec<crate::resolver::Repository> {
    let mut repos = vec![crate::resolver::Repository::central()];
    for repo in repo_args {
        if repo == "central" || repo == "mavenCentral" {
            continue;
        }
        if let Some((id, url)) = repo.split_once('=') {
            repos.push(crate::resolver::Repository {
                id: id.to_string(),
                url: url.to_string(),
            });
        } else {
            repos.push(crate::resolver::Repository {
                id: repo.clone(),
                url: repo.clone(),
            });
        }
    }
    repos
}

fn primary_jar_name(coordinate: &str) -> Result<String> {
    let dep = crate::resolver::parse_coordinate(coordinate)?;
    Ok(match dep.classifier {
        Some(classifier) => format!("{}-{}-{classifier}.jar", dep.module.name, dep.version),
        None => format!("{}-{}.jar", dep.module.name, dep.version),
    })
}

fn resolve_juvx_coordinate(
    coordinate: &str,
    repos: &[crate::resolver::Repository],
) -> Result<String> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    if parts.len() != 2 {
        crate::resolver::parse_coordinate(coordinate)?;
        return Ok(coordinate.to_string());
    }
    let module = crate::resolver::Module {
        org: parts[0].to_string(),
        name: parts[1].to_string(),
    };
    let version = crate::resolver::resolve_latest_version(&module, repos)?;
    Ok(format!("{}:{}:{version}", module.org, module.name))
}

pub fn run(options: JuvxOptions) -> Result<i32> {
    let cache_dir = match options.cache_dir {
        Some(path) => path,
        None => crate::default_cache_dir()?.join("deps"),
    };
    let repos = maven_repositories(&options.repos);
    let requested_coordinate = options.coordinate;
    let coordinate = resolve_juvx_coordinate(&requested_coordinate, &repos)?;
    let coordinates = vec![coordinate.clone()];
    let classpath = crate::resolver::resolve_classpath(&coordinates, &repos, &cache_dir)?;
    if classpath.is_empty() {
        return Err(anyhow!("no JARs resolved for {coordinate}"));
    }

    let mut java = ProcessCommand::new("java");
    if let Some(main_class) = options.main_class {
        java.arg("-cp")
            .arg(std::env::join_paths(&classpath)?)
            .arg(main_class);
    } else {
        let jar_name = primary_jar_name(&coordinate)?;
        let primary_jar = classpath
            .iter()
            .find(|path| {
                path.file_name()
                    .is_some_and(|name| name == jar_name.as_str())
            })
            .ok_or_else(|| anyhow!("resolved classpath did not contain primary JAR {jar_name}"))?;
        java.arg("-jar").arg(primary_jar);
    }
    java.args(options.args);
    let status = java.status().context("failed to launch java")?;
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return Ok(128 + signal);
        }
    }
    Ok(status.code().unwrap_or(1))
}
