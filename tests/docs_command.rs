use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
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

fn serve_files(files: HashMap<&'static str, Vec<u8>>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&requests);
    thread::spawn(move || loop {
        let Ok((mut stream, _)) = listener.accept() else {
            break;
        };
        let mut request = [0_u8; 2048];
        let read = stream.read(&mut request).unwrap_or(0);
        let request_text = String::from_utf8_lossy(&request[..read]);
        let path = request_text
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        seen.lock().unwrap().push(path.clone());
        let (status, body): (&str, &[u8]) = match files.get(path.as_str()) {
            Some(body) => ("200 OK", body.as_slice()),
            None => ("404 Not Found", b"not found"),
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    });
    (base, requests)
}

#[test]
fn docs_local_source_defaults_to_markdown_and_does_not_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Hello.java");
    fs::write(
        &script,
        r#"//DESCRIPTION Says hello to agents
//DOCS usage=Run it with a name.
//DEPS info.picocli:picocli:4.7.7
void main() {
  IO.println("hello");
}
"#,
    )
    .unwrap();
    let cache = tmp.path().join("cache");

    let out = jbx_command()
        .arg("docs")
        .arg(&script)
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("# Hello.java"), "{stdout}");
    assert!(stdout.contains("Says hello to agents"), "{stdout}");
    assert!(stdout.contains("usage: Run it with a name."), "{stdout}");
    assert!(stdout.contains("info.picocli:picocli:4.7.7"), "{stdout}");
    assert!(
        !cache.exists(),
        "local docs generation should not create a cache directory"
    );
}

#[test]
fn docs_remote_gav_fetches_markdown_sidecar_and_reuses_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = b"# Remote docs\n\nUse this from cache.\n".to_vec();
    let (repo, requests) = serve_files(HashMap::from([(
        "/dev/telegraphic/demo/1.0.0/demo-1.0.0-jbx-docs.md",
        docs,
    )]));
    let cache = tmp.path().join("cache");

    for _ in 0..2 {
        let out = jbx_command()
            .arg("docs")
            .arg("dev.telegraphic:demo:1.0.0")
            .arg("--repo")
            .arg(format!("local={repo}"))
            .arg("--cache-dir")
            .arg(&cache)
            .output()
            .unwrap();
        assert_success(&out);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "# Remote docs\n\nUse this from cache.\n"
        );
    }

    let seen = requests.lock().unwrap();
    assert_eq!(
        seen.iter()
            .filter(|path| path.as_str() == "/dev/telegraphic/demo/1.0.0/demo-1.0.0-jbx-docs.md")
            .count(),
        1,
        "remote sidecar should be fetched once then served from cache: {seen:?}"
    );
    assert!(
        cache
            .join("docs/dev/telegraphic/demo/1.0.0/demo-1.0.0-jbx-docs.md")
            .exists(),
        "expected cached docs sidecar under docs namespace"
    );
}

#[test]
fn docs_remote_gav_json_fetches_json_sidecar() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, _) = serve_files(HashMap::from([(
        "/dev/telegraphic/demo/1.0.0/demo-1.0.0-jbx-docs.json",
        br#"{"artifact":"dev.telegraphic:demo:1.0.0","summary":"Remote docs"}"#.to_vec(),
    )]));

    let out = jbx_command()
        .arg("docs")
        .arg("dev.telegraphic:demo:1.0.0")
        .arg("--json")
        .arg("--repo")
        .arg(format!("local={repo}"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["artifact"], "dev.telegraphic:demo:1.0.0");
    assert_eq!(json["summary"], "Remote docs");
}
