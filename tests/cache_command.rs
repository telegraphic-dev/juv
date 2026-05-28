use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
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
fn cache_clear_removes_compiled_script_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let script = tmp.path().join("Hello.java");
    fs::write(
        &script,
        r#"
public class Hello {
    public static void main(String... args) {
        System.out.println("hello");
    }
}
"#,
    )
    .unwrap();

    let build = juv_command()
        .current_dir(tmp.path())
        .arg("build")
        .arg("--cache-dir")
        .arg(&cache)
        .arg(&script)
        .output()
        .unwrap();
    assert_success(&build);
    assert!(cache.exists());
    assert!(
        walkdir_count(&cache) > 1,
        "build should populate cache directory"
    );

    let clear = juv_command()
        .current_dir(tmp.path())
        .arg("cache")
        .arg("clear")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();
    assert_success(&clear);

    assert!(!cache.exists() || walkdir_count(&cache) == 0);
}

#[test]
fn cache_clear_is_idempotent_when_cache_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("missing-cache");

    let clear = juv_command()
        .current_dir(tmp.path())
        .arg("cache")
        .arg("clear")
        .arg("--cache-dir")
        .arg(&cache)
        .output()
        .unwrap();

    assert_success(&clear);
    assert!(!cache.exists());
}

fn walkdir_count(path: &std::path::Path) -> usize {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .count()
}
