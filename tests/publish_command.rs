use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn zip_names(path: &std::path::Path) -> Vec<String> {
    let file = fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut names = Vec::new();
    for i in 0..archive.len() {
        names.push(archive.by_index(i).unwrap().name().to_string());
    }
    names
}

fn zip_entry(path: &std::path::Path, name: &str) -> String {
    let file = fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
    text
}

#[test]
fn publish_dry_run_uses_juv_json_gav_object_and_version_override() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Hello.java");
    fs::write(
        &script,
        r#"
public class Hello {
  public static void main(String[] args) {
    System.out.println("hello");
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "gav": {
    "group": "dev.telegraphic.demo",
    "artifact": "hello-tool",
    "version": "1.0.0"
  },
  "package": "dev.telegraphic.demo.hello",
  "description": "Hello tool"
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--version")
        .arg("2.0.0")
        .arg("--output")
        .arg(&bundle)
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("dev.telegraphic.demo:hello-tool:2.0.0"),
        "{stdout}"
    );
    assert!(stdout.contains("dry run"), "{stdout}");

    let names = zip_names(&bundle);
    let base = "dev/telegraphic/demo/hello-tool/2.0.0";
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-sources.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-javadoc.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom.sha256")),
        "{names:?}"
    );

    let pom = zip_entry(&bundle, &format!("{base}/hello-tool-2.0.0.pom"));
    assert!(
        pom.contains("<groupId>dev.telegraphic.demo</groupId>"),
        "{pom}"
    );
    assert!(pom.contains("<artifactId>hello-tool</artifactId>"), "{pom}");
    assert!(pom.contains("<version>2.0.0</version>"), "{pom}");
    assert!(
        pom.contains("<description>Hello tool</description>"),
        "{pom}"
    );
}

#[test]
fn publish_requires_structured_gav_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Hello.java"), "void main() {}\n").unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{ "main": "Hello.java", "gav": "dev.telegraphic:bad:1.0.0" }"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("gav must be an object"), "{stderr}");
}
