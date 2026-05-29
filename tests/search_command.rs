use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::sync::mpsc;
use std::thread;

fn jbx_command() -> Command {
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

fn serve_search_response(
    body: &'static str,
) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).unwrap_or(0);
        let request_text = String::from_utf8_lossy(&request[..read]).to_string();
        tx.send(request_text).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (base, rx, handle)
}

#[test]
fn search_prints_aligned_table_sorted_by_popularity() {
    let (base, requests, handle) = serve_search_response(
        r#"{
  "response": {
    "numFound": 2,
    "docs": [
      {"id":"com.example:tiny","g":"com.example","a":"tiny","latestVersion":"1.0.0","p":"jar","versionCount": 1},
      {"id":"com.google.inject:guice","g":"com.google.inject","a":"guice","latestVersion":"7.0.0","p":"jar","versionCount": 24}
    ]
  }
}"#,
    );

    let output = jbx_command()
        .arg("search")
        .arg("guice")
        .arg("--limit")
        .arg("5")
        .env("JBX_MAVEN_SEARCH_URL", base)
        .output()
        .expect("failed to run jbx search");

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(
        lines[0],
        "ARTIFACT                 VERSION  PACKAGING  VERSIONS"
    );
    assert_eq!(lines[1], "com.google.inject:guice  7.0.0    jar        24");
    assert_eq!(lines[2], "com.example:tiny         1.0.0    jar        1");
    let request = requests.recv().unwrap();
    handle.join().unwrap();
    assert!(request.starts_with("GET /solrsearch/select?"), "{request}");
    assert!(request.contains("q=guice"), "{request}");
    assert!(request.contains("rows=100"), "{request}");
    assert!(request.contains("wt=json"), "{request}");
}

#[test]
fn search_json_outputs_agent_friendly_payload_and_coordinate_query() {
    let (base, requests, handle) = serve_search_response(
        r#"{
  "response": {
    "numFound": 1,
    "docs": [
      {"id":"com.google.inject:guice:7.0.0","g":"com.google.inject","a":"guice","v":"7.0.0","p":"jar","timestamp": 1684863927000}
    ]
  }
}"#,
    );

    let output = jbx_command()
        .arg("search")
        .arg("com.google.inject:guice:7.0.0")
        .arg("--json")
        .env("JBX_MAVEN_SEARCH_URL", base)
        .output()
        .expect("failed to run jbx search --json");

    assert_success(&output);
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        payload["query"],
        "g:com.google.inject AND a:guice AND v:7.0.0"
    );
    assert_eq!(payload["numFound"], 1);
    assert_eq!(
        payload["artifacts"][0]["coordinate"],
        "com.google.inject:guice:7.0.0"
    );
    assert_eq!(payload["artifacts"][0]["groupId"], "com.google.inject");
    assert_eq!(payload["artifacts"][0]["artifactId"], "guice");
    assert_eq!(payload["artifacts"][0]["version"], "7.0.0");

    let request = requests.recv().unwrap();
    handle.join().unwrap();
    assert!(request.contains("core=gav"), "{request}");
    assert!(
        request.contains("q=g%3Acom.google.inject%20AND%20a%3Aguice%20AND%20v%3A7.0.0")
            || request.contains("q=g%3Acom.google.inject+AND+a%3Aguice+AND+v%3A7.0.0"),
        "{request}"
    );
}

#[test]
fn search_accepts_solr_group_and_id_parameters() {
    let (base, requests, handle) = serve_search_response(
        r#"{
  "response": {
    "numFound": 1,
    "docs": [
      {"id":"io.micronaut:micronaut-core","g":"io.micronaut","a":"micronaut-core","latestVersion":"4.9.0","p":"jar","versionCount": 170}
    ]
  }
}"#,
    );

    let output = jbx_command()
        .arg("search")
        .arg("--group")
        .arg("io.micronaut")
        .arg("--id")
        .arg("micronaut-core")
        .arg("--json")
        .env("JBX_MAVEN_SEARCH_URL", base)
        .output()
        .expect("failed to run jbx search with solr fields");

    assert_success(&output);
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["query"], "g:io.micronaut AND a:micronaut-core");
    assert_eq!(
        payload["artifacts"][0]["artifact"],
        "io.micronaut:micronaut-core"
    );

    let request = requests.recv().unwrap();
    handle.join().unwrap();
    assert!(
        request.contains("q=g%3Aio.micronaut%20AND%20a%3Amicronaut-core")
            || request.contains("q=g%3Aio.micronaut+AND+a%3Amicronaut-core"),
        "{request}"
    );
}

