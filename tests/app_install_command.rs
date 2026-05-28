use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn jbx() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jbx"))
}

// Use a custom bin dir to avoid polluting the user's actual PATH.
fn juv_with_home(home: &PathBuf) -> std::process::Command {
    let mut cmd = std::process::Command::new(jbx());
    cmd.env("XDG_DATA_HOME", home);
    // Ensure the jbx binary can find itself
    cmd.env(
        "PATH",
        format!(
            "{}:{}",
            jbx().parent().unwrap().display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    );
    cmd
}

fn create_test_script(dir: &std::path::Path, name: &str) -> PathBuf {
    let script = dir.join(name);
    fs::write(
        &script,
        "public class Hello {\n  public static void main(String[] args) {\n    System.out.println(\"hello\");\n  }\n}\n",
    )
    .unwrap();
    script
}

#[test]
fn app_install_creates_wrapper_script() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "install", script.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "jbx app install failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("Command installed:"),
        "unexpected stdout: {stdout}"
    );
    assert!(
        stderr.contains("PATH"),
        "expected PATH hint in stderr: {stderr}"
    );

    let installed_path = stdout.lines().next().unwrap();
    let installed = installed_path
        .trim()
        .trim_start_matches("Command installed: ");
    assert!(
        PathBuf::from(installed).exists(),
        "wrapper script does not exist at {installed}"
    );
}

#[test]
fn app_install_with_custom_name() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    let output = juv_with_home(&home.path().to_path_buf())
        .args([
            "app",
            "install",
            "--name",
            "myhello",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "jbx app install --name failed:\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("myhello"),
        "expected 'myhello' in output: {stdout}"
    );

    let installed_path = stdout
        .lines()
        .next()
        .unwrap()
        .trim()
        .trim_start_matches("Command installed: ");
    let wrapper_content = fs::read_to_string(installed_path).unwrap();
    assert!(
        wrapper_content.contains("#!/bin/sh"),
        "wrapper should be a shell script"
    );
    assert!(
        wrapper_content.contains("run --"),
        "wrapper should invoke 'run --'"
    );
}

#[test]
fn app_install_refuses_without_force() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    // First install succeeds
    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "install", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Second install without --force fails
    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "install", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "reinstall without --force should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists"),
        "expected 'already exists' error: {stderr}"
    );
}

#[test]
fn app_install_force_overwrites() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "install", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "install", "--force", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "install --force should succeed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn app_uninstall_removes_command() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    let output = juv_with_home(&home.path().to_path_buf())
        .args([
            "app",
            "install",
            "--name",
            "hello",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "uninstall", "hello"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("Command uninstalled"),
        "expected uninstall message: {stdout}"
    );
}

#[test]
fn app_uninstall_reports_missing() {
    let home = TempDir::new().unwrap();

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "uninstall", "nonexistent"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("not found"),
        "expected 'not found' message: {stdout}"
    );
}

#[test]
fn app_list_shows_installed_commands() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    // Empty list
    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No commands installed"),
        "expected empty list message: {stdout}"
    );

    // Install and list
    let output = juv_with_home(&home.path().to_path_buf())
        .args([
            "app",
            "install",
            "--name",
            "hello",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "expected 'hello' in list: {stdout}"
    );
}

#[test]
fn app_install_refuses_reserved_name() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    let script = create_test_script(script_dir.path(), "Hello.java");

    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "install", "--name", "jbx", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success(), "installing 'jbx' should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reserved"),
        "expected 'reserved' error: {stderr}"
    );
}

#[test]
fn app_install_quotes_script_path_with_spaces() {
    let home = TempDir::new().unwrap();
    let script_dir = TempDir::new().unwrap();
    // Create script in a directory with spaces
    let spaced_dir = script_dir.path().join("my projects");
    fs::create_dir_all(&spaced_dir).unwrap();
    let script = create_test_script(&spaced_dir, "Hello.java");

    let output = juv_with_home(&home.path().to_path_buf())
        .args([
            "app",
            "install",
            "--name",
            "hello",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "install with spaced path failed:\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the wrapper and verify the path is single-quoted
    let installed_path = stdout
        .lines()
        .next()
        .unwrap()
        .trim()
        .trim_start_matches("Command installed: ");
    let wrapper = fs::read_to_string(installed_path).unwrap();
    assert!(
        wrapper.contains("'"),
        "wrapper should single-quote paths with spaces: {wrapper}"
    );
    assert!(
        wrapper.contains("run --"),
        "wrapper should contain 'run --': {wrapper}"
    );

    // Verify app list can parse the quoted path back correctly
    let output = juv_with_home(&home.path().to_path_buf())
        .args(["app", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("my projects") || stdout.contains("Hello.java"),
        "app list should show the script target: {stdout}"
    );
}
