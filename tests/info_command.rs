use std::fs;
use std::path::Path;
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

fn path_exists_under(root: &Path, suffix: &str) -> bool {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| entry.path().to_string_lossy().ends_with(suffix))
}

#[test]
fn info_classpath_prints_runtime_classpath_and_builds_script() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let extra_cp = tmp.path().join("extra-classes");
    fs::create_dir(&extra_cp).unwrap();
    let src = tmp.path().join("InfoMain.java");
    fs::write(
        &src,
        r#"
class InfoMain {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();

    let out = doj_command()
        .arg("info")
        .arg("classpath")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--class-path")
        .arg(&extra_cp)
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let entries: Vec<_> = std::env::split_paths(stdout.trim()).collect();
    assert!(
        entries.iter().any(|entry| entry.ends_with("classes")),
        "classpath should include compiled classes dir, got {stdout:?}"
    );
    assert!(
        entries.iter().any(|entry| entry == &extra_cp),
        "classpath should include explicit classpath entry, got {stdout:?}"
    );
    assert!(path_exists_under(&cache, "/classes/InfoMain.class"));
}

#[test]
fn info_classpath_deps_only_omits_compiled_classes() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let extra_cp = tmp.path().join("lib-classes");
    fs::create_dir(&extra_cp).unwrap();
    let src = tmp.path().join("InfoDepsOnly.java");
    fs::write(
        &src,
        r#"
class InfoDepsOnly {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();

    let out = doj_command()
        .arg("info")
        .arg("classpath")
        .arg("--deps-only")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--class-path")
        .arg(&extra_cp)
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let entries: Vec<_> = std::env::split_paths(stdout.trim()).collect();
    assert_eq!(entries, vec![extra_cp]);
}
