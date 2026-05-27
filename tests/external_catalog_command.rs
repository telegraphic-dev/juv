use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn catalog_add_and_list_register_external_catalogs() {
    let tmp = tempfile::tempdir().unwrap();
    let external = tmp.path().join("external");
    fs::create_dir_all(&external).unwrap();
    fs::write(
        external.join("jbang-catalog.json"),
        r#"{
  "description": "Team catalog",
  "aliases": {
    "remotehello": { "script-ref": "RemoteHello.java", "description": "Remote hello" }
  }
}
"#,
    )
    .unwrap();

    let add = juv_command()
        .current_dir(tmp.path())
        .arg("catalog")
        .arg("add")
        .arg("team")
        .arg(external.join("jbang-catalog.json"))
        .arg("--import")
        .output()
        .unwrap();
    assert_success(&add);

    let catalog = fs::read_to_string(tmp.path().join("jbang-catalog.json")).unwrap();
    assert!(catalog.contains("\"catalog-ref\""), "{catalog}");
    assert!(catalog.contains("\"import\": true"), "{catalog}");

    let list = juv_command()
        .current_dir(tmp.path())
        .arg("catalog")
        .arg("list")
        .output()
        .unwrap();
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("team"), "{stdout}");
    assert!(stdout.contains("Team catalog"), "{stdout}");
}

#[test]
fn imported_catalog_aliases_are_listed_and_runnable() {
    let tmp = tempfile::tempdir().unwrap();
    let external = tmp.path().join("external");
    fs::create_dir_all(&external).unwrap();
    fs::write(
        external.join("RemoteHello.java"),
        r#"class RemoteHello {
  public static void main(String[] args) {
    System.out.println("external " + args[0]);
  }
}
"#,
    )
    .unwrap();
    fs::write(
        external.join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "remotehello": { "script-ref": "RemoteHello.java", "description": "Remote hello" }
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        format!(
            r#"{{
  "catalogs": {{
    "team": {{ "catalog-ref": "{}", "import": true }}
  }}
}}
"#,
            external.join("jbang-catalog.json").display()
        ),
    )
    .unwrap();

    let list = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("list")
        .output()
        .unwrap();
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("remotehello\tRemoteHello.java\tRemote hello"),
        "{stdout}"
    );

    let run = juv_command()
        .current_dir(tmp.path())
        .arg("remotehello")
        .arg("works")
        .output()
        .unwrap();
    assert_success(&run);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "external works\n");
}

#[test]
fn imported_catalog_templates_are_listed_and_rendered_by_init() {
    let tmp = tempfile::tempdir().unwrap();
    let external = tmp.path().join("external");
    fs::create_dir_all(external.join("templates")).unwrap();
    fs::write(
        external.join("templates/tool.java"),
        r#"//JAVA 25+
void main(String... args) {
    IO.println("{{greeting}} {{baseName}}");
}
"#,
    )
    .unwrap();
    fs::write(
        external.join("jbang-catalog.json"),
        r#"{
  "templates": {
    "tool": {
      "description": "Team tool template",
      "file-refs": { "{basename}.java": "templates/tool.java" },
      "properties": {
        "greeting": { "default": "Hello" }
      }
    }
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        format!(
            r#"{{
  "catalogs": {{
    "team": {{ "catalog-ref": "{}", "import": true }}
  }}
}}
"#,
            external.join("jbang-catalog.json").display()
        ),
    )
    .unwrap();

    let list = juv_command()
        .current_dir(tmp.path())
        .arg("template")
        .arg("list")
        .output()
        .unwrap();
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("tool\tTeam tool template"), "{stdout}");

    let init = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("tool")
        .arg("MyTool.java")
        .output()
        .unwrap();
    assert_success(&init);
    let script = fs::read_to_string(tmp.path().join("MyTool.java")).unwrap();
    assert!(script.contains("IO.println(\"Hello MyTool\")"), "{script}");
}

#[test]
fn malformed_catalog_template_fails_with_clear_error() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "templates": {
    "broken": { "description": "Missing file refs" }
  }
}
"#,
    )
    .unwrap();

    let init = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("broken")
        .arg("Broken.java")
        .output()
        .unwrap();

    assert!(!init.status.success());
    let stderr = String::from_utf8_lossy(&init.stderr);
    assert!(
        stderr.contains("template 'broken' does not define file-refs"),
        "{stderr}"
    );
    assert!(!tmp.path().join("Broken.java").exists());
}

#[test]
fn template_properties_without_defaults_fail_before_writing_broken_source() {
    let tmp = tempfile::tempdir().unwrap();
    let external = tmp.path().join("external");
    fs::create_dir_all(external.join("templates")).unwrap();
    fs::write(
        external.join("templates/tool.java"),
        r#"//JAVA 25+
void main() {
    IO.println("{{greeting}}");
}
"#,
    )
    .unwrap();
    fs::write(
        external.join("jbang-catalog.json"),
        r#"{
  "templates": {
    "tool": {
      "file-refs": { "{basename}.java": "templates/tool.java" },
      "properties": { "greeting": { "description": "Greeting to print" } }
    }
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        format!(
            r#"{{
  "catalogs": {{
    "team": {{ "catalog-ref": "{}", "import": true }}
  }}
}}
"#,
            external.join("jbang-catalog.json").display()
        ),
    )
    .unwrap();

    let init = juv_command()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--template")
        .arg("tool")
        .arg("Tool.java")
        .output()
        .unwrap();

    assert!(!init.status.success());
    let stderr = String::from_utf8_lossy(&init.stderr);
    assert!(
        stderr.contains("template property 'greeting' has no default value"),
        "{stderr}"
    );
    assert!(!tmp.path().join("Tool.java").exists());
}

#[test]
fn catalog_add_prefetches_description_using_catalog_file_relative_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let external = tmp.path().join("external");
    let catalogs = tmp.path().join("catalogs");
    fs::create_dir_all(&external).unwrap();
    fs::create_dir_all(&catalogs).unwrap();
    fs::write(
        external.join("jbang-catalog.json"),
        r#"{ "description": "Sibling catalog" }
"#,
    )
    .unwrap();

    let add = juv_command()
        .current_dir(tmp.path())
        .arg("catalog")
        .arg("add")
        .arg("sibling")
        .arg("../external")
        .arg("--file")
        .arg(catalogs.join("jbang-catalog.json"))
        .output()
        .unwrap();
    assert_success(&add);

    let list = juv_command()
        .current_dir(&catalogs)
        .arg("catalog")
        .arg("list")
        .output()
        .unwrap();
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("sibling\t../external\tSibling catalog"),
        "{stdout}"
    );
}

#[test]
fn self_imported_catalog_with_relative_dot_path_does_not_recurse_forever() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"class Hello {
  public static void main(String[] args) {
    System.out.println("hello");
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "hello": { "script-ref": "Hello.java", "description": "Hello" }
  },
  "catalogs": {
    "self": { "catalog-ref": "./jbang-catalog.json", "import": true }
  }
}
"#,
    )
    .unwrap();

    let list = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("list")
        .output()
        .unwrap();
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert_eq!(
        stdout.matches("hello\tHello.java\tHello").count(),
        1,
        "{stdout}"
    );
}
