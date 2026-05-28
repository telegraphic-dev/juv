use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

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

fn serve_files(files: HashMap<&'static str, &'static str>, requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            let request_text = String::from_utf8_lossy(&request[..read]);
            let path = request_text
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            let (status, body) = match files.get(path) {
                Some(body) => ("200 OK", *body),
                None => ("404 Not Found", "not found"),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    base
}

fn serve_file_sequences(
    files: HashMap<&'static str, Vec<&'static str>>,
    requests: usize,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files = Arc::new(Mutex::new(
        files
            .into_iter()
            .map(|(path, bodies)| (path, VecDeque::from(bodies)))
            .collect::<HashMap<_, _>>(),
    ));
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            let request_text = String::from_utf8_lossy(&request[..read]);
            let path = request_text
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            let body = files
                .lock()
                .unwrap()
                .get_mut(path)
                .and_then(|bodies| bodies.pop_front());
            let (status, body) = match body {
                Some(body) => ("200 OK", body),
                None => ("404 Not Found", "not found"),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    base
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

    let out = juv_command()
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
fn remote_relative_resources_reject_dot_segments() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let base = serve_files(
        HashMap::from([(
            "/dot/Main.java",
            r#"//SOURCES helpers/./Helper.java
class Main { public static void main(String[] args) {} }
"#,
        )]),
        1,
    );
    let url = format!("{base}/dot/Main.java");

    let out = juv_command()
        .arg("run")
        .arg("--trust")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&url)
        .output()
        .unwrap();

    assert_failure(&out);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("must not contain empty or parent segments"),
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

    let trust = juv_command()
        .arg("trust")
        .arg("add")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&url)
        .output()
        .unwrap();
    assert_success(&trust);

    let listed = juv_command()
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
    let trust_again = juv_command()
        .arg("trust")
        .arg("add")
        .arg("--cache-dir")
        .arg(&rewritten_cache)
        .arg(&run_url)
        .output()
        .unwrap();
    assert_success(&trust_again);
    let out = juv_command()
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

    let out = juv_command()
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
fn remote_relative_sources_are_downloaded_next_to_main_script() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let base = serve_files(
        HashMap::from([
            (
                "/scripts/RemoteMain.java",
                r#"//SOURCES helpers/RemoteHelper.java
class RemoteMain {
  public static void main(String[] args) { System.out.print(RemoteHelper.message()); }
}
"#,
            ),
            (
                "/scripts/helpers/RemoteHelper.java",
                r#"class RemoteHelper {
  static String message() { return "remote-source-ok"; }
}
"#,
            ),
        ]),
        2,
    );
    let url = format!("{base}/scripts/RemoteMain.java");

    let out = juv_command()
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
        "remote-source-ok"
    );
}

#[test]
fn remote_relative_source_like_deps_are_downloaded_next_to_main_script() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let base = serve_files(
        HashMap::from([
            (
                "/deps/RemoteDepMain.java",
                r#"//DEPS RemoteDepHelper.java
class RemoteDepMain {
  public static void main(String[] args) { System.out.print(RemoteDepHelper.message()); }
}
"#,
            ),
            (
                "/deps/RemoteDepHelper.java",
                r#"class RemoteDepHelper {
  static String message() { return "remote-source-dep-ok"; }
}
"#,
            ),
        ]),
        2,
    );
    let url = format!("{base}/deps/RemoteDepMain.java");

    let out = juv_command()
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
        "remote-source-dep-ok"
    );
}

#[test]
fn trusted_remote_relative_source_change_blocks_execution() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let base = serve_file_sequences(
        HashMap::from([
            (
                "/secure/Main.java",
                vec![
                    r#"//SOURCES Helper.java
class Main {
  public static void main(String[] args) { System.out.print(Helper.message()); }
}
"#,
                    r#"//SOURCES Helper.java
class Main {
  public static void main(String[] args) { System.out.print(Helper.message()); }
}
"#,
                ],
            ),
            (
                "/secure/Helper.java",
                vec![
                    r#"class Helper {
  static String message() { return "trusted-helper"; }
}
"#,
                    r#"class Helper {
  static String message() { return "changed-helper"; }
}
"#,
                ],
            ),
        ]),
        4,
    );
    let url = format!("{base}/secure/Main.java");

    let trusted = juv_command()
        .arg("trust")
        .arg("add")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&url)
        .output()
        .unwrap();
    assert_success(&trusted);

    let out = juv_command()
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
fn remote_relative_files_are_downloaded_as_classpath_resources() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let base = serve_files(
        HashMap::from([
            (
                "/examples/RemoteResource.java",
                r#"//FILES config/app.properties=resources/app.properties
class RemoteResource {
  public static void main(String[] args) throws Exception {
    try (var in = RemoteResource.class.getResourceAsStream("/config/app.properties")) {
      System.out.print(new String(in.readAllBytes()).trim());
    }
  }
}
"#,
            ),
            (
                "/examples/resources/app.properties",
                "answer=remote-file-ok\n",
            ),
        ]),
        2,
    );
    let url = format!("{base}/examples/RemoteResource.java");

    let out = juv_command()
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
        "answer=remote-file-ok"
    );
}

#[test]
fn trust_remove_and_clear_manage_trusted_remote_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = "class RemoteHello { public static void main(String[] args) {} }\n";
    let url = serve_once(src);

    assert_success(
        &juv_command()
            .arg("trust")
            .arg("add")
            .arg("--cache-dir")
            .arg(&cache)
            .arg(&url)
            .output()
            .unwrap(),
    );
    assert_success(
        &juv_command()
            .arg("trust")
            .arg("remove")
            .arg("--cache-dir")
            .arg(&cache)
            .arg(&url)
            .output()
            .unwrap(),
    );
    let listed = juv_command()
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
        &juv_command()
            .arg("trust")
            .arg("add")
            .arg("--cache-dir")
            .arg(&cache)
            .arg(&url2)
            .output()
            .unwrap(),
    );
    assert_success(
        &juv_command()
            .arg("trust")
            .arg("clear")
            .arg("--cache-dir")
            .arg(&cache)
            .output()
            .unwrap(),
    );
    let listed_after_clear = juv_command()
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
