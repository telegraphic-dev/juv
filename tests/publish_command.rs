use std::fs;
use std::process::{Command, Output};

fn juv_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_juv"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn zip_names(path: &std::path::Path) -> Vec<String> {
    let file = fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut names = Vec::new();
    for i in 0..archive.len() {
        names.push(archive.by_index(i).unwrap().name().to_string());
    }
    names
}

fn zip_entry(path: &std::path::Path, name: &str) -> String {
    let bytes = zip_entry_bytes(path, name);
    String::from_utf8(bytes).unwrap()
}

fn zip_entry_bytes(path: &std::path::Path, name: &str) -> Vec<u8> {
    let file = fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
    bytes
}

fn zip_names_from_bytes(bytes: Vec<u8>) -> Vec<String> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).unwrap();
    let mut names = Vec::new();
    for i in 0..archive.len() {
        names.push(archive.by_index(i).unwrap().name().to_string());
    }
    names
}

#[test]
fn publish_dry_run_uses_flat_id_metadata_and_version_override() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("Hello.java");
    fs::write(
        &script,
        r#"
public class Hello {
  public static void main(String[] args) {
    System.out.println(Helper.message());
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("Helper.java"),
        r#"
class Helper {
  static String message() {
    return "hello";
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "hello-tool",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo.hello",
  "description": "Hello tool",
  "url": "https://github.com/telegraphic-dev/hello-tool",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic", "organizationUrl": "https://github.com/telegraphic-dev"}],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/hello-tool.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/hello-tool.git",
    "url": "https://github.com/telegraphic-dev/hello-tool"
  },
  "sources": ["Helper.java"],
  "dependencies": ["info.picocli:picocli:4.7.7"]
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--version")
        .arg("2.0.0")
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("dev.telegraphic.demo:hello-tool:2.0.0"),
        "{stdout}"
    );
    assert!(stdout.contains("dry run"), "{stdout}");

    let names = zip_names(&bundle);
    let base = "dev/telegraphic/demo/hello-tool/2.0.0";
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-sources.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-javadoc.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom.md5")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom.sha1")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom.sha256")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0.pom.sha512")),
        "{names:?}"
    );

    let pom = zip_entry(&bundle, &format!("{base}/hello-tool-2.0.0.pom"));
    let sources_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/hello-tool-2.0.0-sources.jar"),
    ));
    assert!(
        sources_names.contains(&"dev/telegraphic/demo/hello/Hello.java".to_string()),
        "{sources_names:?}"
    );
    assert!(
        sources_names.contains(&"dev/telegraphic/demo/hello/Helper.java".to_string()),
        "{sources_names:?}"
    );
    let javadoc_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/hello-tool-2.0.0-javadoc.jar"),
    ));
    assert!(
        javadoc_names
            .iter()
            .any(|name| name.ends_with("index.html")),
        "{javadoc_names:?}"
    );
    assert!(
        javadoc_names
            .iter()
            .any(|name| name.ends_with("Hello.html") || name.ends_with("HelloTool.html")),
        "{javadoc_names:?}"
    );
    assert!(
        pom.contains("<groupId>dev.telegraphic.demo</groupId>"),
        "{pom}"
    );
    assert!(pom.contains("<artifactId>hello-tool</artifactId>"), "{pom}");
    assert!(pom.contains("<version>2.0.0</version>"), "{pom}");
    assert!(
        pom.contains("<description>Hello tool</description>"),
        "{pom}"
    );
    assert!(pom.contains("<packaging>jar</packaging>"), "{pom}");
    assert!(
        pom.contains("<url>https://github.com/telegraphic-dev/hello-tool</url>"),
        "{pom}"
    );
    assert!(pom.contains("<licenses>"), "{pom}");
    assert!(pom.contains("<name>MIT License</name>"), "{pom}");
    assert!(pom.contains("<developers>"), "{pom}");
    assert!(pom.contains("<name>Telegraphic</name>"), "{pom}");
    assert!(pom.contains("<scm>"), "{pom}");
    assert!(
        pom.contains(
            "<connection>scm:git:https://github.com/telegraphic-dev/hello-tool.git</connection>"
        ),
        "{pom}"
    );
    assert!(pom.contains("<dependencies>"), "{pom}");
    assert!(pom.contains("<groupId>info.picocli</groupId>"), "{pom}");
    assert!(pom.contains("<artifactId>picocli</artifactId>"), "{pom}");
    assert!(pom.contains("<version>4.7.7</version>"), "{pom}");
}