#[test]
fn search_ignores_empty_version_filter_without_switching_to_gav_core() {
    let (base, requests, handle) = serve_search_response(
        r#"{
  "response": {
    "numFound": 1,
    "docs": [
      {"id":"io.micronaut:micronaut-core","g":"io.micronaut","a":"micronaut-core","latestVersion":"4.9.0","p":"jar","versionCount": 170}
    ]
  }
}"#,
    );

    let output = jbx_command()
        .arg("search")
        .arg("--group")
        .arg("io.micronaut")
        .arg("--id")
        .arg("micronaut-core")
        .arg("--version")
        .arg("")
        .env("JBX_MAVEN_SEARCH_URL", base)
        .output()
        .expect("failed to run jbx search with empty version");

    assert_success(&output);
    let request = requests.recv().unwrap();
    handle.join().unwrap();
    assert!(!request.contains("core=gav"), "{request}");
    assert!(!request.contains("v%3A"), "{request}");
}

#[test]
fn search_keeps_exact_artifact_id_matches_above_popularity_sort() {
    let (base, _requests, handle) = serve_search_response(
        r#"{
  "response": {
    "numFound": 3,
    "docs": [
      {"id":"io.micronaut.spring:micronaut-spring-web","g":"io.micronaut.spring","a":"micronaut-spring-web","latestVersion":"5.11.0","p":"jar","versionCount": 58},
      {"id":"io.micronaut:micronaut-spring","g":"io.micronaut","a":"micronaut-spring","latestVersion":"2.0.1","p":"jar","versionCount": 44},
      {"id":"io.github.crac.io.micronaut:micronaut-spring","g":"io.github.crac.io.micronaut","a":"micronaut-spring","latestVersion":"1.3.7","p":"jar","versionCount": 1}
    ]
  }
}"#,
    );

    let output = jbx_command()
        .arg("search")
        .arg("micronaut-spring")
        .arg("--limit")
        .arg("3")
        .env("JBX_MAVEN_SEARCH_URL", base)
        .output()
        .expect("failed to run jbx search");

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines[1].starts_with("io.micronaut:micronaut-spring"),
        "{stdout}"
    );
    assert!(
        lines[2].starts_with("io.github.crac.io.micronaut:micronaut-spring"),
        "{stdout}"
    );
    assert!(
        lines[3].starts_with("io.micronaut.spring:micronaut-spring-web"),
        "{stdout}"
    );
    handle.join().unwrap();
}

#[test]
fn search_json_num_found_fallback_uses_untruncated_docs() {
    let (base, _requests, handle) = serve_search_response(
        r#"{
  "response": {
    "docs": [
      {"id":"com.example:one","g":"com.example","a":"one","latestVersion":"1.0.0","p":"jar","versionCount": 1},
      {"id":"com.example:two","g":"com.example","a":"two","latestVersion":"1.0.0","p":"jar","versionCount": 2}
    ]
  }
}"#,
    );

    let output = jbx_command()
        .arg("search")
        .arg("example")
        .arg("--limit")
        .arg("1")
        .arg("--json")
        .env("JBX_MAVEN_SEARCH_URL", base)
        .output()
        .expect("failed to run jbx search without numFound");

    assert_success(&output);
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["numFound"], 2);
    assert_eq!(payload["artifacts"].as_array().unwrap().len(), 1);
    handle.join().unwrap();
}
