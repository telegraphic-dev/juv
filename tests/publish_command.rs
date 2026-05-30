use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

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

fn zip_entry_from_bytes(bytes: Vec<u8>, name: &str) -> String {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
    content
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
        names.contains(&format!("{base}/hello-tool-2.0.0-jbx-docs.md")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-jbx-docs.json")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-jbx-docs.md.sha256")),
        "{names:?}"
    );
    assert!(
        names.contains(&format!("{base}/hello-tool-2.0.0-jbx-docs.json.sha256")),
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
    let manifest = zip_entry_from_bytes(
        zip_entry_bytes(&bundle, &format!("{base}/hello-tool-2.0.0.jar")),
        "META-INF/MANIFEST.MF",
    );
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
        manifest.contains("Main-Class: dev.telegraphic.demo.hello.Hello"),
        "{manifest}"
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
fn publish_renders_runtime_dependencies_with_runtime_scope() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"public class Hello {
  public static void main(String[] args) {
    System.out.println("hello");
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbx.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "runtime-tool",
  "version": "1.0.0",
  "description": "Runtime tool",
  "url": "https://github.com/telegraphic-dev/runtime-tool",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic"}],
  "scm": {"connection": "scm:git:https://github.com/telegraphic-dev/runtime-tool.git", "url": "https://github.com/telegraphic-dev/runtime-tool"},
  "runtimeDependencies": ["org.slf4j:slf4j-nop:2.0.17"]
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
        .arg(tmp.path().join("jbx.json"))
        .arg("--output")
        .arg(&bundle)
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .output()
        .unwrap();

    assert_success(&out);
    let base = "dev/telegraphic/demo/runtime-tool/1.0.0";
    let pom = zip_entry(&bundle, &format!("{base}/runtime-tool-1.0.0.pom"));
    assert!(pom.contains("<groupId>org.slf4j</groupId>"), "{pom}");
    assert!(pom.contains("<artifactId>slf4j-nop</artifactId>"), "{pom}");
    assert!(pom.contains("<version>2.0.17</version>"), "{pom}");
    assert!(pom.contains("<scope>runtime</scope>"), "{pom}");
}

#[test]
fn publish_defaults_to_jbx_json_descriptor() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        r#"public class Hello {
  public static void main(String[] args) {
    System.out.println("hello jbx");
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbx.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "jbx-tool",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo.jbx",
  "description": "JBX tool",
  "url": "https://github.com/telegraphic-dev/jbx-tool",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic"}],
  "scm": {"connection": "scm:git:https://github.com/telegraphic-dev/jbx-tool.git", "url": "https://github.com/telegraphic-dev/jbx-tool"}
}
"#,
    )
    .unwrap();
    let bundle = tmp.path().join("bundle.zip");

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
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
        stdout.contains("dev.telegraphic.demo:jbx-tool:1.0.0"),
        "{stdout}"
    );
    let names = zip_names(&bundle);
    assert!(
        names.contains(&"dev/telegraphic/demo/jbx-tool/1.0.0/jbx-tool-1.0.0.pom".to_string()),
        "{names:?}"
    );
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
        tmp.path().join("jbx.json"),
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
        .arg("jbx.json")
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
fn publish_resolves_main_fqn_from_descriptor_to_java_source() {
    let tmp = tempfile::tempdir().unwrap();
    let source_dir = tmp.path().join("src/main/java/com/example/app");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(
        source_dir.join("Hello.java"),
        r#"package com.example.app;
public class Hello {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbx.json"),
        r#"{
  "group": "com.example",
  "id": "hello",
  "version": "0.0.0",
  "package": "com.example.app",
  "main": "com.example.app.Hello",
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
        .arg(tmp.path().join("jbx.json"))
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
        sources_names.contains(&"com/example/app/Hello.java".to_string()),
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
        tmp.path().join("jbx.json"),
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
        .arg(tmp.path().join("jbx.json"))
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
        tmp.path().join("jbx.json"),
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
        .arg("jbx.json")
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
            tmp.path().join("jbx.json"),
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
            .arg("jbx.json")
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
        tmp.path().join("jbx.json"),
        r#"{ "main": "Hello.java", "group": "dev.telegraphic", "version": "1.0.0" }"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--skip-signing")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
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

#[derive(Default, Debug)]
struct RecordedCentralRequests {
    upload: Option<String>,
    status: Option<String>,
}

fn start_mock_central(states: Vec<&'static str>) -> (String, Arc<Mutex<RecordedCentralRequests>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let recorded = Arc::new(Mutex::new(RecordedCentralRequests::default()));
    let recorded_thread = Arc::clone(&recorded);
    std::thread::spawn(move || {
        let mut statuses = states.into_iter();
        for stream in listener.incoming().take(1 + statuses.len()) {
            let mut stream = stream.unwrap();
            let request = read_http_request(&mut stream);
            let first_line = request.lines().next().unwrap_or_default().to_string();
            if first_line.starts_with("POST /api/v1/publisher/upload") {
                recorded_thread.lock().unwrap().upload = Some(request);
                write!(
                    stream,
                    "HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: 13\r\nConnection: close\r\n\r\ndeployment-42"
                )
                .unwrap();
            } else if first_line.starts_with("POST /api/v1/publisher/status") {
                recorded_thread.lock().unwrap().status = Some(request);
                let state = statuses.next().unwrap_or("PUBLISHED");
                let body = format!(
                    r#"{{"deploymentId":"deployment-42","deploymentState":"{state}","purls":["pkg:maven/dev.telegraphic.demo/uploaded@1.0.0"]}}"#
                );
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            } else {
                recorded_thread.lock().unwrap().status = Some(request);
                write!(
                    stream,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
        }
    });
    (url, recorded)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    loop {
        let read = stream.read(&mut buf).unwrap();
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..read]);
        if let Some(header_end) = find_header_end(&bytes) {
            let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            let body_start = header_end + 4;
            while bytes.len().saturating_sub(body_start) < content_length {
                let read = stream.read(&mut buf).unwrap();
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..read]);
            }
            break;
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn fake_gpg_path(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let bin = tmp.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gpg = bin.join("gpg");
    fs::write(
        &gpg,
        r#"#!/usr/bin/env sh
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
    break
  fi
  prev="$arg"
done
printf 'fake-signature' > "$out"
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&gpg, fs::Permissions::from_mode(0o755)).unwrap();
    }
    bin
}

#[test]
fn publish_uploads_signed_bundle_to_maven_central_and_polls_status() {
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
        tmp.path().join("jbx.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo",
  "id": "uploaded",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo.uploaded",
  "description": "Uploaded tool",
  "url": "https://github.com/telegraphic-dev/uploaded",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic"}],
  "scm": {"connection": "scm:git:https://github.com/telegraphic-dev/uploaded.git", "url": "https://github.com/telegraphic-dev/uploaded"}
}
"#,
    )
    .unwrap();
    let (central_url, recorded) = start_mock_central(vec!["PUBLISHED"]);
    let fake_bin = fake_gpg_path(&tmp);
    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = juv_command()
        .arg("publish")
        .arg("--publish")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--central-url")
        .arg(central_url)
        .arg("--poll-interval")
        .arg("0")
        .arg("--max-wait-seconds")
        .arg("5")
        .env("PATH", path)
        .env("CENTRAL_TOKEN_USERNAME", "user")
        .env("CENTRAL_TOKEN_PASSWORD", "pass")
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("uploaded Maven Central deployment deployment-42"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Maven Central deployment deployment-42: PUBLISHED"),
        "{stdout}"
    );
    assert!(
        stdout.contains("pkg:maven/dev.telegraphic.demo/uploaded@1.0.0"),
        "{stdout}"
    );
    let recorded = recorded.lock().unwrap();
    let upload = recorded.upload.as_ref().unwrap();
    assert!(
        upload.starts_with(
            "POST /api/v1/publisher/upload?name=uploaded-1.0.0&publishingType=AUTOMATIC HTTP/1.1"
        ),
        "{upload}"
    );
    assert!(
        upload.contains("Authorization: Bearer dXNlcjpwYXNz"),
        "{upload}"
    );
    assert!(
        upload.contains("Content-Type: multipart/form-data; boundary="),
        "{upload}"
    );
    assert!(
        upload.contains("name=\"bundle\"; filename=\"uploaded-1.0.0-central-bundle.zip\""),
        "{upload}"
    );
    assert!(upload.contains("uploaded-1.0.0.jar.asc"), "{upload}");
    let status = recorded.status.as_ref().unwrap();
    assert!(
        status.starts_with("POST /api/v1/publisher/status?id=deployment-42 HTTP/1.1"),
        "{status}"
    );
}

#[test]
fn publish_requires_central_credentials_before_uploading() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("Hello.java"),
        "public class Hello { public static void main(String[] args) {} }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbx.json"),
        r#"{
  "main": "Hello.java",
  "group": "dev.telegraphic.demo", "id": "needs-creds", "version": "1.0.0",
  "package": "dev.telegraphic.demo.creds",
  "description": "Needs credentials",
  "url": "https://github.com/telegraphic-dev/needs-creds",
  "licenses": [{"name": "MIT License", "url": "https://opensource.org/licenses/MIT"}],
  "developers": [{"name": "Telegraphic"}],
  "scm": {"connection": "scm:git:https://github.com/telegraphic-dev/needs-creds.git", "url": "https://github.com/telegraphic-dev/needs-creds"}
}
"#,
    )
    .unwrap();
    let fake_bin = fake_gpg_path(&tmp);
    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = juv_command()
        .arg("publish")
        .arg("--publish")
        .arg("--file")
        .arg(tmp.path().join("jbx.json"))
        .arg("--target-dir")
        .arg(tmp.path().join("publish-target"))
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .env("PATH", path)
        .env_remove("CENTRAL_PORTAL_TOKEN")
        .env_remove("CENTRAL_TOKEN")
        .env_remove("MAVEN_CENTRAL_TOKEN")
        .env_remove("SONATYPE_TOKEN")
        .env_remove("CENTRAL_TOKEN_USERNAME")
        .env_remove("CENTRAL_TOKEN_PASSWORD")
        .env_remove("CENTRAL_PORTAL_USERNAME")
        .env_remove("CENTRAL_PORTAL_PASSWORD")
        .env_remove("CENTRAL_USERNAME")
        .env_remove("CENTRAL_PASSWORD")
        .env_remove("MAVEN_CENTRAL_USERNAME")
        .env_remove("MAVEN_CENTRAL_PASSWORD")
        .env_remove("SONATYPE_USERNAME")
        .env_remove("SONATYPE_PASSWORD")
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Maven Central publishing requires CENTRAL_PORTAL_TOKEN"),
        "{stderr}"
    );
}

#[test]
fn publish_rejects_dry_run_publish_combination_before_work() {
    let out = juv_command()
        .arg("publish")
        .arg("--dry-run")
        .arg("--publish")
        .arg("--file")
        .arg("does-not-matter.json")
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with")
            || stderr.contains("--dry-run and --publish are mutually exclusive"),
        "{stderr}"
    );
}
