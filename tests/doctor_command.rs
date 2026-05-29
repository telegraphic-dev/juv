use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Output};
use std::thread;

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

fn serve_maven_search() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || serve_search_response(stream));
        }
    });
    format!("http://{addr}")
}

fn serve_search_response(mut stream: TcpStream) {
    let mut request_line = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        reader.read_line(&mut request_line).unwrap();
        assert!(
            request_line.starts_with("GET /solrsearch/select?"),
            "unexpected request line: {request_line}"
        );
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header == "\r\n" || header.is_empty() {
                break;
            }
        }
    }
    let body = r#"{"response":{"numFound":1,"docs":[{"g":"org.junit.platform","a":"junit-platform-console-standalone","latestVersion":"1.13.4","p":"jar","versionCount":99}]}}"#;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).unwrap();
}

#[test]
fn doctor_json_reports_core_checks_and_skips_contextual_tools() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_jdk = tmp.path().join("jdk-25");
    fs::create_dir_all(fake_jdk.join("bin")).unwrap();
    fs::write(fake_jdk.join("bin/java"), b"#!/bin/sh\n").unwrap();
    fs::write(fake_jdk.join("bin/javac"), b"#!/bin/sh\n").unwrap();
    fs::write(fake_jdk.join("release"), "JAVA_VERSION=\"25.0.1\"\n").unwrap();

    let out = jbx_command()
        .env("JAVA_HOME", &fake_jdk)
        .env("JBX_MAVEN_SEARCH_URL", serve_maven_search())
        .arg("doctor")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert!(checks
        .iter()
        .any(|check| check["name"] == "jdk" && check["status"] == "ok"));
    assert!(checks
        .iter()
        .any(|check| check["name"] == "maven-central" && check["status"] == "ok"));
    assert!(checks
        .iter()
        .any(|check| check["name"] == "cache" && check["status"] == "ok"));
    assert!(checks
        .iter()
        .any(|check| check["name"] == "gpg" && check["status"] == "skipped"));
    assert!(checks
        .iter()
        .any(|check| check["name"] == "native-image" && check["status"] == "skipped"));
}

#[test]
fn doctor_json_reports_version_drift_for_classifier_coordinate() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_jdk = tmp.path().join("jdk-25");
    fs::create_dir_all(fake_jdk.join("bin")).unwrap();
    fs::write(fake_jdk.join("bin/java"), b"#!/bin/sh\n").unwrap();
    fs::write(fake_jdk.join("bin/javac"), b"#!/bin/sh\n").unwrap();
    fs::write(fake_jdk.join("release"), "JAVA_VERSION=\"25.0.1\"\n").unwrap();
    let script = tmp.path().join("Hello.java");
    fs::write(
        &script,
        "//DEPS org.junit.platform:junit-platform-console-standalone:all:1.11.3\nvoid main() {}\n",
    )
    .unwrap();

    let out = jbx_command()
        .env("JAVA_HOME", &fake_jdk)
        .env("JBX_MAVEN_SEARCH_URL", serve_maven_search())
        .arg("doctor")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&script)
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let checks = json["checks"].as_array().unwrap();
    let drift = checks
        .iter()
        .find(|check| check["name"] == "dependency-version")
        .unwrap();
    assert!(
        drift["summary"]
            .as_str()
            .unwrap()
            .contains("declares 1.11.3"),
        "{drift:#}"
    );
    assert!(
        !drift["summary"].as_str().unwrap().contains("declares all"),
        "{drift:#}"
    );
}

#[test]
fn doctor_cache_probe_does_not_overwrite_fixed_probe_name() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_jdk = tmp.path().join("jdk-25");
    fs::create_dir_all(fake_jdk.join("bin")).unwrap();
    fs::write(fake_jdk.join("bin/java"), b"#!/bin/sh\n").unwrap();
    fs::write(fake_jdk.join("bin/javac"), b"#!/bin/sh\n").unwrap();
    fs::write(fake_jdk.join("release"), "JAVA_VERSION=\"25.0.1\"\n").unwrap();
    let cache = tmp.path().join("cache");
    fs::create_dir_all(&cache).unwrap();
    let existing_probe = cache.join(".jbx-doctor-write-test");
    fs::write(&existing_probe, b"keep me").unwrap();

    let out = jbx_command()
        .env("JAVA_HOME", &fake_jdk)
        .env("JBX_MAVEN_SEARCH_URL", serve_maven_search())
        .arg("doctor")
        .arg("--json")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(fs::read(&existing_probe).unwrap(), b"keep me");
}
