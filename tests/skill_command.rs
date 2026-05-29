use std::process::{Command, Output};

fn jbx_command() -> Command {
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
fn skill_list_prints_bundled_jbx_skill() {
    let out = jbx_command().arg("skill").arg("list").output().unwrap();
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("jbx\tOne-stop shop Java toolbox"),
        "{stdout}"
    );
}

#[test]
fn skill_get_defaults_to_jbx_skill() {
    let out = jbx_command().arg("skill").arg("get").output().unwrap();
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("---\nname: jbx\n"), "{stdout}");
    assert!(
        stdout.contains("curl -fsSL https://jbx.telegraphic.dev/install.sh | bash"),
        "{stdout}"
    );
    assert!(stdout.contains("jbx check [path...] [--json]"), "{stdout}");
}

#[test]
fn skill_get_named_jbx_skill() {
    let out = jbx_command()
        .arg("skill")
        .arg("get")
        .arg("jbx")
        .output()
        .unwrap();
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("name: jbx"), "{stdout}");
}
