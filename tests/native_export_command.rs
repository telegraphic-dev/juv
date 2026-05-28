use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
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

#[cfg(unix)]
fn write_fake_native_image(path: &Path, log: &Path) {
    fs::write(
        path,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$@" > '{}'
out_dir='.'
out_name=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -H:Path=*) out_dir="${{1#-H:Path=}}" ;;
    -H:Name=*) out_name="${{1#-H:Name=}}" ;;
  esac
  shift
done
[ -n "$out_name" ] || out_name='native-image'
mkdir -p "$out_dir"
printf 'native-binary\n' > "$out_dir/$out_name"
"#,
            log.display()
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[test]
#[cfg(unix)]
fn export_native_invokes_native_image_with_classpath_main_and_native_options() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let native_image = tmp.path().join("native-image");
    let log = tmp.path().join("native-image.args");
    let output = tmp.path().join("hello-native");
    write_fake_native_image(&native_image, &log);

    let src = tmp.path().join("HelloNative.java");
    fs::write(
        &src,
        r#"//NATIVE_OPTIONS --no-fallback
class HelloNative {
  public static void main(String[] args) {
    System.out.println("hello native");
  }
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("export")
        .arg("native")
        .arg("--cache-dir")
        .arg(&cache)
        .arg("--native-image")
        .arg(&native_image)
        .arg("--native-option")
        .arg("--initialize-at-build-time=HelloNative")
        .arg("--output")
        .arg(&output)
        .arg(&src)
        .output()
        .unwrap();

    assert_success(&out);
    assert_eq!(fs::read_to_string(&output).unwrap(), "native-binary\n");
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("--no-fallback\n"), "{args}");
    assert!(
        args.contains("--initialize-at-build-time=HelloNative\n"),
        "{args}"
    );
    assert!(args.contains("-cp\n"), "{args}");
    assert!(args.contains("HelloNative\n"), "{args}");
    assert!(
        args.contains(&format!(
            "-H:Name={}\n",
            output.file_name().unwrap().to_string_lossy()
        )),
        "{args}"
    );
    assert!(
        args.contains(&format!("-H:Path={}\n", output.parent().unwrap().display())),
        "{args}"
    );
}

#[test]
#[cfg(unix)]
fn export_native_refuses_to_overwrite_without_force() {
    let tmp = tempfile::tempdir().unwrap();
    let native_image = tmp.path().join("native-image");
    let log = tmp.path().join("native-image.args");
    let output = tmp.path().join("existing-bin");
    write_fake_native_image(&native_image, &log);
    fs::write(&output, "existing\n").unwrap();
    let src = tmp.path().join("ExistingNative.java");
    fs::write(
        &src,
        r#"class ExistingNative {
  public static void main(String[] args) {}
}
"#,
    )
    .unwrap();

    let out = juv_command()
        .arg("export")
        .arg("native")
        .arg("--native-image")
        .arg(&native_image)
        .arg("--output")
        .arg(&output)
        .arg(&src)
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("use --force to overwrite"), "{stderr}");
    assert_eq!(fs::read_to_string(&output).unwrap(), "existing\n");
    assert!(
        !log.exists(),
        "native-image must not run on overwrite refusal"
    );
}
