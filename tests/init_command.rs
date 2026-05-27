use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
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

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("Hello.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.starts_with("///usr/bin/env jbang"));
    assert!(content.contains("//JAVA 25+"));
    assert!(content.contains("void main(String... args)"));
    assert!(content.contains("IO.println(\"Hello World\")"));
    assert!(!content.contains("public class"));
}

#[test]
fn init_writes_deps_and_java_directives() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("UseDeps.java");

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--java")
        .arg("25")
        .arg("--deps")
        .arg("info.picocli:picocli:4.7.6,com.google.guava:guava:33.3.1-jre")
        .arg("UseDeps.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("//JAVA 25"));
    assert!(content.contains("//DEPS info.picocli:picocli:4.7.6"));
    assert!(content.contains("//DEPS com.google.guava:guava:33.3.1-jre"));
}

#[test]
fn init_rejects_lower_java_for_unnamed_class_templates() {
    let tmp = tempfile::tempdir().unwrap();

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--java")
        .arg("17")
        .arg("Old.java")
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("uses Java 25 unnamed classes"), "{stderr}");
}

#[test]
fn init_allows_lower_java_for_class_based_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Greet.java");

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("cli")
        .arg("--java")
        .arg("17")
        .arg("Greet.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("//JAVA 17"));
    assert!(content.contains("class Greet implements Callable<Integer>"));
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Existing.java");
    fs::write(&script, "keep me").unwrap();

    let out = juv_command()
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

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--force")
        .arg("Existing.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("void main(String... args)"));
    assert!(!content.contains("replace me"));
}

#[test]
fn init_cli_template_creates_picocli_script() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Greet.java");

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("cli")
        .arg("--java")
        .arg("25")
        .arg("Greet.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("//JAVA 25"));
    assert!(content.contains("//DEPS info.picocli:picocli:4.7.6"));
    assert!(content.contains("@Command(name = \"Greet\""));
    assert!(content.contains("class Greet implements Callable<Integer>"));
}

#[test]
fn init_agent_template_creates_java_agent_skeleton() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Spy.java");

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("agent")
        .arg("Spy.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("//JAVAAGENT\n"));
    assert!(content.contains("//MANIFEST Premain-Class=Spy"));
    assert!(content.contains("//MANIFEST Can-Redefine-Classes=true"));
    assert!(content.contains("public static void premain"));
    assert!(content.contains("public class Spy"));
}

#[test]
fn init_compact_template_uses_java_25_compact_source_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("HelloCompact.java");

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("compact")
        .arg("HelloCompact.java")
        .output()
        .unwrap();

    assert_success(&out);
    let content = fs::read_to_string(&script).unwrap();
    assert!(content.contains("//JAVA 25+"));
    assert!(content.contains("void main(String... args)"));
    assert!(content.contains("IO.println(\"Hello World\")"));
    assert!(!content.contains("public class"));
}

#[test]
fn init_rejects_unknown_template() {
    let tmp = tempfile::tempdir().unwrap();

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("bogus")
        .arg("Bad.java")
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown init template 'bogus'"), "{stderr}");
}

#[test]
fn template_list_prints_builtin_templates() {
    let out = juv_command().arg("template").arg("list").output().unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello\tBasic Java 25 unnamed-class Hello World script"));
    assert!(stdout.contains("compact\tJava 25 compact-source Hello World script"));
    assert!(stdout.contains("cli\tPicocli command-line application"));
    assert!(stdout.contains("agent\tJava agent skeleton"));
}
