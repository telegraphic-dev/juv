use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::thread;

fn juvx_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juvx"))
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

fn command_output(mut command: Command) -> String {
    let output = command.output().expect("failed to run command");
    assert_success(&output);
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn build_executable_jar(tmp: &tempfile::TempDir) -> Vec<u8> {
    let source_dir = tmp.path().join("src/dev/telegraphic/tool");
    let classes_dir = tmp.path().join("classes");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&classes_dir).unwrap();
    let source = source_dir.join("Tool.java");
    fs::write(
        &source,
        r#"
package dev.telegraphic.tool;

public class Tool {
  public static void main(String[] args) {
    System.out.println("juvx " + String.join(",", args));
  }
}
"#,
    )
    .unwrap();

    let mut javac = Command::new("javac");
    javac.arg("-d").arg(&classes_dir).arg(&source);
    command_output(javac);

    let jar_path = tmp.path().join("hello-tool-1.0.0.jar");
    let mut jar = Command::new("jar");
    jar.arg("--create")
        .arg("--file")
        .arg(&jar_path)
        .arg("--main-class")
        .arg("dev.telegraphic.tool.Tool")
        .arg("-C")
        .arg(&classes_dir)
        .arg(".");
    command_output(jar);

    fs::read(jar_path).unwrap()
}

fn serve_files(files: HashMap<&'static str, Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
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
            .unwrap_or("/");
        let (status, body): (&str, &[u8]) = match files.get(path) {
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
    base
}

#[test]
fn juvx_runs_executable_jar_from_gav() {
    let tmp = tempfile::tempdir().unwrap();
    let jar = build_executable_jar(&tmp);
    let pom = br#"
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>dev.telegraphic</groupId>
  <artifactId>hello-tool</artifactId>
  <version>1.0.0</version>
</project>
"#
    .to_vec();
    let repo = serve_files(HashMap::from([
        (
            "/dev/telegraphic/hello-tool/1.0.0/hello-tool-1.0.0.pom",
            pom,
        ),
        (
            "/dev/telegraphic/hello-tool/1.0.0/hello-tool-1.0.0.jar",
            jar,
        ),
    ]));

    let output = juvx_command()
        .arg("--repo")
        .arg(format!("local={repo}"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("dev.telegraphic:hello-tool:1.0.0")
        .arg("--")
        .arg("alpha")
        .arg("beta")
        .output()
        .expect("failed to run juv juvx");

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "juvx alpha,beta"
    );

    let output = juvx_command()
        .arg("--repo")
        .arg(format!("local={repo}"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache-main"))
        .arg("dev.telegraphic:hello-tool:1.0.0")
        .arg("--main")
        .arg("dev.telegraphic.tool.Tool")
        .arg("--")
        .arg("gamma")
        .output()
        .expect("failed to run juv juvx with --main after coordinate");

    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "juvx gamma");
}

#[test]
fn juvx_uses_latest_metadata_version_when_gav_version_is_omitted() {
    let tmp = tempfile::tempdir().unwrap();
    let jar = build_executable_jar(&tmp);
    let metadata = br#"
<metadata>
  <groupId>dev.telegraphic</groupId>
  <artifactId>hello-tool</artifactId>
  <versioning>
    <latest>1.0.0</latest>
    <release>1.0.0</release>
    <versions>
      <version>0.9.0</version>
      <version>1.0.0</version>
    </versions>
  </versioning>
</metadata>
"#
    .to_vec();
    let pom = br#"
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>dev.telegraphic</groupId>
  <artifactId>hello-tool</artifactId>
  <version>1.0.0</version>
</project>
"#
    .to_vec();
    let repo = serve_files(HashMap::from([
        ("/dev/telegraphic/hello-tool/maven-metadata.xml", metadata),
        (
            "/dev/telegraphic/hello-tool/1.0.0/hello-tool-1.0.0.pom",
            pom,
        ),
        (
            "/dev/telegraphic/hello-tool/1.0.0/hello-tool-1.0.0.jar",
            jar,
        ),
    ]));

    let output = juvx_command()
        .arg("--repo")
        .arg(format!("local={repo}"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache-latest"))
        .arg("dev.telegraphic:hello-tool")
        .arg("--")
        .arg("delta")
        .output()
        .expect("failed to run juvx with omitted coordinate version");

    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "juvx delta");
}
