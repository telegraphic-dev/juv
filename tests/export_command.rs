use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
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

fn assert_failure(out: &Output) {
    assert!(
        !out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn java25_bin() -> PathBuf {
    let out = juv_command()
        .arg("jdk")
        .arg("home")
        .arg("25")
        .output()
        .unwrap();
    assert_success(&out);
    PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()).join("bin/java")
}

fn run_java_jar(jar: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(java25_bin());
    cmd.arg("-jar").arg(jar).args(args).output().unwrap()
}

fn compile_helper_jar(root: &Path) -> PathBuf {
    let helper_src = root.join("Helper.java");
    let classes = root.join("helper-classes");
    let helper_jar = root.join("helper.jar");
    fs::create_dir_all(&classes).unwrap();
    fs::write(
        &helper_src,
        r#"
public class Helper {
  public static String message() { return "portable-helper"; }
}
"#,
    )
    .unwrap();
    let javac = Command::new("javac")
        .arg("-d")
        .arg(&classes)
        .arg(&helper_src)
        .output()
        .unwrap();
    assert_success(&javac);
    let jar = Command::new("jar")
        .arg("--create")
        .arg("--file")
        .arg(&helper_jar)
        .arg("-C")
        .arg(&classes)
        .arg(".")
        .output()
        .unwrap();
    assert_success(&jar);
    helper_jar
}

fn read_manifest(jar: &Path) -> String {
    let file = fs::File::open(jar).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut manifest = String::new();
    zip.by_name("META-INF/MANIFEST.MF")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    manifest
}

#[test]
fn export_local_creates_runnable_jar() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = tmp.path().join("Hello.java");
    let out_jar = tmp.path().join("hello-app.jar");
    let spaced_deps = tmp.path().join("deps with spaces");
    fs::create_dir_all(&spaced_deps).unwrap();
    let helper_jar = compile_helper_jar(&spaced_deps);
    fs::write(
        &src,
        r#"
class Hello {
  public static void main(String[] args) {
    System.out.println("hello " + args[0] + " " + Helper.message());
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("export")
        .arg("local")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--class-path")
        .arg(&helper_jar)
        .arg("--output")
        .arg(&out_jar)
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    assert!(out_jar.exists());
    let manifest = read_manifest(&out_jar);
    assert!(manifest.contains("file://"));
    assert!(manifest.contains("deps%20with%20spaces"));
    assert!(manifest.lines().all(|line| line.len() <= 72));
    let run = run_java_jar(&out_jar, &["jar"]);
    assert_success(&run);
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "hello jar portable-helper"
    );
}

#[test]
fn export_refuses_to_overwrite_without_force() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("Overwrite.java");
    let out_jar = tmp.path().join("overwrite.jar");
    fs::write(
        &src,
        r#"
class Overwrite {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();
    fs::write(&out_jar, "already here").unwrap();

    let out = juv_command()
        .arg("export")
        .arg("local")
        .arg("--output")
        .arg(&out_jar)
        .arg(&src)
        .output()
        .unwrap();

    assert_failure(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--force"));
}

#[test]
fn export_portable_copies_classpath_entries_to_lib() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let helper_jar = compile_helper_jar(tmp.path());
    let spaced_helper_jar = tmp.path().join("helper lib.jar");
    fs::copy(&helper_jar, &spaced_helper_jar).unwrap();
    let src = tmp.path().join("UseHelper.java");
    let out_jar = tmp.path().join("dist").join("use-helper.jar");
    fs::write(
        &src,
        r#"
class UseHelper {
  public static void main(String[] args) {
    System.out.println(Helper.message());
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("export")
        .arg("portable")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--class-path")
        .arg(&spaced_helper_jar)
        .arg("--output")
        .arg(&out_jar)
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    assert!(out_jar.exists());
    assert!(out_jar
        .parent()
        .unwrap()
        .join("lib/helper lib.jar")
        .exists());
    let manifest = read_manifest(&out_jar);
    assert!(manifest.contains("lib/helper%20lib.jar"));
    assert!(manifest.lines().all(|line| line.len() <= 72));
    let run = run_java_jar(&out_jar, &[]);
    assert_success(&run);
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "portable-helper"
    );
}

#[test]
fn export_portable_rejects_duplicate_dependency_filenames_even_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let first_dir = tmp.path().join("first");
    let second_dir = tmp.path().join("second");
    fs::create_dir_all(&first_dir).unwrap();
    fs::create_dir_all(&second_dir).unwrap();
    let first = compile_helper_jar(&first_dir);
    let second = compile_helper_jar(&second_dir);
    let src = tmp.path().join("DuplicateDeps.java");
    let out_jar = tmp.path().join("duplicate.jar");
    fs::write(
        &src,
        r#"
class DuplicateDeps {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("export")
        .arg("portable")
        .arg("--force")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--class-path")
        .arg(&first)
        .arg("--class-path")
        .arg(&second)
        .arg("--output")
        .arg(&out_jar)
        .arg(&src)
        .output()
        .unwrap();

    assert_failure(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("duplicate dependency filename"));
}

#[test]
fn export_supports_catalog_aliases() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let src = tmp.path().join("AliasedExport.java");
    let out_jar = tmp.path().join("aliased.jar");
    fs::write(
        &src,
        r#"
class AliasedExport {
  public static void main(String[] args) {
    System.out.println("alias export");
  }
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("jbang-catalog.json"),
        r#"{
  "aliases": {
    "shipit": {
      "script-ref": "AliasedExport.java"
    }
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .current_dir(tmp.path())
        .arg("export")
        .arg("local")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--output")
        .arg(&out_jar)
        .arg("shipit")
        .output()
        .unwrap();

    assert_success(&out);
    let run = run_java_jar(&out_jar, &[]);
    assert_success(&run);
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "alias export");
}
