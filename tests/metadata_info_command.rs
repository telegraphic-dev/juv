use std::fs;
use std::process::{Command, Output};

fn doj_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_doj"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn fixture_script() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("README.md");
    fs::write(&docs, "# Docs\n").unwrap();
    fs::write(tmp.path().join("Helper.java"), "class Helper {}\n").unwrap();
    fs::write(tmp.path().join("Extra.java"), "class Extra {}\n").unwrap();
    fs::write(tmp.path().join("app.properties"), "answer=42\n").unwrap();
    let src = tmp.path().join("MetaMain.java");
    fs::write(
        &src,
        r#"
//DESCRIPTION First line
//DESCRIPTION Second line
//GAV dev.telegraphic:doj-demo:1.2.3
//MODULE dev.telegraphic.demo
//DOCS README.md
//DOCS api=https://example.com/api
//MANIFEST Main-Class=MetaMain
//JAVAAGENT org.example:agent:1.0=debug=true
//NATIVE_OPTIONS --no-fallback "--initialize-at-build-time=com.acme.Foo"
//CDS
//NOINTEGRATIONS
//JAVA 21
//DEPS Helper.java
//REPOS central=https://repo1.maven.org/maven2
//SOURCES Extra.java
//FILES config/app.properties=app.properties
//JAVAC_OPTIONS -Xlint:unchecked
//RUNTIME_OPTIONS -Ddemo=true
class MetaMain {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    (tmp, src)
}

#[test]
fn directives_parse_broad_jbang_metadata() {
    let (_tmp, src) = fixture_script();

    let out = doj_command()
        .arg("info")
        .arg("directives")
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("gav: Some(\n        \"dev.telegraphic:doj-demo:1.2.3\""),
        "{stdout}"
    );
    assert!(
        stdout.contains("module: Some(\n        \"dev.telegraphic.demo\""),
        "{stdout}"
    );
    assert!(stdout.contains("enable_cds: true"), "{stdout}");
    assert!(stdout.contains("disable_integrations: true"), "{stdout}");
    assert!(stdout.contains("native_options"), "{stdout}");
    assert!(
        stdout.contains("--initialize-at-build-time=com.acme.Foo"),
        "{stdout}"
    );
    assert!(stdout.contains("manifest_options"), "{stdout}");
    assert!(stdout.contains("java_agents"), "{stdout}");
    assert!(stdout.contains("docs"), "{stdout}");
}

#[test]
fn info_tools_prints_json_and_selects_fields() {
    let (_tmp, src) = fixture_script();
    let cache = tempfile::tempdir().unwrap();

    let out = doj_command()
        .arg("info")
        .arg("tools")
        .arg("--cache-dir")
        .arg(cache.path())
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["originalResource"], src.to_string_lossy().as_ref());
    assert_eq!(json["mainClass"], "MetaMain");
    assert_eq!(json["gav"], "dev.telegraphic:doj-demo:1.2.3");
    assert_eq!(json["module"], "dev.telegraphic.demo");
    assert_eq!(json["description"], "First line\nSecond line");
    assert_eq!(json["javaVersion"], "21");
    assert_eq!(json["requestedJavaVersion"], "21");
    assert_eq!(json["dependencies"][0], "Helper.java");
    assert_eq!(json["repositories"][0]["id"], "central");
    assert_eq!(
        json["repositories"][0]["url"],
        "https://repo1.maven.org/maven2"
    );
    assert_eq!(json["compileOptions"][0], "-Xlint:unchecked");
    assert_eq!(json["runtimeOptions"][0], "-Ddemo=true");
    assert!(json["applicationClassesDir"]
        .as_str()
        .unwrap()
        .ends_with("/classes"));
    assert_eq!(json["docs"]["main"][0]["originalResource"], "README.md");
    assert_eq!(
        json["docs"]["api"][0]["originalResource"],
        "https://example.com/api"
    );

    let selected = doj_command()
        .arg("info")
        .arg("tools")
        .arg("--select")
        .arg("mainClass")
        .arg("--cache-dir")
        .arg(cache.path())
        .arg(&src)
        .output()
        .unwrap();
    assert_success(&selected);
    assert_eq!(String::from_utf8_lossy(&selected.stdout).trim(), "MetaMain");
}

#[test]
fn info_docs_prints_description_and_doc_targets() {
    let (_tmp, src) = fixture_script();

    let out = doj_command()
        .arg("info")
        .arg("docs")
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("First line\nSecond line"), "{stdout}");
    assert!(stdout.contains("main:"), "{stdout}");
    assert!(stdout.contains("README.md"), "{stdout}");
    assert!(stdout.contains("api:"), "{stdout}");
    assert!(stdout.contains("https://example.com/api"), "{stdout}");
}

#[test]
fn info_cache_prints_effective_cache_dir() {
    let cache = tempfile::tempdir().unwrap();

    let out = doj_command()
        .arg("info")
        .arg("cache")
        .arg("--cache-dir")
        .arg(cache.path())
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        cache.path().to_string_lossy()
    );
}
