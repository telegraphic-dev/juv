use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;
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

#[test]
fn check_uses_jbang_dependency_directives_for_classpath() {
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
    let source = tmp.path().join("UseDep.java");
    fs::write(
        &source,
        format!(
            r#"
//REPOS local={repo}
//DEPS com.example:greeter:1.0.0
import com.example.Greeter;
class UseDep {{
  String message() {{
    return Greeter.message();
  }}
}}
"#
        ),
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

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(payload["ok"], true, "{stdout}");
    assert_eq!(
        payload["diagnostics"].as_array().unwrap().len(),
        0,
        "{stdout}"
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
