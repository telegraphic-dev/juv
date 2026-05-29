use std::fs;
use std::process::{Command, Output};

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

fn assert_failure(out: &Output) {
    assert!(
        !out.status.success(),
        "expected failure\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn graph_dump_prints_only_javaparser_json() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example {\n    void main() {\n        String message = \"hello\";\n    }\n}\n",
    )
    .unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["!"], "com.github.javaparser.ast.CompilationUnit");
    assert!(value["types"].as_array().unwrap().iter().any(|node| {
        node["!"] == "com.github.javaparser.ast.body.ClassOrInterfaceDeclaration"
            && node["name"]["identifier"] == "Example"
    }));
    assert!(
        value.get("nodes").is_none(),
        "must not emit jbx custom graph nodes"
    );
    assert!(!String::from_utf8_lossy(&out.stdout).starts_with("jbx-graph"));
}

#[test]
fn graph_dump_supports_jbang_compact_source() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("nanocode_basic.java");
    fs::write(
        &source,
        "///usr/bin/env jbang \"$0\" \"$@\" ; exit $?\n//JAVA 25\nimport java.util.*;\n\nString greeting = \"hello\";\n\nvoid main() {\n    IO.println(greeting);\n}\n",
    )
    .unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["!"], "com.github.javaparser.ast.CompilationUnit");
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn graph_import_prints_java_source_from_javaparser_json() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    let json = tmp.path().join("Example.json");
    fs::write(
        &source,
        "class Example {\n    void main() {\n        String message = \"hello\";\n    }\n}\n",
    )
    .unwrap();

    let dump = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();
    assert_success(&dump);
    fs::write(&json, &dump.stdout).unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("import")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&json)
        .output()
        .unwrap();

    assert_success(&out);
    let java = String::from_utf8_lossy(&out.stdout);
    assert!(java.contains("class Example"), "{java}");
    assert!(java.contains("String message = \"hello\";"), "{java}");
}

#[test]
fn graph_import_writes_output_file() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    let json = tmp.path().join("Example.json");
    let output = tmp.path().join("RoundTrip.java");
    fs::write(&source, "class Example {}\n").unwrap();

    let dump = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();
    assert_success(&dump);
    fs::write(&json, &dump.stdout).unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("import")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--output")
        .arg(&output)
        .arg(&json)
        .output()
        .unwrap();

    assert_success(&out);
    assert!(
        out.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(fs::read_to_string(&output)
        .unwrap()
        .contains("class Example"));
}

#[test]
fn graph_rejects_old_custom_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(&source, "class Example {}\n").unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();

    assert_failure(&out);
}
