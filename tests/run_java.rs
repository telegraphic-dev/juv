use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
}

fn run_jbx(args: &[&std::path::Path], text_args: &[&str]) -> Output {
    let mut cmd = juv_command();
    // GitHub Actions can run several Java processes at once and occasionally
    // emits hsperfdata lock warnings into stdout. Disable shared perfdata so
    // tests assert script output, not VM housekeeping noise.
    cmd.env("JAVA_TOOL_OPTIONS", "-XX:+PerfDisableSharedMem");
    for arg in args {
        cmd.arg(arg);
    }
    for arg in text_args {
        cmd.arg(arg);
    }
    cmd.output().unwrap()
}

#[test]
fn runs_single_java_file_with_jbang_style_main_directive() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("Hello.java");
    fs::write(
        &src,
        r#"
//MAIN Hello
class Hello {
  public static void main(String[] args) {
    System.out.println("hello " + args[0]);
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &src], &["world"]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello world");
}

#[test]
fn top_level_java_shorthand_passes_dash_args_without_separator() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("Args.java");
    fs::write(
        &src,
        r#"
class Args {
  public static void main(String[] args) {
    System.out.println(String.join(",", args));
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[&src], &["--name", "jay"]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "--name,jay");
}

#[test]
fn explicit_run_passes_dash_args_without_separator() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("Args.java");
    fs::write(
        &src,
        r#"
class Args {
  public static void main(String[] args) {
    System.out.println(String.join(",", args));
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &src], &["--name", "jay"]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "--name,jay");
}

#[test]
fn infers_main_class_from_package_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("demo");
    fs::create_dir_all(&src_dir).unwrap();
    let src = src_dir.join("Packaged.java");
    fs::write(
        &src,
        r#"
package demo;
public class Packaged {
  public static void main(String[] args) {
    System.out.println(Packaged.class.getName());
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &src], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "demo.Packaged");
}

#[test]
fn compiles_additional_sources_declared_with_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let main = tmp.path().join("Main.java");
    let helper = tmp.path().join("Helper.java");
    fs::write(
        &helper,
        r#"
class Helper {
  static String message() { return "from helper"; }
}
"#,
    )
    .unwrap();
    fs::write(
        &main,
        r#"
//SOURCES Helper.java
class Main {
  public static void main(String[] args) {
    System.out.println(Helper.message());
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &main], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "from helper");
}

#[test]
fn compiles_nested_sources_declared_by_companion_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let main = tmp.path().join("Main.java");
    let helper = tmp.path().join("Helper.java");
    let nested = tmp.path().join("Nested.java");
    fs::write(
        &nested,
        r#"
class Nested {
  static String message() { return "from nested"; }
}
"#,
    )
    .unwrap();
    fs::write(
        &helper,
        r#"
//SOURCES Nested.java
class Helper {
  static String message() { return Nested.message(); }
}
"#,
    )
    .unwrap();
    fs::write(
        &main,
        r#"
//SOURCES Helper.java
class Main {
  public static void main(String[] args) {
    System.out.println(Helper.message());
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &main], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "from nested");
}

#[test]
fn treats_non_coordinate_deps_as_source_dependencies() {
    let tmp = tempfile::tempdir().unwrap();
    let main = tmp.path().join("Main.java");
    let helper = tmp.path().join("DepHelper.java");
    fs::write(
        &helper,
        r#"
class DepHelper {
  static String message() { return "source-dep-ok"; }
}
"#,
    )
    .unwrap();
    fs::write(
        &main,
        r#"
//DEPS DepHelper.java
class Main {
  public static void main(String[] args) {
    System.out.print(DepHelper.message());
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &main], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "source-dep-ok");
}

#[test]
fn makes_files_directive_available_as_classpath_resource_with_target_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("ReadResource.java");
    let resource = tmp.path().join("app.properties");
    fs::write(&resource, "answer=42\n").unwrap();
    fs::write(
        &src,
        r#"
//FILES config/app.properties=app.properties
class ReadResource {
  public static void main(String[] args) throws Exception {
    try (var in = ReadResource.class.getResourceAsStream("/config/app.properties")) {
      System.out.print(new String(in.readAllBytes()).trim());
    }
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &src], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "answer=42");
}

#[test]
fn applies_runtime_options_from_directive() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("RuntimeProp.java");
    fs::write(
        &src,
        r#"
//RUNTIME_OPTIONS -Djuv.test.value=runtime-ok
class RuntimeProp {
  public static void main(String[] args) {
    System.out.print(System.getProperty("juv.test.value"));
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &src], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "runtime-ok");
}

#[test]
fn preview_directive_enables_preview_for_compile_and_runtime() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("PreviewSwitch.java");
    fs::write(
        &src,
        r#"
//PREVIEW
class PreviewSwitch {
  public static void main(String[] args) {
    Object value = "preview-ok";
    String result = switch (value) {
      case String text -> text;
      default -> "nope";
    };
    System.out.print(result);
  }
}
"#,
    )
    .unwrap();

    let out = run_jbx(&[std::path::Path::new("run"), &src], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "preview-ok");
}

#[test]
#[cfg(unix)]
fn resolves_deps_with_native_resolver() {
    let tmp = tempfile::tempdir().unwrap();

    // Build a tiny library JAR: com.example:greeter:1.0.0
    let lib_src_dir = tmp.path().join("libsrc/com/example");
    let lib_classes = tmp.path().join("libclasses");
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
        .arg("-d")
        .arg(&lib_classes)
        .arg(&lib_src)
        .output()
        .unwrap();
    assert!(
        javac.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    // Set up local Maven repo layout: com/example/greeter/1.0.0/
    let repo_base = tmp.path().join("repo/com/example/greeter/1.0.0");
    fs::create_dir_all(&repo_base).unwrap();

    let jar_path = repo_base.join("greeter-1.0.0.jar");
    let jar = Command::new("jar")
        .arg("--create")
        .arg("--file")
        .arg(&jar_path)
        .arg("-C")
        .arg(&lib_classes)
        .arg(".")
        .output()
        .unwrap();
    assert!(
        jar.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&jar.stdout),
        String::from_utf8_lossy(&jar.stderr)
    );

    // Write a POM and Maven metadata for the artifact.
    let pom_path = repo_base.join("greeter-1.0.0.pom");
    fs::write(
        &pom_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <groupId>com.example</groupId>
  <artifactId>greeter</artifactId>
  <version>1.0.0</version>
  <packaging>jar</packaging>
</project>"#,
    )
    .unwrap();
    fs::write(
        tmp.path()
            .join("repo/com/example/greeter/maven-metadata.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>com.example</groupId>
  <artifactId>greeter</artifactId>
  <versioning>
    <release>1.0.0</release>
    <versions><version>1.0.0</version></versions>
  </versioning>
</metadata>"#,
    )
    .unwrap();

    // In-process static file server — no TOCTOU port race, no python3 dependency
    let repo_dir = tmp.path().join("repo");
    let repo_dir_clone = repo_dir.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let _server_handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let repo = repo_dir_clone.clone();
            std::thread::spawn(move || serve_file(stream, &repo));
        }
    });

    let app = tmp.path().join("UseDep.java");
    fs::write(
        &app,
        format!(
            r#"
//REPOS local=http://127.0.0.1:{port}
//DEPS com.example:greeter
import com.example.Greeter;
class UseDep {{
  public static void main(String[] args) {{
    System.out.print(Greeter.message());
  }}
}}
"#
        ),
    )
    .unwrap();

    let out = juv_command().arg("run").arg(&app).output().unwrap();

    // Server thread will be cleaned up when the process exits

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "deps-ok");
}

/// Minimal HTTP file server for integration tests.
/// Serves GET requests with static files from `root_dir`.
fn serve_file(mut stream: std::net::TcpStream, root_dir: &std::path::Path) {
    use std::io::{BufRead, Read, Write};
    let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
    let mut request = String::new();
    if reader.read_line(&mut request).is_err() {
        return;
    }
    // Parse path from "GET /path HTTP/1.1"
    let path = request
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .trim_start_matches('/');
    let file_path = root_dir.join(path);

    let (status, body) = if file_path.exists() && file_path.starts_with(root_dir) {
        let mut buf = Vec::new();
        let mut f = std::fs::File::open(&file_path).unwrap();
        f.read_to_end(&mut buf).unwrap();
        ("200 OK", buf)
    } else {
        ("404 Not Found", b"Not Found".to_vec())
    };

    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}
