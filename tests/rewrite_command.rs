use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::thread;

fn jbx_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jbx"))
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "expected success\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

const FAKE_REWRITE_COORDINATE: &str = "dev.telegraphic.jbx:jbx-rewrite:999-test";

fn install_fake_artifact(home: &Path, group: &str, artifact: &str, version: &str) {
    let repo = home
        .join(".m2/repository")
        .join(group.replace('.', "/"))
        .join(artifact)
        .join(version);
    fs::create_dir_all(&repo).unwrap();
    fs::write(
        repo.join(format!("{artifact}-{version}.pom")),
        format!(
            r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>{group}</groupId>
  <artifactId>{artifact}</artifactId>
  <version>{version}</version>
</project>
"#
        ),
    )
    .unwrap();
    let empty = home.join("empty-jar-dir");
    fs::create_dir_all(&empty).unwrap();
    let jar = Command::new("jar")
        .arg("cf")
        .arg(repo.join(format!("{artifact}-{version}.jar")))
        .arg("-C")
        .arg(&empty)
        .arg(".")
        .status()
        .unwrap();
    assert!(jar.success());
}

fn install_fake_rewrite_helper(home: &Path) {
    install_fake_artifact(home, "org.openrewrite", "rewrite-java", "8.56.1");
    install_fake_artifact(home, "org.openrewrite", "rewrite-java-21", "8.56.1");
    install_fake_artifact(home, "org.slf4j", "slf4j-api", "2.0.17");
    install_fake_artifact(home, "org.slf4j", "slf4j-nop", "2.0.17");
    let repo = home.join(".m2/repository/dev/telegraphic/jbx/jbx-rewrite/999-test");
    fs::create_dir_all(&repo).unwrap();
    fs::write(
        repo.join("jbx-rewrite-999-test.pom"),
        r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>dev.telegraphic.jbx</groupId>
  <artifactId>jbx-rewrite</artifactId>
  <version>999-test</version>
</project>
"#,
    )
    .unwrap();
    let source_dir = home.join("fake-helper-src/dev/telegraphic/jbx/rewrite");
    let classes = home.join("fake-helper-classes");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&classes).unwrap();
    let source = source_dir.join("JbxRewrite.java");
    fs::write(
        &source,
        r#"package dev.telegraphic.jbx.rewrite;

import java.nio.file.*;
import java.util.*;
import java.io.*;

public class JbxRewrite {
  public static void main(String[] args) throws Exception {
    List<String> a = Arrays.asList(args);
    if (a.contains("--discover")) {
      if (a.contains("--json")) {
        System.out.println("[{\"name\":\"org.openrewrite.java.format.AutoFormat\",\"short\":\"auto-format\",\"displayName\":\"AutoFormat\",\"description\":\"\",\"options\":[]}]");
      } else {
        System.out.println("SHORT        RECIPE");
        System.out.println("auto-format  org.openrewrite.java.format.AutoFormat");
      }
      return;
    }
    boolean apply = a.contains("--apply");
    String report = value(a, "--report", "rewrite");
    String oldPkg = option(a, "old", "oldPackageName", "com.old");
    String newPkg = option(a, "new", "newPackageName", "com.fresh");
    List<Path> files = new ArrayList<>();
    for (int i = 0; i < a.size(); i++) {
      if (a.get(i).equals("--source") && i + 1 < a.size()) collect(Path.of(a.get(++i)), files);
    }
    int changes = 0;
    StringBuilder patch = new StringBuilder();
    for (Path file : files) {
      String text = Files.readString(file);
      String updated = text;
      Path target = file;
      if (a.toString().contains("ChangePackage") || a.toString().contains("change-package")) {
        updated = text.replace(oldPkg, newPkg);
        target = Path.of(file.toString().replace(oldPkg.replace('.', File.separatorChar), newPkg.replace('.', File.separatorChar)));
      } else {
        updated = text.replace("class Example{void main(){System.out.println(\"hi\");}}", "class Example {\n    void main() {\n        System.out.println(\"hi\");\n    }\n}");
      }
      if (!updated.equals(text) || !target.equals(file)) {
        changes++;
        patch.append("--- a/").append(file).append("\n+++ b/").append(target).append("\n");
        patch.append(" class Example {\n+    void main() {\n");
        if (apply) {
          Files.createDirectories(target.getParent() == null ? Path.of(".") : target.getParent());
          Files.writeString(target, updated);
          if (!target.equals(file)) Files.deleteIfExists(file);
        }
      }
    }
    Files.createDirectories(Path.of(report));
    Files.writeString(Path.of(report).resolve("rewrite.patch"), patch.toString());
    System.out.println("Rewrite results: " + changes + " change(s)");
    System.out.println("Patch: " + Path.of(report).resolve("rewrite.patch"));
  }

  static String value(List<String> args, String flag, String fallback) {
    for (int i = 0; i + 1 < args.size(); i++) if (args.get(i).equals(flag)) return args.get(i + 1);
    return fallback;
  }

  static String option(List<String> args, String shortName, String longName, String fallback) {
    for (int i = 0; i + 1 < args.size(); i++) if (args.get(i).equals("--option")) {
      String value = args.get(i + 1);
      if (value.startsWith(shortName + "=") || value.startsWith(longName + "=")) return value.substring(value.indexOf('=') + 1);
    }
    return fallback;
  }

  static void collect(Path path, List<Path> files) throws IOException {
    if (Files.isDirectory(path)) {
      try (var stream = Files.walk(path)) { stream.filter(p -> p.toString().endsWith(".java")).forEach(files::add); }
    } else files.add(path);
  }
}
"#,
    )
    .unwrap();
    let javac = Command::new("javac")
        .arg("--release")
        .arg("21")
        .arg("-d")
        .arg(&classes)
        .arg(&source)
        .status()
        .unwrap();
    assert!(javac.success());
    let jar = Command::new("jar")
        .arg("cf")
        .arg(repo.join("jbx-rewrite-999-test.jar"))
        .arg("-C")
        .arg(&classes)
        .arg(".")
        .status()
        .unwrap();
    assert!(jar.success());
}

