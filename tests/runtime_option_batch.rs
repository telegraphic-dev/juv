use serde_json::Value;
use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "expected success\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn run_accepts_jbang_style_cli_overrides_for_java_sources_files_and_runtime_options() {
    let tmp = tempfile::tempdir().unwrap();
    let helper = tmp.path().join("Helper.java");
    let resource = tmp.path().join("message.txt");
    let src = tmp.path().join("CliOptions.java");

    fs::write(
        &helper,
        "class Helper { static String text() { return \"helper\"; } }\n",
    )
    .unwrap();
    fs::write(&resource, "resource").unwrap();
    fs::write(
        &src,
        r#"class CliOptions {
  public static void main(String[] args) throws Exception {
    String message = new String(CliOptions.class.getResourceAsStream("/data/message.txt").readAllBytes());
    System.out.println(Helper.text() + ":" + message + ":" + System.getProperty("juv.test"));
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("run")
        .arg("--source")
        .arg("Helper.java")
        .arg("--files")
        .arg("data/message.txt=message.txt")
        .arg("--java-option")
        .arg("-Djuv.test=ok")
        .arg(&src)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "helper:resource:ok"
    );
}

#[test]
fn build_accepts_java_repo_source_files_and_compile_option_overrides() {
    let tmp = tempfile::tempdir().unwrap();
    let helper = tmp.path().join("BuildHelper.java");
    let resource = tmp.path().join("build.txt");
    let src = tmp.path().join("BuildOptions.java");
    let cache = tmp.path().join("cache");

    fs::write(&helper, "class BuildHelper {}\n").unwrap();
    fs::write(&resource, "build resource").unwrap();
    fs::write(&src, "class BuildOptions { BuildHelper h; }\n").unwrap();

    let out = juv_command()
        .arg("build")
        .arg("--java")
        .arg("25")
        .arg("--repo")
        .arg("central=https://repo1.maven.org/maven2")
        .arg("--source")
        .arg("BuildHelper.java")
        .arg("--files")
        .arg("assets/build.txt=build.txt")
        .arg("--compile-option")
        .arg("-Xlint:none")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&src)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let listed = juv_command()
        .arg("cache")
        .arg("list")
        .arg("--json")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();
    assert_success(&listed);
    let entries: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let classes = entries[0]["classesDir"].as_str().unwrap();
    assert!(std::path::Path::new(classes)
        .join("BuildHelper.class")
        .exists());
    assert!(std::path::Path::new(classes)
        .join("assets/build.txt")
        .exists());
}

#[test]
fn info_exposes_option_collections_added_by_directives_and_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("InfoOptions.java");
    fs::write(
        &src,
        r#"//JAVAC_OPTIONS -Xlint:none
//JAVA_OPTIONS -Dfrom.directive=yes
//NATIVE_OPTIONS --no-fallback
//JAVAAGENT agent.jar=debug
//MANIFEST Main-Class=InfoOptions
class InfoOptions { public static void main(String[] args) {} }
"#,
    )
    .unwrap();

    for (field, expected) in [
        ("compile-options", "-Xlint:none"),
        ("runtime-options", "-Dfrom.directive=yes"),
        ("native-options", "--no-fallback"),
        ("javaagents", "agent.jar=debug"),
        ("manifest", "Main-Class=InfoOptions"),
    ] {
        let out = juv_command()
            .arg("info")
            .arg(field)
            .arg(&src)
            .output()
            .unwrap();
        assert_success(&out);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            expected,
            "field {field}"
        );
    }

    let tools = juv_command()
        .arg("info")
        .arg("tools")
        .arg("--java")
        .arg("25")
        .arg("--repo")
        .arg("custom=https://repo.example.invalid/maven")
        .arg("--source")
        .arg("InfoOptions.java")
        .arg("--files")
        .arg("data.txt=InfoOptions.java")
        .arg("--compile-option")
        .arg("-parameters")
        .arg("--java-option")
        .arg("-Dfrom.cli=yes")
        .arg(&src)
        .output()
        .unwrap();
    assert_success(&tools);
    let json: Value = serde_json::from_slice(&tools.stdout).unwrap();
    assert_eq!(json["requestedJavaVersion"], "25");
    assert!(json["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|repo| repo["id"] == "custom"));
    assert!(json["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "InfoOptions.java"));
    assert!(json["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "data.txt=InfoOptions.java"));
    assert!(json["compileOptions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "-parameters"));
    assert!(json["runtimeOptions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "-Dfrom.cli=yes"));
}