#[test]
fn publish_auto_discovers_local_java_sources_when_descriptor_omits_sources() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"public class Hello {
  public static void main(String[] args) {
    System.out.println(Helper.message());
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("Helper.java"),
        r#"class Helper {
  static String message() { return "auto"; }
}
"#,
    )
    .unwrap();
    fs::create_dir_all(tmp.path().join("target/generated")).unwrap();
    fs::write(
        tmp.path().join("target/generated/Ignored.java"),
        "class Ignored {}\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "auto-sources",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo.auto",
  "description": "Auto sources",
  "url": "https://github.com/telegraphic-dev/auto-sources",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic"}],
  "scm": {"connection": "scm:git:https://github.com/telegraphic-dev/auto-sources.git", "url": "https://github.com/telegraphic-dev/auto-sources"}
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("auto-bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let base = "dev/telegraphic/demo/auto-sources/1.0.0";
    let sources_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/auto-sources-1.0.0-sources.jar"),
    ));
    assert!(
        sources_names.contains(&"dev/telegraphic/demo/auto/Hello.java".to_string()),
        "{sources_names:?}"
    );
    assert!(
        sources_names.contains(&"dev/telegraphic/demo/auto/Helper.java".to_string()),
        "{sources_names:?}"
    );
    assert!(
        !sources_names
            .iter()
            .any(|name| name.ends_with("Ignored.java")),
        "{sources_names:?}"
    );
}

#[test]
fn publish_target_dir_dot_does_not_delete_unrelated_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("keep.txt"), "do not delete").unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"public class Hello {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo", "id": "safe", "version": "1.0.0",
  "package": "dev.telegraphic.demo.safe",
  "description": "Test artifact",
  "url": "https://github.com/telegraphic-dev/test-artifact",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic", "organizationUrl": "https://github.com/telegraphic-dev"}],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/test-artifact.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/test-artifact.git",
    "url": "https://github.com/telegraphic-dev/test-artifact"
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg("juv.json")
        .arg("--target-dir")
        .arg(".")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(
        fs::read_to_string(tmp.path().join("keep.txt")).unwrap(),
        "do not delete"
    );
    assert!(tmp
        .path()
        .join("repository/dev/telegraphic/demo/safe/1.0.0/safe-1.0.0.pom")
        .exists());
}

#[test]
fn publish_packages_java_compact_source_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        "void main() { IO.println(\"hello\"); }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "compact",
  "version": "1.0.0",
  "java": "25+",
  "description": "Test artifact",
  "url": "https://github.com/telegraphic-dev/test-artifact",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic", "organizationUrl": "https://github.com/telegraphic-dev"}],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/test-artifact.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/test-artifact.git",
    "url": "https://github.com/telegraphic-dev/test-artifact"
  }
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("compact-bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let names = zip_names(&bundle);
    let base = "dev/telegraphic/demo/compact/1.0.0";
    assert!(
        names.contains(&format!("{base}/compact-1.0.0.jar")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/compact-1.0.0-sources.jar")),
        "{names:?}"
    );
    let javadoc_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/compact-1.0.0-javadoc.jar"),
    ));
    assert!(
        javadoc_names
            .iter()
            .any(|name| name.ends_with("index.html")),
        "{javadoc_names:?}"
    );
    assert!(
        javadoc_names
            .iter()
            .any(|name| name.ends_with("Hello.html")),
        "{javadoc_names:?}"
    );
    assert!(
        !javadoc_names
            .iter()
            .any(|name| name.ends_with("README.txt")),
        "{javadoc_names:?}"
    );
}

#[test]
fn publish_keeps_sources_jar_paths_matching_existing_package_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"package dev.telegraphic.demo.packaged;

public class Hello {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "packaged",
  "version": "1.0.0",
  "description": "Test artifact",
  "url": "https://github.com/telegraphic-dev/test-artifact",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic", "organizationUrl": "https://github.com/telegraphic-dev"}],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/test-artifact.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/test-artifact.git",
    "url": "https://github.com/telegraphic-dev/test-artifact"
  }
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("packaged-bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let base = "dev/telegraphic/demo/packaged/1.0.0";
    let sources_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/packaged-1.0.0-sources.jar"),
    ));
    assert!(
        sources_names.contains(&"dev/telegraphic/demo/packaged/Hello.java".to_string()),
        "{sources_names:?}"
    );
}

