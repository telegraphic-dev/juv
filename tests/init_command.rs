use std::fs;
use std::process::{Command, Output};

fn doj_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_doj"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn init_creates_default_java_script_from_filename() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Hello.java");

    let out = doj_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("Hello.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.starts_with("///usr/bin/env jbang"));
    assert!(content.contains("public class Hello"));
    assert!(content.contains("public static void main(String... args)"));
    assert!(content.contains("out.println(\"Hello World\")"));
}

#[test]
fn init_writes_deps_and_java_directives() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("UseDeps.java");

    let out = doj_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--java")
        .arg("17")
        .arg("--deps")
        .arg("info.picocli:picocli:4.7.6,com.google.guava:guava:33.3.1-jre")
        .arg("UseDeps.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("//JAVA 17"));
    assert!(content.contains("//DEPS info.picocli:picocli:4.7.6"));
    assert!(content.contains("//DEPS com.google.guava:guava:33.3.1-jre"));
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Existing.java");
    fs::write(&script, "keep me").unwrap();

    let out = doj_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("Existing.java")
        .output()
        .unwrap();

    assert!(!out.status.success());
    assert_eq!(fs::read_to_string(&script).unwrap(), "keep me");
}

#[test]
fn init_force_overwrites_existing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Existing.java");
    fs::write(&script, "replace me").unwrap();

    let out = doj_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--force")
        .arg("Existing.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("public class Existing"));
    assert!(!content.contains("replace me"));
}
