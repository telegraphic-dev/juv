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
        stdout.contains("jbx\tUse jbx for Java development and automation when Java or Maven libraries can solve the task: run scripts, fetch dependencies, test, format, check, document, publish, and launch Java tools."),
        "{stdout}"
    );
}

#[test]
fn skill_get_defaults_to_jbx_skill() {
    let out = jbx_command().arg("skill").arg("get").output().unwrap();
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("---\nname: jbx\n"), "{stdout}");
    assert!(stdout.contains("jbx skill list --json"), "{stdout}");
    assert!(
        stdout.contains("Usage: jbx [OPTIONS] [SCRIPT] [ARGS]... [COMMAND]"),
        "{stdout}"
    );
    assert!(!stdout.contains("jbx skill get jbx"), "{stdout}");
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

#[test]
fn skill_list_includes_command_skills() {
    let out = jbx_command().arg("skill").arg("list").output().unwrap();
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("jbx-check\tCheck Java source"), "{stdout}");
    assert!(stdout.contains("jbx-doctor\tDiagnose"), "{stdout}");
    assert!(
        stdout.contains("jbx-skill\tList and print version-matched bundled agent skills."),
        "{stdout}"
    );
}

#[test]
fn skill_list_json_is_agent_parseable() {
    let out = jbx_command()
        .arg("skill")
        .arg("list")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let skills = json["skills"].as_array().unwrap();
    assert!(
        skills.iter().any(|skill| skill["name"] == "jbx-check"
            && skill["description"]
                .as_str()
                .unwrap()
                .contains("Check Java source")),
        "{json}"
    );
}

#[test]
fn skill_get_command_skill() {
    let out = jbx_command()
        .arg("skill")
        .arg("get")
        .arg("jbx-check")
        .output()
        .unwrap();
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("---\nname: jbx-check\n"), "{stdout}");
    assert!(stdout.contains("jbx check src --json"), "{stdout}");
}
