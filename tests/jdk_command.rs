use std::process::Command;

fn jbx_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
}

fn jbx_output(args: &[&str]) -> String {
    let output = jbx_command()
        .args(args)
        .output()
        .expect("failed to run jbx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        panic!(
            "jbx {} failed:\nstdout={stdout}\nstderr={stderr}",
            args.join(" ")
        );
    }
    stdout.to_string()
}

#[test]
fn jdk_list_shows_discovered_jdks() {
    // At minimum, the system JDK should be discoverable
    let output = jbx_output(&["jdk", "list"]);
    // Should contain at least one JDK with a version number
    assert!(
        output
            .lines()
            .any(|line| line.starts_with(char::is_numeric)),
        "jdk list should show at least one JDK, got: {output}"
    );
}

#[test]
fn jdk_home_returns_path_for_baseline_jdk() {
    // CI installs Java 25, and jbx's baseline/default is Java 25.
    let output = jbx_output(&["jdk", "home", "25"]);
    assert!(
        output.contains("jdks") || output.contains("jvm") || output.contains("Java"),
        "jdk home 25 should return a JDK path, got: {output}"
    );
}

#[test]
fn jdk_home_accepts_jbang_plus_version_selector() {
    let plain = jbx_output(&["jdk", "home", "25"]);
    let plus = jbx_output(&["jdk", "home", "25+"]);
    assert_eq!(
        plain.trim(),
        plus.trim(),
        "jdk home 25+ should resolve to the same path as jdk home 25"
    );
}

#[test]
fn jdk_home_fails_for_missing_version_without_auto_install() {
    // Version 999 doesn't exist; without auto-install it should fail
    let output = jbx_command()
        .args(["jdk", "home", "999"])
        .output()
        .expect("failed to run jbx");
    assert!(
        !output.status.success(),
        "jdk home 999 should fail without auto-install"
    );
}

#[test]
fn jdk_home_defaults_to_25() {
    // No version specified should default to 25
    let output = jbx_output(&["jdk", "home"]);
    // The output should contain "25" somewhere in the path
    assert!(
        output.contains("25"),
        "jdk home (default) should reference JDK 25, got: {output}"
    );
}

#[test]
fn jdk_symlink_cache_avoids_re_search() {
    // After calling jdk home, the cache should have a valid entry.
    let output = jbx_output(&["jdk", "home", "25"]);
    let _jdk_path = output.trim();

    let cache_dir = dirs::cache_dir().unwrap().join("jbx").join("jdks");
    let link = cache_dir.join("25");
    assert!(link.exists(), "JDK cache should exist for JDK 25");

    // The symlink target should match what jdk home returned
    // (either it's a symlink pointing to the right place or it IS the JDK)
    let resolved = if link.is_symlink() {
        std::fs::read_link(&link).unwrap_or_else(|_| link.clone())
    } else {
        std::fs::canonicalize(&link).unwrap_or_else(|_| link.clone())
    };
    assert!(
        resolved.join("bin").join("java").is_file(),
        "symlink target should have bin/java, got: {}",
        resolved.display()
    );
    assert!(
        resolved.join("bin").join("javac").is_file(),
        "symlink target should have bin/javac, got: {}",
        resolved.display()
    );
    assert!(
        resolved.join("release").is_file(),
        "symlink target should have a release file, got: {}",
        resolved.display()
    );
}
#[cfg(unix)]
#[test]
fn jdk_home_removes_stale_cache_entry_that_is_not_jdk_root() {
    let cache_dir = dirs::cache_dir().unwrap().join("jbx").join("jdks");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let stale = cache_dir.join("999");
    let _ = std::fs::remove_file(&stale);
    std::os::unix::fs::symlink("/usr", &stale).unwrap();

    let output = jbx_command()
        .args(["jdk", "home", "999"])
        .output()
        .expect("failed to run jbx");

    assert!(
        !output.status.success(),
        "stale /usr symlink must not be accepted as a JDK"
    );
    assert!(!stale.exists(), "stale cache entry should be removed");
}