#[test]
fn publish_resolves_sources_relative_to_descriptor_file() {
    let tmp = tempfile::tempdir().unwrap();
    let source_dir = tmp.path().join("src/main/java/dev/telegraphic/demo/nested");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(
        source_dir.join("Hello.java"),
        r#"package dev.telegraphic.demo.nested;

public class Hello {
  public static void main(String[] args) {
    System.out.println(Helper.message());
  }
}
"#,
    )
    .unwrap();
    fs::write(
        source_dir.join("Helper.java"),
        r#"package dev.telegraphic.demo.nested;

class Helper {
  static String message() { return "nested"; }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "src/main/java/dev/telegraphic/demo/nested/Hello.java",
  "sources": ["src/main/java/dev/telegraphic/demo/nested/Helper.java"],
  "group": "dev.telegraphic.demo",
  "id": "nested",
  "version": "1.0.0",
  "description": "Nested tool",
  "url": "https://github.com/telegraphic-dev/nested",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic"}],
  "scm": {"connection": "scm:git:https://github.com/telegraphic-dev/nested.git", "url": "https://github.com/telegraphic-dev/nested"}
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("nested-bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let base = "dev/telegraphic/demo/nested/1.0.0";
    let sources_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/nested-1.0.0-sources.jar"),
    ));
    assert!(
        sources_names.contains(&"dev/telegraphic/demo/nested/Helper.java".to_string()),
        "{sources_names:?}"
    );
}

#[test]
fn publish_resolves_extensionless_main_from_descriptor_to_java_file() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("hello.java"),
        "/// Hello docs.\nvoid main() { IO.println(\"hello\"); }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "group": "com.example",
  "id": "hello",
  "version": "0.0.0",
  "package": "com.example",
  "main": "hello",
  "description": "Hello tool",
  "url": "https://github.com/example/hello",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Example"}],
  "scm": {"connection": "scm:git:https://github.com/example/hello.git", "url": "https://github.com/example/hello"}
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("bundle.zip");

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let base = "com/example/hello/0.0.0";
    let sources_names = zip_names_from_bytes(zip_entry_bytes(
        &bundle,
        &format!("{base}/hello-0.0.0-sources.jar"),
    ));
    assert!(
        sources_names.contains(&"hello.java".to_string()),
        "{sources_names:?}"
    );
}

#[test]
fn publish_reports_missing_extensionless_main_with_context() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "group": "com.example",
  "id": "hello",
  "version": "0.0.0",
  "package": "com.example",
  "main": "hello",
  "description": "Hello tool",
  "url": "https://github.com/example/hello",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Example"}],
  "scm": {"connection": "scm:git:https://github.com/example/hello.git", "url": "https://github.com/example/hello"}
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("publish main source not found"), "{stderr}");
    assert!(stderr.contains("hello.java"), "{stderr}");
}

#[test]
fn publish_rejects_path_unsafe_coordinates() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"public class Hello {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{
  "main": "Hello.java",
  "group": ".tmp",
  "id": "escape",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo.safe"
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg("juv.json")
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid group"), "{stderr}");
}

#[test]
fn publish_rejects_path_unsafe_id_and_version_segments() {
    for (field, value) in [("id", "."), ("version", "..")] {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Hello.java"),
            r#"public class Hello {
  public static void main(String[] args) {}
}
"#,
        )
        .unwrap();
        fs::write(
            tmp.path().join("juv.json"),
            format!(
                r#"{{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "{}",
  "version": "{}",
  "package": "dev.telegraphic.demo.safe"
}}
"#,
                if field == "id" { value } else { "safe" },
                if field == "version" { value } else { "1.0.0" }
            ),
        )
        .unwrap();

        let out = juv_command()
            .current_dir(tmp.path())
            .arg("publish")
            .arg("--dry-run")
            .arg("--file")
            .arg("juv.json")
            .arg("--cache-dir")
            .arg(tmp.path().join("cache"))
            .output()
            .unwrap();

        assert!(!out.status.success(), "{field}={value}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(&format!("invalid {field}")), "{stderr}");
    }
}

#[test]
fn publish_requires_flat_group_id_version_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Hello.java"), "void main() {}\n").unwrap();
    fs::write(
        tmp.path().join("juv.json"),
        r#"{ "main": "Hello.java", "group": "dev.telegraphic", "version": "1.0.0" }"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("juv.json"))
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("id is required"), "{stderr}");
}
