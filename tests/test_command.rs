use serde_json::Value;
use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
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

fn write_junit_test(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let test = tmp.path().join("CalculatorTest.java");
    fs::write(
        &test,
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertEquals;

class CalculatorTest {
  @Test
  void addsNumbers() {
    assertEquals(4, 2 + 2);
  }
}
"#,
    )
    .unwrap();
    test
}

fn write_failing_junit_test(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let test = tmp.path().join("BrokenTest.java");
    fs::write(
        &test,
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertEquals;

class BrokenTest {
  @Test
  void failsClearly() {
    assertEquals(5, 2 + 2);
  }
}
"#,
    )
    .unwrap();
    test
}

#[test]
fn test_runs_junit_standalone_launcher_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let test = write_junit_test(&tmp);

    let out = juv_command()
        .arg("test")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&test)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CalculatorTest"), "stdout was {stdout}");
    assert!(stdout.contains("addsNumbers"), "stdout was {stdout}");
}

#[test]
fn test_json_prints_converted_junit_report() {
    let tmp = tempfile::tempdir().unwrap();
    let test = write_junit_test(&tmp);

    let out = juv_command()
        .arg("test")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&test)
        .output()
        .unwrap();

    assert_success(&out);
    let json: Value = serde_json::from_slice(&out.stdout).expect("stdout should be JSON");
    assert_eq!(json["tests"], 1);
    assert_eq!(json["failures"], 0);
    assert_eq!(json["skipped"], 0);
    assert_eq!(json["testCases"][0]["className"], "CalculatorTest");
    assert_eq!(json["testCases"][0]["name"], "addsNumbers()");
    assert_eq!(json["testCases"][0]["status"], "passed");
}

#[test]
fn test_json_prints_failures_before_exiting_nonzero() {
    let tmp = tempfile::tempdir().unwrap();
    let test = write_failing_junit_test(&tmp);

    let out = juv_command()
        .arg("test")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&test)
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "failing tests should return non-zero"
    );
    let json: Value = serde_json::from_slice(&out.stdout).expect("stdout should be JSON");
    assert_eq!(json["tests"], 1);
    assert_eq!(json["failures"], 1);
    assert_eq!(json["testCases"][0]["className"], "BrokenTest");
    assert_eq!(json["testCases"][0]["name"], "failsClearly()");
    assert_eq!(json["testCases"][0]["status"], "failed");
}

#[test]
fn test_xml_prints_junit_report_xml() {
    let tmp = tempfile::tempdir().unwrap();
    let test = write_junit_test(&tmp);

    let out = juv_command()
        .arg("test")
        .arg("--xml")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&test)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("<testsuite"), "stdout was {stdout}");
    assert!(stdout.contains("CalculatorTest"), "stdout was {stdout}");
    assert!(stdout.contains("addsNumbers()"), "stdout was {stdout}");
}
