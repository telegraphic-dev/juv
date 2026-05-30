use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn jbx_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_project(root: &std::path::Path) {
    fs::write(
        root.join("Hello.java"),
        r#"
public class Hello {
  public static String message() { return "hello"; }
  public static void main(String[] args) { System.out.println(message()); }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("jbx.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "hello-tool",
  "version": "1.0.0",
  "java": "21",
  "package": "dev.telegraphic.demo.hello",
  "description": "Hello tool",
  "url": "https://github.com/telegraphic-dev/hello-tool",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic", "organizationUrl": "https://github.com/telegraphic-dev"}],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/hello-tool.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/hello-tool.git",
    "url": "https://github.com/telegraphic-dev/hello-tool"
  }
}
"#,
    )
    .unwrap();
}

fn write_minimal_project(root: &std::path::Path) {
    fs::write(
        root.join("Hello.java"),
        r#"
public class Hello {
  public static void main(String[] args) { System.out.println("hello"); }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("jbx.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "minimal-tool",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo.minimal"
}
"#,
    )
    .unwrap();
}

#[test]
fn install_writes_current_project_to_selected_maven_repository_layout() {
    let tmp = tempfile::tempdir().unwrap();
    write_project(tmp.path());
    let destination = tmp.path().join("repo");

    let out = jbx_command()
        .arg("install")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
        .arg("--destination")
        .arg(&destination)
        .arg("--target-dir")
        .arg(tmp.path().join("install-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let base = destination.join("dev/telegraphic/demo/hello-tool/1.0.0");
    assert!(base.join("hello-tool-1.0.0.jar").is_file());
    assert!(base.join("hello-tool-1.0.0-sources.jar").is_file());
    assert!(base.join("hello-tool-1.0.0-javadoc.jar").is_file());
    assert!(base.join("hello-tool-1.0.0.pom").is_file());
    let metadata = fs::read_to_string(
        destination.join("dev/telegraphic/demo/hello-tool/maven-metadata-local.xml"),
    )
    .unwrap();
    assert!(metadata.contains("<latest>1.0.0</latest>"), "{metadata}");
    assert!(metadata.contains("<release>1.0.0</release>"), "{metadata}");
    assert!(metadata.contains("<version>1.0.0</version>"), "{metadata}");
    assert!(String::from_utf8_lossy(&out.stdout).contains(base.to_string_lossy().as_ref()));
}

#[test]
fn install_merges_existing_local_maven_metadata_versions() {
    let tmp = tempfile::tempdir().unwrap();
    write_project(tmp.path());
    let destination = tmp.path().join("repo");
    let metadata_dir = destination.join("dev/telegraphic/demo/hello-tool");
    fs::create_dir_all(&metadata_dir).unwrap();
    fs::write(
        metadata_dir.join("maven-metadata-local.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>dev.telegraphic.demo</groupId>
  <artifactId>hello-tool</artifactId>
  <versioning>
    <latest>0.9.0</latest>
    <release>0.9.0</release>
    <versions>
      <version>0.9.0</version>
    </versions>
    <lastUpdated>20250102030405</lastUpdated>
  </versioning>
</metadata>
"#,
    )
    .unwrap();

    let out = jbx_command()
        .arg("install")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
        .arg("--destination")
        .arg(&destination)
        .arg("--target-dir")
        .arg(tmp.path().join("install-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let metadata = fs::read_to_string(metadata_dir.join("maven-metadata-local.xml")).unwrap();
    assert!(metadata.contains("<latest>1.0.0</latest>"), "{metadata}");
    assert!(metadata.contains("<release>1.0.0</release>"), "{metadata}");
    assert!(metadata.contains("<version>0.9.0</version>"), "{metadata}");
    assert!(metadata.contains("<version>1.0.0</version>"), "{metadata}");
}

#[test]
fn install_accepts_minimal_non_central_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    write_minimal_project(tmp.path());
    let destination = tmp.path().join("repo");

    let out = jbx_command()
        .arg("install")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
        .arg("--destination")
        .arg(&destination)
        .arg("--target-dir")
        .arg(tmp.path().join("install-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let pom = fs::read_to_string(
        destination.join("dev/telegraphic/demo/minimal-tool/1.0.0/minimal-tool-1.0.0.pom"),
    )
    .unwrap();
    assert!(
        pom.contains("<artifactId>minimal-tool</artifactId>"),
        "{pom}"
    );
    assert!(!pom.contains("<licenses>"), "{pom}");
}

#[test]
fn install_defaults_to_current_project_and_maven_local_repository() {
    let tmp = tempfile::tempdir().unwrap();
    write_project(tmp.path());
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let out = jbx_command()
        .arg("install")
        .arg("--target-dir")
        .arg(tmp.path().join("install-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .env("HOME", &home)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let base = home.join(".m2/repository/dev/telegraphic/demo/hello-tool/1.0.0");
    assert!(base.join("hello-tool-1.0.0.jar").is_file());
    assert!(base.join("hello-tool-1.0.0.pom").is_file());
}

#[test]
fn publish_serve_exposes_prepared_artifacts_over_maven_repository_layout() {
    let tmp = tempfile::tempdir().unwrap();
    write_project(tmp.path());
    let mut child = jbx_command()
        .arg("publish")
        .arg("--serve")
        .arg("0")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
        .arg("--target-dir")
        .arg(tmp.path().join("serve-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("serving Maven repository at"), "{line}");
    let url = line
        .split_whitespace()
        .find(|part| part.starts_with("http://"))
        .unwrap()
        .trim_end_matches('/')
        .to_string();

    let pom_url = format!("{url}/dev/telegraphic/demo/hello-tool/1.0.0/hello-tool-1.0.0.pom");
    let metadata_url = format!("{url}/dev/telegraphic/demo/hello-tool/maven-metadata.xml");
    let metadata_sha1_url = format!("{metadata_url}.sha1");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut pom = None;
    let mut metadata = None;
    let mut metadata_sha1 = None;
    while Instant::now() < deadline {
        if pom.is_none() {
            if let Ok(response) = ureq::get(&pom_url).call() {
                if response.status() == 200 {
                    pom = Some(response.into_string().unwrap());
                }
            }
        }
        if metadata.is_none() {
            if let Ok(response) = ureq::get(&metadata_url).call() {
                if response.status() == 200 {
                    metadata = Some(response.into_string().unwrap());
                }
            }
        }
        if metadata_sha1.is_none() {
            if let Ok(response) = ureq::get(&metadata_sha1_url).call() {
                if response.status() == 200 {
                    metadata_sha1 = Some(response.into_string().unwrap());
                }
            }
        }
        if pom.is_some() && metadata.is_some() && metadata_sha1.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let pom = pom.unwrap_or_else(|| panic!("failed to fetch {pom_url}"));
    assert!(
        pom.contains("<groupId>dev.telegraphic.demo</groupId>"),
        "{pom}"
    );
    assert!(pom.contains("<artifactId>hello-tool</artifactId>"), "{pom}");
    let metadata = metadata.unwrap_or_else(|| panic!("failed to fetch {metadata_url}"));
    assert!(metadata.contains("<latest>1.0.0</latest>"), "{metadata}");
    assert!(metadata.contains("<release>1.0.0</release>"), "{metadata}");
    assert!(metadata.contains("<version>1.0.0</version>"), "{metadata}");
    let metadata_sha1 =
        metadata_sha1.unwrap_or_else(|| panic!("failed to fetch {metadata_sha1_url}"));
    assert_eq!(metadata_sha1.trim().len(), 40, "{metadata_sha1}");

    let run = jbx_command()
        .arg("--repo")
        .arg(&url)
        .arg("dev.telegraphic.demo:hello-tool:1.0.0")
        .arg("--cache-dir")
        .arg(tmp.path().join("run-cache"))
        .output()
        .unwrap();
    assert_success(&run);
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "hello");
    kill_child(&mut child);
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