fn configure_fake_rewrite_helper(command: &mut Command, home: &Path) {
    install_fake_rewrite_helper(home);
    command
        .env("HOME", home)
        .env("JBX_REWRITE_HELPER_COORDINATE", FAKE_REWRITE_COORDINATE);
}

fn serve_rewrite_modules_response() -> String {
    serve_maven_search_response(
        r#"{
  "response": {
    "numFound": 1,
    "docs": [
      {"id":"org.openrewrite:rewrite-maven","g":"org.openrewrite","a":"rewrite-maven","latestVersion":"8.56.1","p":"jar","versionCount": 120}
    ]
  }
}"#,
    )
}

fn serve_rewrite_recipe_modules_response() -> String {
    serve_maven_search_response(
        r#"{
  "response": {
    "numFound": 1,
    "docs": [
      {"id":"org.openrewrite.recipe:rewrite-spring","g":"org.openrewrite.recipe","a":"rewrite-spring","latestVersion":"6.4.0","p":"jar","versionCount": 82}
    ]
  }
}"#,
    )
}

fn serve_maven_search_response(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap_or(0);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    base
}

#[test]
fn rewrite_patch_writes_patch_without_changing_source() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let source = tmp.path().join("Example.java");
    let original = "class Example{void main(){System.out.println(\"hi\");}}\n";
    fs::write(&source, original).unwrap();

    let mut command = jbx_command();
    configure_fake_rewrite_helper(&mut command, &home);
    let out = command
        .arg("rewrite")
        .arg("patch")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--recipe")
        .arg("auto-format")
        .arg("--source")
        .arg(&source)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(fs::read_to_string(&source).unwrap(), original);
    let patch = fs::read_to_string(tmp.path().join("rewrite/rewrite.patch")).unwrap();
    assert!(patch.contains("class Example"), "{patch}");
    assert!(patch.contains("+    void main()"), "{patch}");
}

