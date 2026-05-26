use std::fs;
use std::process::{Command, Output};

fn doj_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_doj"))
}

fn run_doj(args: &[&std::path::Path], text_args: &[&str]) -> Output {
    let mut cmd = doj_command();
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

    let out = run_doj(&[std::path::Path::new("run"), &src], &["world"]);

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

    let out = run_doj(&[std::path::Path::new("run"), &src], &[]);

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

    let out = run_doj(&[std::path::Path::new("run"), &main], &[]);

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

    let out = run_doj(&[std::path::Path::new("run"), &main], &[]);

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

    let out = run_doj(&[std::path::Path::new("run"), &src], &[]);

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
//RUNTIME_OPTIONS -Ddoj.test.value=runtime-ok
class RuntimeProp {
  public static void main(String[] args) {
    System.out.print(System.getProperty("doj.test.value"));
  }
}
"#,
    )
    .unwrap();

    let out = run_doj(&[std::path::Path::new("run"), &src], &[]);

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

    let out = run_doj(&[std::path::Path::new("run"), &src], &[]);

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
fn resolves_deps_with_coursier_compatible_classpath_file() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
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
    let jar_path = tmp.path().join("greeter.jar");
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

    let fake_bin = tmp.path().join("bin");
    fs::create_dir_all(&fake_bin).unwrap();
    let fake_cs = fake_bin.join("cs");
    fs::write(
        &fake_cs,
        format!(
            r#"#!/bin/sh
if [ "$1" = "--help" ]; then exit 0; fi
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--classpath-file" ]; then
    shift
    printf '%s\n' '{}' > "$1"
    exit 0
  fi
  shift
done
exit 2
"#,
            jar_path.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake_cs, fs::Permissions::from_mode(0o755)).unwrap();

    let app = tmp.path().join("UseDep.java");
    fs::write(
        &app,
        r#"
//DEPS com.example:greeter:1.0.0
import com.example.Greeter;
class UseDep {
  public static void main(String[] args) {
    System.out.print(Greeter.message());
  }
}
"#,
    )
    .unwrap();

    let old_path = std::env::var("PATH").unwrap_or_default();
    let out = doj_command()
        .env("PATH", format!("{}:{old_path}", fake_bin.display()))
        .arg("run")
        .arg(&app)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "deps-ok");
}
