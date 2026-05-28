use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
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
fn run_resolves_alias_from_local_jbang_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"class Hello {
  public static void main(String[] args) {
    System.out.println("hello " + args[0] + " " + args[1]);
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "hello": {
      "script-ref": "Hello.java",
      "arguments": ["from-catalog"]
    }
  }
}
"#,
    )
    .unwrap();

    let output = juv_command()
        .current_dir(tmp.path())
        .arg("run")
        .arg("hello")
        .arg("from-cli")
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hello from-catalog from-cli\n"
    );
}

#[test]
fn shorthand_resolves_alias_from_parent_catalog_with_base_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("nested");
    fs::create_dir_all(tmp.path().join("scripts")).unwrap();
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        tmp.path().join("scripts/Tool.java"),
        r#"class Tool {
  public static void main(String[] args) {
    System.out.println("tool " + args[0]);
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "base-ref": "scripts",
  "aliases": {
    "tool": { "script-ref": "Tool.java" }
  }
}
"#,
    )
    .unwrap();

    let output = juv_command()
        .current_dir(&nested)
        .arg("tool")
        .arg("works")
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tool works\n");
}

#[test]
fn alias_list_prints_local_catalog_aliases() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "alpha": { "script-ref": "Alpha.java", "description": "Alpha script" },
    "beta": { "script-ref": "tools/Beta.java" }
  }
}
"#,
    )
    .unwrap();

    let output = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("list")
        .output()
        .unwrap();

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha\tAlpha.java\tAlpha script"),
        "{stdout}"
    );
    assert!(stdout.contains("beta\ttools/Beta.java"), "{stdout}");
}

#[test]
fn alias_list_json_prints_complete_alias_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "alpha": {
      "script-ref": "Alpha.java",
      "description": "Alpha script",
      "arguments": ["one"],
      "dependencies": ["org.example:lib:1.0"],
      "repositories": ["central"],
      "sources": ["Helper.java"],
      "files": ["data.txt"],
      "classpaths": ["libs/local.jar"],
      "compile-options": ["--release", "25"],
      "runtime-options": ["-Dalpha=1"],
      "java": "25+",
      "main": "Alpha"
    },
    "beta": { "script-ref": "Beta.java" }
  }
}
"#,
    )
    .unwrap();

    let output = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("list")
        .arg("--json")
        .output()
        .unwrap();

    assert_success(&output);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let alpha = &json[0];
    assert_eq!(alpha["name"], "alpha");
    assert_eq!(alpha["scriptRef"], "Alpha.java");
    assert_eq!(alpha["description"], "Alpha script");
    assert_eq!(alpha["arguments"], serde_json::json!(["one"]));
    assert_eq!(
        alpha["dependencies"],
        serde_json::json!(["org.example:lib:1.0"])
    );
    assert_eq!(alpha["repositories"], serde_json::json!(["central"]));
    assert_eq!(alpha["sources"], serde_json::json!(["Helper.java"]));
    assert_eq!(alpha["files"], serde_json::json!(["data.txt"]));
    assert_eq!(alpha["classpaths"], serde_json::json!(["libs/local.jar"]));
    assert_eq!(
        alpha["compileOptions"],
        serde_json::json!(["--release", "25"])
    );
    assert_eq!(alpha["runtimeOptions"], serde_json::json!(["-Dalpha=1"]));
    assert_eq!(alpha["javaVersion"], "25+");
    assert_eq!(alpha["mainClass"], "Alpha");
    assert!(json[1]["description"].is_null());
}

#[test]
fn alias_add_creates_catalog_entry_with_metadata_and_runs() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"class Hello {
  public static void main(String[] args) {
    System.out.println("hello " + args[0] + " " + args[1]);
  }
}
"#,
    )
    .unwrap();

    let add = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("--name")
        .arg("hello")
        .arg("--description")
        .arg("Friendly hello")
        .arg("--java")
        .arg("25")
        .arg("--main")
        .arg("Hello")
        .arg("--runtime-option")
        .arg("-Dhello=true")
        .arg("Hello.java")
        .arg("from-alias")
        .output()
        .unwrap();
    assert_success(&add);

    let catalog: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join("jbang-catalog.json")).unwrap())
            .unwrap();
    let alias = &catalog["aliases"]["hello"];
    assert_eq!(alias["script-ref"], "Hello.java");
    assert_eq!(alias["description"], "Friendly hello");
    assert_eq!(alias["java"], "25");
    assert_eq!(alias["main"], "Hello");
    assert_eq!(alias["arguments"], serde_json::json!(["from-alias"]));
    assert_eq!(
        alias["runtime-options"],
        serde_json::json!(["-Dhello=true"])
    );

    let run = juv_command()
        .current_dir(tmp.path())
        .arg("hello")
        .arg("from-cli")
        .output()
        .unwrap();
    assert_success(&run);
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "hello from-alias from-cli\n"
    );
}

#[test]
fn alias_add_uses_file_option_and_requires_force_to_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog = tmp.path().join("custom-catalog.json");

    let first = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("--file")
        .arg(&catalog)
        .arg("One.java")
        .output()
        .unwrap();
    assert_success(&first);

    let duplicate = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("--file")
        .arg(&catalog)
        .arg("One.java")
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("use --force"));

    let forced = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("--file")
        .arg(&catalog)
        .arg("--force")
        .arg("--name")
        .arg("One")
        .arg("--description")
        .arg("Updated")
        .arg("Other.java")
        .output()
        .unwrap();
    assert_success(&forced);

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(catalog).unwrap()).unwrap();
    assert_eq!(json["aliases"]["One"]["script-ref"], "Other.java");
    assert_eq!(json["aliases"]["One"]["description"], "Updated");
}

#[test]
fn alias_remove_deletes_alias_from_nearest_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "keep": { "script-ref": "Keep.java" },
    "drop": { "script-ref": "Drop.java" }
  }
}
"#,
    )
    .unwrap();

    let output = juv_command()
        .current_dir(&nested)
        .arg("alias")
        .arg("remove")
        .arg("drop")
        .output()
        .unwrap();
    assert_success(&output);

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join("jbang-catalog.json")).unwrap())
            .unwrap();
    assert!(json["aliases"]["drop"].is_null());
    assert_eq!(json["aliases"]["keep"]["script-ref"], "Keep.java");
}

#[test]
fn alias_add_rejects_global_and_file_together() {
    let tmp = tempfile::tempdir().unwrap();
    let output = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("--global")
        .arg("--file")
        .arg(tmp.path().join("custom.json"))
        .arg("Hello.java")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
}

#[test]
fn alias_add_rejects_non_object_catalog_without_clobbering() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog = tmp.path().join("jbang-catalog.json");
    fs::write(&catalog, "null\n").unwrap();

    let output = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("Hello.java")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("catalog root is not a JSON object"));
    assert_eq!(fs::read_to_string(catalog).unwrap(), "null\n");
}

#[test]
fn alias_add_infers_name_from_jbang_script_extensions() {
    let tmp = tempfile::tempdir().unwrap();
    let output = juv_command()
        .current_dir(tmp.path())
        .arg("alias")
        .arg("add")
        .arg("scripts/Hello.kt")
        .output()
        .unwrap();
    assert_success(&output);

    let catalog: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join("jbang-catalog.json")).unwrap())
            .unwrap();
    assert_eq!(
        catalog["aliases"]["Hello"]["script-ref"],
        "scripts/Hello.kt"
    );
}