#[test]
fn rewrite_apply_updates_source_and_accepts_module_shorthand() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let source = tmp.path().join("Example.java");
    fs::write(
        &source,
        "class Example{void main(){System.out.println(\"hi\");}}\n",
    )
    .unwrap();

    let mut command = jbx_command();
    configure_fake_rewrite_helper(&mut command, &home);
    let out = command
        .arg("rewrite")
        .arg("apply")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--module")
        .arg("java")
        .arg("--recipe")
        .arg("auto-format")
        .arg("--source")
        .arg(&source)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let updated = fs::read_to_string(&source).unwrap();
    assert!(updated.contains("class Example {"), "{updated}");
    assert!(updated.contains("    void main()"), "{updated}");
}

#[test]
fn rewrite_apply_passes_recipe_options_for_parameterized_recipes() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let source_dir = tmp.path().join("src/main/java/com/old");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("Example.java");
    fs::write(
        &source,
        "package com.old;\n\nclass Example {\n    com.old.Other other;\n}\n",
    )
    .unwrap();

    let mut command = jbx_command();
    configure_fake_rewrite_helper(&mut command, &home);
    let out = command
        .arg("rewrite")
        .arg("apply")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--recipe")
        .arg("change-package")
        .arg("--option")
        .arg("old=com.old")
        .arg("--option")
        .arg("new=com.fresh")
        .arg("--source")
        .arg(tmp.path().join("src/main/java"))
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_success(&out);
    let rewritten = tmp.path().join("src/main/java/com/fresh/Example.java");
    let updated = fs::read_to_string(&rewritten).unwrap();
    assert!(updated.contains("package com.fresh;"), "{updated}");
    assert!(updated.contains("com.fresh.Other"), "{updated}");
}

#[test]
fn rewrite_modules_lists_and_searches_standard_modules() {
    let out = jbx_command()
        .arg("rewrite")
        .arg("modules")
        .arg("--search")
        .arg("maven")
        .arg("--group")
        .arg("org.openrewrite")
        .arg("--limit")
        .arg("1")
        .arg("--rewrite-version")
        .arg("8.60.0")
        .arg("--json")
        .env("JBX_MAVEN_SEARCH_URL", serve_rewrite_modules_response())
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"short\": \"maven\""), "{stdout}");
    assert!(
        stdout.contains("org.openrewrite:rewrite-maven:8.60.0"),
        "{stdout}"
    );
}

#[test]
fn rewrite_modules_uses_recipe_module_latest_version_by_default() {
    let out = jbx_command()
        .arg("rewrite")
        .arg("modules")
        .arg("--search")
        .arg("spring")
        .arg("--group")
        .arg("org.openrewrite.recipe")
        .arg("--json")
        .env(
            "JBX_MAVEN_SEARCH_URL",
            serve_rewrite_recipe_modules_response(),
        )
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("org.openrewrite.recipe:rewrite-spring:6.4.0"),
        "{stdout}"
    );
    assert!(!stdout.contains(":8.56.1"), "{stdout}");
}

#[test]
fn rewrite_recipes_lists_recipe_short_names() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let mut command = jbx_command();
    configure_fake_rewrite_helper(&mut command, &home);
    let out = command
        .arg("rewrite")
        .arg("recipes")
        .arg("java")
        .arg("--cache-dir")
        .arg(tmp.path().join("cache"))
        .arg("--search")
        .arg("auto-format")
        .arg("--limit")
        .arg("1")
        .arg("--json")
        .output()
        .unwrap();

    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"short\":\"auto-format\""), "{stdout}");
    assert!(
        stdout.contains("org.openrewrite.java.format.AutoFormat"),
        "{stdout}"
    );
}
