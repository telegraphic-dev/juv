use std::fs;
use std::io::{Read, Write};
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
    let mut request = [0_u8; 1024];
    let _ = stream.read(&mut request);
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
