use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
}

fn run_juv(args: &[&std::path::Path], text_args: &[&str]) -> Output {
    let mut cmd = juv_command();
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

    let out = run_juv(&[std::path::Path::new("run"), &src], &["world"]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello world");
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

    let out = run_juv(&[std::path::Path::new("run"), &src], &[]);

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

    let out = run_juv(&[std::path::Path::new("run"), &main], &[]);

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "from helper");
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

    let out = run_juv(&[std::path::Path::new("run"), &main], &[]);

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

    let out = run_juv(&[std::path::Path::new("run"), &src], &[]);

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

    let out = run_juv(&[std::path::Path::new("run"), &src], &[]);

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

    let out = run_juv(&[std::path::Path::new("run"), &src], &[]);

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
    use std::os::unix::fs::PermissionsExt;

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

    // Write a POM for the artifact
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

    // Start a tiny HTTP server to serve the local Maven repo
    let port = 19876;
    let server_script = tmp.path().join("serve.sh");
    let repo_dir = tmp.path().join("repo");
    fs::write(
        &server_script,
        format!(
            "#!/bin/sh\nexec python3 -m http.server {port} --bind 127.0.0.1 --directory '{}'",
            repo_dir.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&server_script, fs::Permissions::from_mode(0o755)).unwrap();

    #[allow(clippy::zombie_processes)]
    let mut server = Command::new(&server_script)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // Wait for server to be ready
    let mut ready = false;
    for _ in 0..50 {
        if ureq::get(&format!("http://127.0.0.1:{port}/"))
            .call()
            .is_ok()
        {
            ready = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(ready, "local Maven repo server did not start");

    let app = tmp.path().join("UseDep.java");
    fs::write(
        &app,
        format!(
            r#"
//REPOS local=http://127.0.0.1:{port}
//DEPS com.example:greeter:1.0.0
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

    // Clean up server
    let _ = kill(server.id());
    let _ = server.wait();

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "deps-ok");
}

fn kill(pid: u32) -> std::io::Result<()> {
    Command::new("kill").arg(pid.to_string()).output()?;
    Ok(())
}
