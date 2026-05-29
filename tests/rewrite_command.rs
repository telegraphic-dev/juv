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

#[test]
fn rewrite_patch_writes_patch_without_changing_source() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    let original = "class Example{void main(){System.out.println(\"hi\");}}\n";
    fs::write(&source, original).unwrap();

    let out = jbx_command()
        .arg("rewrite")
        .arg("patch")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--recipe")
        .arg("auto-format")
        .arg("--source")
        .arg(&source)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(fs::read_to_string(&source).unwrap(), original);
    let patch = fs::read_to_string(tmp.path().join("rewrite/rewrite.patch")).unwrap();
    assert!(patch.contains("class Example"), "{patch}");
    assert!(patch.contains("+    void main()"), "{patch}");
}

#[test]
fn rewrite_apply_updates_source_and_accepts_module_shorthand() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example{void main(){System.out.println(\"hi\");}}\n",
    )
    .unwrap();

    let out = jbx_command()
        .arg("rewrite")
        .arg("apply")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--module")
        .arg("java")
        .arg("--recipe")
        .arg("auto-format")
        .arg("--source")
        .arg(&source)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let updated = fs::read_to_string(&source).unwrap();
    assert!(updated.contains("class Example {"), "{updated}");
    assert!(updated.contains("    void main()"), "{updated}");
}

#[test]
fn rewrite_apply_passes_recipe_options_for_parameterized_recipes() {
    let tmp = tempfile::tempdir().unwrap();
    let source_dir = tmp.path().join("src/main/java/com/old");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("Example.java");
    fs::write(
        &source,
        "package com.old;\n\nclass Example {\n    com.old.Other other;\n}\n",
    )
    .unwrap();

    let out = jbx_command()
        .arg("rewrite")
        .arg("apply")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--recipe")
        .arg("change-package")
        .arg("--option")
        .arg("old=com.old")
        .arg("--option")
        .arg("new=com.fresh")
        .arg("--source")
        .arg(tmp.path().join("src/main/java"))
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let rewritten = tmp.path().join("src/main/java/com/fresh/Example.java");
    let updated = fs::read_to_string(&rewritten).unwrap();
    assert!(updated.contains("package com.fresh;"), "{updated}");
    assert!(updated.contains("com.fresh.Other"), "{updated}");
}

#[test]
fn rewrite_modules_lists_and_searches_standard_modules() {
    let out = jbx_command()
        .arg("rewrite")
        .arg("modules")
        .arg("--search")
        .arg("maven")
        .arg("--limit")
        .arg("1")
        .arg("--json")
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"short\":\"maven\""), "{stdout}");
    assert!(stdout.contains("org.openrewrite:rewrite-maven"), "{stdout}");
}

#[test]
fn rewrite_recipes_lists_recipe_short_names() {
    let tmp = tempfile::tempdir().unwrap();
    let out = jbx_command()
        .arg("rewrite")
        .arg("recipes")
        .arg("java")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--search")
        .arg("auto-format")
        .arg("--limit")
        .arg("1")
        .arg("--json")
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"short\":\"auto-format\""), "{stdout}");
    assert!(
        stdout.contains("org.openrewrite.java.format.AutoFormat"),
        "{stdout}"
    );
}
