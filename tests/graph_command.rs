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

fn graph_hash(output: &str) -> String {
    output
        .lines()
        .find_map(|line| line.strip_prefix("graph-hash "))
        .expect("graph hash line")
        .to_string()
}

#[test]
fn graph_dump_prints_stable_agent_friendly_ast_nodes() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example {\n    void main() {\n        String message = \"hello\";\n        IO.println(message);\n    }\n}\n",
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("jbx-graph v1\ngraph-hash "), "{stdout}");
    assert!(stdout.contains("kind=class name=\"Example\""), "{stdout}");
    assert!(stdout.contains("kind=method name=\"main\""), "{stdout}");
    assert!(
        stdout.contains("kind=variable name=\"message\""),
        "{stdout}"
    );
    assert!(stdout.contains("kind=literal value=\"hello\""), "{stdout}");
}

#[test]
fn graph_dump_escapes_tabs_in_literal_values() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example {\n    void main() {\n        String message = \"hello\\tthere\";\n    }\n}\n",
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("kind=literal value=\"hello\\tthere\""),
        "{stdout}"
    );
    assert!(!stdout.contains("hello\tthere"), "{stdout}");
}

#[test]
fn graph_dump_handles_jbang_java25_compact_source() {
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
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "variable" && node["name"] == "greeting"));
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "method" && node["name"] == "main"));
}
#[test]
fn graph_dump_handles_java25_module_import_compact_source() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("nanocode_module.java");
    fs::write(
        &source,
        "//JAVA 25\nimport module java.base;\n\nvoid main() {\n    IO.println(\"hello\");\n}\n",
    )
    .unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "method" && node["name"] == "main"));
}

#[test]
fn graph_dump_handles_compact_source_without_openrewrite_slf4j_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("nanocode_basic.java");
    fs::write(
        &source,
        "void main() {\n    String message = \"hello\";\n    IO.println(message);\n}\n",
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.starts_with("jbx-graph v1\ngraph-hash "), "{stdout}");
    assert!(stdout.contains("kind=method name=\"main\""), "{stdout}");
    assert!(
        stdout.contains("kind=variable name=\"message\""),
        "{stdout}"
    );
    assert!(stdout.contains("kind=literal value=\"hello\""), "{stdout}");
    assert!(stdout.contains("kind=call name=\"println\""), "{stdout}");
    assert!(!stdout.contains("NoClassDefFoundError"), "{stdout}");
    assert!(!stderr.contains("NoClassDefFoundError"), "{stderr}");
}

#[test]
fn graph_dump_detects_compact_source_when_top_level_string_contains_brace() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("nanocode_brace.java");
    fs::write(
        &source,
        "String template = \"prefix {\";\n\nvoid main() {\n    IO.println(template);\n}\n",
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("kind=method name=\"main\""), "{stdout}");
    assert!(
        stdout.contains("kind=variable name=\"template\""),
        "{stdout}"
    );
    assert!(
        stdout.contains("kind=literal value=\"prefix {\""),
        "{stdout}"
    );
}

#[test]
fn graph_dump_json_prints_ast_nodes() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("nanocode_basic.java");
    fs::write(
        &source,
        "void main() {\n    String message = \"hello\";\n    IO.println(message);\n}\n",
    )
    .unwrap();

    let out = jbx_command()
        .arg("graph")
        .arg("dump")
        .arg("--json")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["version"], "jbx-graph v1");
    assert_eq!(value["path"], source.to_string_lossy().as_ref());
    assert!(value["graphHash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "method" && node["name"] == "main"));
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "literal" && node["value"] == "hello"));
}

#[test]
fn graph_patch_updates_string_literal_through_openrewrite_ast() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example {\n    void main() {\n        String message = \"hello\";\n        IO.println(message);\n    }\n}\n",
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
    let stdout = String::from_utf8_lossy(&dump.stdout);
    let hash = graph_hash(&stdout);

    let out = jbx_command()
        .arg("graph")
        .arg("patch")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--expect-graph-hash")
        .arg(hash)
        .arg("--op")
        .arg("set node=\"#literal-1\" field=\"value\" expect=\"hello\" value=\"goodbye\"")
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let updated = fs::read_to_string(&source).unwrap();
    assert!(updated.contains("\"goodbye\""), "{updated}");
    assert!(!updated.contains("\"hello\""), "{updated}");
}

#[test]
fn graph_patch_updates_compact_source_without_leaking_wrapper() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("nanocode_basic.java");
    fs::write(
        &source,
        "void main() {\n    String message = \"hello\";\n    IO.println(message);\n}\n",
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
    let stdout = String::from_utf8_lossy(&dump.stdout);
    let hash = graph_hash(&stdout);

    let out = jbx_command()
        .arg("graph")
        .arg("patch")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--expect-graph-hash")
        .arg(hash)
        .arg("--op")
        .arg("set node=\"#literal-1\" field=\"value\" expect=\"hello\" value=\"goodbye\"")
        .arg(&source)
        .output()
        .unwrap();

    assert_success(&out);
    let updated = fs::read_to_string(&source).unwrap();
    assert!(updated.contains("\"goodbye\""), "{updated}");
    assert!(!updated.contains("__JbxCompactSource"), "{updated}");
    assert!(updated.trim_start().starts_with("void main()"), "{updated}");
}

#[test]
fn graph_patch_rejects_non_string_literals() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example {\n    int answer() {\n        return 42;\n    }\n}\n",
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
    let stdout = String::from_utf8_lossy(&dump.stdout);
    let hash = graph_hash(&stdout);

    let out = jbx_command()
        .arg("graph")
        .arg("patch")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--expect-graph-hash")
        .arg(hash)
        .arg("--op")
        .arg("set node=\"#literal-1\" field=\"value\" expect=\"42\" value=\"99\"")
        .arg(&source)
        .output()
        .unwrap();

    assert_failure(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not a string literal"), "{stderr}");
    assert!(fs::read_to_string(&source).unwrap().contains("return 42;"));
}
