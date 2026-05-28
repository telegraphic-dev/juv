use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn juv_command() -> Command {
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

fn path_exists_under(root: &Path, suffix: &str) -> bool {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| entry.path().to_string_lossy().ends_with(suffix))
}

#[test]
fn build_compiles_script_into_cache_without_running_it() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let marker = tmp.path().join("should-not-exist.txt");
    let src = tmp.path().join("BuildOnly.java");
    fs::write(
        &src,
        format!(
            r#"
class BuildOnly {{
  public static void main(String[] args) throws Exception {{
    java.nio.file.Files.writeString(java.nio.file.Path.of("{}"), "ran");
  }}
}}
"#,
            marker.display()
        ),
    )
    .unwrap();

    let out = juv_command()
        .arg("build")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    assert!(!marker.exists(), "build must compile, not run the script");
    assert!(
        path_exists_under(&cache, "/classes/BuildOnly.class"),
        "expected compiled class under cache, got: {:?}",
        fs::read_dir(&cache).map(|it| it.count())
    );
}

#[test]
fn build_compiles_sources_and_copies_files_resources() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let main = tmp.path().join("Main.java");
    let helper = tmp.path().join("Helper.java");
    let resource = tmp.path().join("app.properties");
    fs::write(
        &helper,
        r#"
class Helper {
  static String message() { return "helper"; }
}
"#,
    )
    .unwrap();
    fs::write(&resource, "answer=42\n").unwrap();
    fs::write(
        &main,
        r#"
//SOURCES Helper.java
//FILES config/app.properties=app.properties
class Main {
  static String compileCheck() { return Helper.message(); }
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("build")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&main)
        .output()
        .unwrap();

    assert_success(&out);
    assert!(path_exists_under(&cache, "/classes/Main.class"));
    assert!(path_exists_under(&cache, "/classes/Helper.class"));
    assert!(path_exists_under(&cache, "/classes/config/app.properties"));
}
