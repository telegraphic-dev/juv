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

fn assert_failure(out: &Output) {
    assert!(
        !out.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn metadata_script(dir: &std::path::Path) -> std::path::PathBuf {
    fs::write(dir.join("Helper.java"), "class Helper {}\n").unwrap();
    fs::write(dir.join("Extra.java"), "class Extra {}\n").unwrap();
    fs::write(dir.join("app.properties"), "answer=42\n").unwrap();
    let src = dir.join("BatchMain.java");
    fs::write(
        &src,
        r#"//DESCRIPTION Batch demo
//JAVA 25
//MAIN BatchMain
//GAV dev.telegraphic:batch:1.0
//MODULE dev.telegraphic.batch
//DEPS Helper.java
//REPOS central=https://repo1.maven.org/maven2
//SOURCES Extra.java
//FILES config/app.properties=app.properties
class BatchMain {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    src
}

#[test]
fn info_field_commands_print_single_metadata_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let src = metadata_script(tmp.path());

    for (field, expected) in [
        ("main", "BatchMain"),
        ("java", "25"),
        ("description", "Batch demo"),
        ("gav", "dev.telegraphic:batch:1.0"),
        ("module", "dev.telegraphic.batch"),
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
}

#[test]
fn info_collection_commands_print_directive_collections_one_per_line() {
    let tmp = tempfile::tempdir().unwrap();
    let src = metadata_script(tmp.path());

    for (field, expected) in [
        ("deps", "Helper.java"),
        ("repos", "central=https://repo1.maven.org/maven2"),
        ("sources", "Extra.java"),
        ("files", "config/app.properties=app.properties"),
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
}

#[test]
fn missing_optional_single_info_field_fails_with_clear_error() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("Plain.java");
    fs::write(
        &src,
        "class Plain { public static void main(String[] args) {} }\n",
    )
    .unwrap();

    let out = juv_command()
        .arg("info")
        .arg("gav")
        .arg(&src)
        .output()
        .unwrap();

    assert_failure(&out);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no //GAV directive found"),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn cache_path_prints_default_or_overridden_cache_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache-root");

    let out = juv_command()
        .arg("cache")
        .arg("path")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        cache.to_string_lossy()
    );
}

#[test]
fn cache_list_lists_cached_script_entries_after_build() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache-root");
    let src = tmp.path().join("ListMe.java");
    fs::write(
        &src,
        "class ListMe { public static void main(String[] args) {} }\n",
    )
    .unwrap();

    let build = juv_command()
        .arg("build")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&src)
        .output()
        .unwrap();
    assert_success(&build);

    let out = juv_command()
        .arg("cache")
        .arg("list")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ListMe.java"), "stdout:\n{stdout}");
    assert!(stdout.contains("classes"), "stdout:\n{stdout}");
}

#[test]
fn cache_list_is_empty_when_cache_dir_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("missing-cache");

    let out = juv_command()
        .arg("cache")
        .arg("list")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "");
}
