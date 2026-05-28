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

#[test]
fn check_defaults_to_current_directory_and_reports_error_prone() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    fs::write(
        tmp.path().join("Example.java"),
        r#"
class Example {
  boolean same(String value) {
    return value == "x";
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("check")
        .arg("--java")
        .arg("21")
        .arg("--json")
        .arg("--cache-dir")
        .arg(&cache)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(payload["ok"], true);
    assert!(stdout.contains("ReferenceEquality"), "{stdout}");
    assert!(stdout.contains("Example.java"), "{stdout}");
}

#[test]
fn check_json_escapes_diagnostic_paths_and_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = tmp.path().join("quote\"backslash\\dir");
    fs::create_dir(&src).unwrap();
    let source = src.join("Weird\"Name.java");
    fs::write(
        &source,
        r#"
class WeirdName {
  String broken() {
    return "bad\q";
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("check")
        .arg("--java")
        .arg("21")
        .arg("--no-error-prone")
        .arg("--json")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&source)
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let diagnostic = payload["diagnostics"].as_array().unwrap().first().unwrap();
    assert!(
        diagnostic["file"]
            .as_str()
            .unwrap()
            .contains("Weird\"Name.java"),
        "{stdout}"
    );
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap()
            .contains("illegal escape character"),
        "{stdout}"
    );
}

#[test]
fn check_reports_invalid_wrapper_classpath_without_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache:with-colon");
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        r#"
class Example {}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("check")
        .arg("--java")
        .arg("21")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&source)
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("failed to build juv check compiler wrapper classpath"),
        "{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn check_supports_directories_and_no_error_prone() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(
        src.join("Broken.java"),
        r#"
class Broken {
  int value() {
    return missing;
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("check")
        .arg("--java")
        .arg("21")
        .arg("--no-error-prone")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&src)
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Broken.java"), "{stdout}");
    assert!(stdout.contains("cannot find symbol"), "{stdout}");
}

#[test]
fn check_warnings_as_errors_fails_error_prone_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    fs::write(
        tmp.path().join("Example.java"),
        r#"
class Example {
  boolean same(String value) {
    return value == "x";
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("check")
        .arg("--java")
        .arg("21")
        .arg("--warnings-as-errors")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(tmp.path())
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ReferenceEquality"), "{stdout}");
    assert!(
        stdout.contains("warnings found and -Werror specified"),
        "{stdout}"
    );
}
