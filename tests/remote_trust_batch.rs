use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::thread;

fn doj_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_doj"))
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

fn serve_once(body: &'static str) -> String {
    serve_n(body, 1)
}

fn serve_n(body: &'static str, requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/RemoteHello.java", listener.local_addr().unwrap());
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    url
}

#[test]
fn remote_scripts_require_trust_before_execution() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = r#"class RemoteHello {
  public static void main(String[] args) { System.out.println("remote blocked"); }
}
"#;
    let url = serve_once(src);

    let out = doj_command()
        .arg("run")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&url)
        .output()
        .unwrap();

    assert_failure(&out);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not trusted"),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn trust_add_allows_remote_script_execution_and_trust_list_shows_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = r#"class RemoteHello {
  public static void main(String[] args) { System.out.println("remote ok " + args[0]); }
}
"#;
    let url = serve_once(src);

    let trust = doj_command()
        .arg("trust")
        .arg("add")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&url)
        .output()
        .unwrap();
    assert_success(&trust);

    let listed = doj_command()
        .arg("trust")
        .arg("list")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();
    assert_success(&listed);
    assert!(String::from_utf8_lossy(&listed.stdout).contains(&url));

    let run_url = serve_n(src, 2);
    let rewritten_cache = cache.join("run");
    let trust_again = doj_command()
        .arg("trust")
        .arg("add")
        .arg("--cache-dir")
        .arg(&rewritten_cache)
        .arg(&run_url)
        .output()
        .unwrap();
    assert_success(&trust_again);
    let out = doj_command()
        .arg("run")
        .arg("--cache-dir")
        .arg(&rewritten_cache)
        .arg(&run_url)
        .arg("world")
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "remote ok world"
    );
}

#[test]
fn run_trust_flag_trusts_and_runs_remote_script_in_one_step() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = r#"class RemoteHello {
  public static void main(String[] args) { System.out.println("trusted inline"); }
}
"#;
    let url = serve_once(src);

    let out = doj_command()
        .arg("run")
        .arg("--trust")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&url)
        .output()
        .unwrap();
    assert_success(&out);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "trusted inline"
    );

    let trust_file = cache.join("trust.tsv");
    assert!(trust_file.exists(), "trust file should be written");
    assert!(fs::read_to_string(trust_file).unwrap().contains(&url));
}

#[test]
fn trust_remove_and_clear_manage_trusted_remote_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = "class RemoteHello { public static void main(String[] args) {} }\n";
    let url = serve_once(src);

    assert_success(
        &doj_command()
            .arg("trust")
            .arg("add")
            .arg("--cache-dir")
            .arg(&cache)
            .arg(&url)
            .output()
            .unwrap(),
    );
    assert_success(
        &doj_command()
            .arg("trust")
            .arg("remove")
            .arg("--cache-dir")
            .arg(&cache)
            .arg(&url)
            .output()
            .unwrap(),
    );
    let listed = doj_command()
        .arg("trust")
        .arg("list")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();
    assert_success(&listed);
    assert!(!String::from_utf8_lossy(&listed.stdout).contains(&url));

    let url2 = serve_once(src);
    assert_success(
        &doj_command()
            .arg("trust")
            .arg("add")
            .arg("--cache-dir")
            .arg(&cache)
            .arg(&url2)
            .output()
            .unwrap(),
    );
    assert_success(
        &doj_command()
            .arg("trust")
            .arg("clear")
            .arg("--cache-dir")
            .arg(&cache)
            .output()
            .unwrap(),
    );
    let listed_after_clear = doj_command()
        .arg("trust")
        .arg("list")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();
    assert_success(&listed_after_clear);
    assert_eq!(
        String::from_utf8_lossy(&listed_after_clear.stdout).trim(),
        ""
    );
}
