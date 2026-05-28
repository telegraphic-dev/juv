use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;
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

fn with_junit_version(command: &mut Command) -> &mut Command {
    command.arg("--junit-version").arg("6.1.0")
}

fn compile_greeter_jar(root: &Path) -> Vec<u8> {
    let lib_src_dir = root.join("libsrc/com/example");
    let lib_classes = root.join("libclasses");
    fs::create_dir_all(&lib_src_dir).unwrap();
    fs::create_dir_all(&lib_classes).unwrap();
    let lib_src = lib_src_dir.join("Greeter.java");
    fs::write(
        &lib_src,
        r#"
package com.example;
public class Greeter {
  public static String message() { return "deps-ok"; }
}
"#,
    )
    .unwrap();
    let javac = Command::new("javac")
        .arg("--release")
        .arg("8")
        .arg("-d")
        .arg(&lib_classes)
        .arg(&lib_src)
        .output()
        .unwrap();
    assert_success(&javac);

    let jar_path = root.join("greeter-1.0.0.jar");
    let jar = Command::new("jar")
        .arg("--create")
        .arg("--file")
        .arg(&jar_path)
        .arg("-C")
        .arg(&lib_classes)
        .arg(".")
        .output()
        .unwrap();
    assert_success(&jar);
    fs::read(jar_path).unwrap()
}

fn serve_files(files: std::collections::HashMap<&'static str, Vec<u8>>) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let files = files.clone();
            std::thread::spawn(move || {
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                if reader.read_line(&mut request).is_err() {
                    return;
                }
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                while {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap_or(0) > 0 && line != "\r\n"
                } {}
                match files.get(path) {
                    Some(body) => {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(body);
                    }
                    None => {
                        let body = b"not found";
                        let header = format!(
                            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(body);
                    }
                }
            });
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn assert_well_formed_xml(xml: &str) {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(_) => buf.clear(),
            Err(err) => panic!("XML should be well-formed: {err}; xml was {xml}"),
        }
    }
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

#[test]
fn test_xml_output_is_well_formed_with_multiple_suites() {
    let tmp = tempfile::tempdir().unwrap();
    let first = write_junit_test(&tmp);
    let second = tmp.path().join("OtherTest.java");
    fs::write(
        &second,
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertTrue;

class OtherTest {
  @Test
  void alsoRuns() {
    assertTrue(true);
  }
}
"#,
    )
    .unwrap();

    let out = with_junit_version(juv_command().arg("test"))
        .arg("--xml")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--source")
        .arg("OtherTest.java")
        .arg(&first)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_well_formed_xml(&stdout);
    assert!(stdout.contains("CalculatorTest"), "stdout was {stdout}");
    assert!(stdout.contains("OtherTest"), "stdout was {stdout}");
    assert!(stdout.matches("<?xml").count() <= 1, "stdout was {stdout}");
}

#[test]
fn test_without_target_runs_all_tests_in_current_directory() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Calculator.java"),
        r#"
class Calculator {
  static int add(int left, int right) {
    return left + right;
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("CalculatorTest.java"),
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertEquals;

class CalculatorTest {
  @Test
  void addsNumbers() {
    assertEquals(9, Calculator.add(4, 5));
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("OtherTest.java"),
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertTrue;

class OtherTest {
  @Test
  void alsoRuns() {
    assertTrue(true);
  }
}
"#,
    )
    .unwrap();

    let out = with_junit_version(juv_command().arg("test"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CalculatorTest"), "stdout was {stdout}");
    assert!(stdout.contains("OtherTest"), "stdout was {stdout}");
}

#[test]
fn test_runs_all_tests_in_directory_and_compiles_sources() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Calculator.java"),
        r#"
class Calculator {
  static int add(int left, int right) {
    return left + right;
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("CalculatorTest.java"),
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertEquals;

class CalculatorTest {
  @Test
  void addsNumbers() {
    assertEquals(9, Calculator.add(4, 5));
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("OtherTest.java"),
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertTrue;

class OtherTest {
  @Test
  void alsoRuns() {
    assertTrue(true);
  }
}
"#,
    )
    .unwrap();

    let out = with_junit_version(juv_command().arg("test"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CalculatorTest"), "stdout was {stdout}");
    assert!(stdout.contains("OtherTest"), "stdout was {stdout}");
}

#[test]
fn test_directory_target_merges_dependency_directives_from_companion_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let jar = compile_greeter_jar(tmp.path());
    let repo = serve_files(std::collections::HashMap::from([
        (
            "/com/example/greeter/1.0.0/greeter-1.0.0.pom",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>greeter</artifactId>
  <version>1.0.0</version>
  <packaging>jar</packaging>
</project>"#
                .to_vec(),
        ),
        ("/com/example/greeter/1.0.0/greeter-1.0.0.jar", jar),
    ]));
    fs::write(
        tmp.path().join("App.java"),
        format!(
            r#"
//REPOS local={repo}
//DEPS com.example:greeter:1.0.0
import com.example.Greeter;

class App {{
  static String message() {{
    return Greeter.message();
  }}
}}
"#
        ),
    )
    .unwrap();
    fs::write(
        tmp.path().join("AppTest.java"),
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertEquals;

class AppTest {
  @Test
  void usesDependencyFromCompanionSource() {
    assertEquals("deps-ok", App.message());
  }
}
"#,
    )
    .unwrap();

    let out = with_junit_version(juv_command().arg("test"))
        .arg("--cache-dir")
        .arg(&cache)
        .arg(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("usesDependencyFromCompanionSource"),
        "stdout was {stdout}"
    );
}

#[test]
fn test_compiles_matching_source_for_test_script() {
    let tmp = tempfile::tempdir().unwrap();
    let main = tmp.path().join("Calculator.java");
    fs::write(
        &main,
        r#"
class Calculator {
  static int add(int left, int right) {
    return left + right;
  }
}
"#,
    )
    .unwrap();
    let test = tmp.path().join("CalculatorTest.java");
    fs::write(
        &test,
        r#"
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.assertEquals;

class CalculatorTest {
  @Test
  void usesCompanionSource() {
    assertEquals(7, Calculator.add(3, 4));
  }
}
"#,
    )
    .unwrap();

    let out = with_junit_version(juv_command().arg("test"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&test)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("usesCompanionSource"),
        "stdout was {stdout}"
    );
}
