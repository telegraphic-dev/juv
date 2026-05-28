use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
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

fn write_fake_formatter(dir: &Path) {
    let bin_dir = dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let formatter = bin_dir.join("palantir-java-format");
    fs::write(
        &formatter,
        r#"#!/usr/bin/env python3
import pathlib
import sys

args = sys.argv[1:]
check = '--dry-run' in args and '--set-exit-if-changed' in args
replace = '--replace' in args
paths = [a for a in args if not a.startswith('-')]

def format_source(text):
    text = text.replace('//JAVA 25+', '// JAVA 25+')
    text = text.replace('//DEPS ', '// DEPS ')
    text = text.replace('class Example{void main(){IO.println("hi");}}', 'class Example {\n    void main() {\n        IO.println("hi");\n    }\n}')
    text = text.replace('class Nested{void ok(){}}', 'class Nested {\n    void ok() {}\n}')
    text = text.replace('void main(){IO.println("hi");}', 'void main() {\n        IO.println("hi");\n    }')
    text = text.replace('class __JuvFormatterWrapper {\n    void main() {\n        IO.println("hi");\n    }\n}', 'class __JuvFormatterWrapper {\n    void main() {\n        IO.println("hi");\n    }\n}')
    return text

if args == ['-']:
    stdin = sys.stdin.read()
    if 'abstract class ' in stdin:
        raise SystemExit(7)
    sys.stdout.write(format_source(stdin))
    raise SystemExit(0)

changed = False
for path_text in paths:
    path = pathlib.Path(path_text)
    text = path.read_text()
    formatted = format_source(text)
    if formatted != text:
        changed = True
        if replace:
            path.write_text(formatted)
if check and changed:
    raise SystemExit(1)
raise SystemExit(0)
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&formatter).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&formatter, permissions).unwrap();
    }
}

fn path_with_fake_formatter(tmp: &tempfile::TempDir) -> String {
    let existing = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(
        std::iter::once(tmp.path().join("bin")).chain(std::env::split_paths(&existing)),
    )
    .unwrap()
    .to_string_lossy()
    .to_string()
}

#[test]
fn fmt_formats_single_class_file_with_system_native_formatter() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_formatter(tmp.path());
    let source = tmp.path().join("Example.java");
    fs::write(&source, "class Example{void main(){IO.println(\"hi\");}}\n").unwrap();

    let out = juv_command()
        .arg("fmt")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .env("PATH", path_with_fake_formatter(&tmp))
        .output()
        .unwrap();

    assert_success(&out);
    let formatted = fs::read_to_string(&source).unwrap();
    assert!(formatted.contains("class Example {"), "{formatted}");
    assert!(formatted.contains("    void main() {"), "{formatted}");
}

#[test]
fn fmt_check_reports_unformatted_without_rewriting() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_formatter(tmp.path());
    let source = tmp.path().join("Example.java");
    let original = "class Example{void main(){IO.println(\"hi\");}}\n";
    fs::write(&source, original).unwrap();

    let out = juv_command()
        .arg("fmt")
        .arg("--check")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .env("PATH", path_with_fake_formatter(&tmp))
        .output()
        .unwrap();

    assert_failure(&out);
    assert_eq!(fs::read_to_string(&source).unwrap(), original);
}

#[test]
fn fmt_recurses_directories_and_skips_build_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_formatter(tmp.path());
    let src_dir = tmp.path().join("src");
    let target_dir = tmp.path().join("target");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();
    let nested = src_dir.join("Nested.java");
    let skipped = target_dir.join("Nested.java");
    fs::write(&nested, "class Nested{void ok(){}}\n").unwrap();
    fs::write(&skipped, "class Nested{void ok(){}}\n").unwrap();

    let out = juv_command()
        .arg("fmt")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(tmp.path())
        .env("PATH", path_with_fake_formatter(&tmp))
        .output()
        .unwrap();

    assert_success(&out);
    assert!(fs::read_to_string(&nested)
        .unwrap()
        .contains("class Nested {"));
    assert_eq!(
        fs::read_to_string(&skipped).unwrap(),
        "class Nested{void ok(){}}\n"
    );
}

#[test]
fn fmt_wraps_compact_source_before_formatting_and_unwraps_afterwards() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_formatter(tmp.path());
    let source = tmp.path().join("hello.java");
    fs::write(
        &source,
        "//JAVA 25+\n//DEPS com.example:demo:1.0\nimport java.util.List;\nvoid main(){IO.println(\"hi\");}\nclass Helper {}\n",
    )
    .unwrap();

    let out = juv_command()
        .arg("fmt")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .env("PATH", path_with_fake_formatter(&tmp))
        .output()
        .unwrap();

    assert_success(&out);
    let formatted = fs::read_to_string(&source).unwrap();
    assert!(
        formatted
            .starts_with("// JAVA 25+\n// DEPS com.example:demo:1.0\nimport java.util.List;\n"),
        "{formatted}"
    );
    assert!(
        formatted.contains("void main() {\n    IO.println(\"hi\");\n}"),
        "{formatted}"
    );
    assert!(formatted.contains("class Helper {}"), "{formatted}");
    assert!(!formatted.contains("__JuvFormatterWrapper"), "{formatted}");
}

#[test]
fn fmt_does_not_wrap_regular_type_declarations_with_modifiers() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_formatter(tmp.path());
    let source = tmp.path().join("Base.java");
    fs::write(&source, "abstract class Base { void main(){} }\n").unwrap();

    let out = juv_command()
        .arg("fmt")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg(&source)
        .env("PATH", path_with_fake_formatter(&tmp))
        .output()
        .unwrap();

    assert_success(&out);
}
