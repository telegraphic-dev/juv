---
name: jbx-export
description: Export local, portable, or native runnable artifacts.
---

# `export`

Export local, portable, or native runnable artifacts.

## When to use it

- Package a script for a machine that should not re-resolve dependencies at runtime.
- Create a portable directory with jars and launch metadata for CI artifacts.
- Build a native executable when GraalVM/native-image is available.

## Common workflows

```bash
jbx export local Hello.java --output build/hello.jar
jbx export portable Hello.java --output dist/hello
jbx export native Hello.java --output dist/hello
```

## Real-life examples

### Package a script for a CI artifact

```bash
jbx check Hello.java --json
jbx export portable Hello.java --output dist/hello
```

Use `export` after compilation is already clean. It turns a script into a local jar, portable directory, or native executable for machines that should not rediscover everything at runtime.

### Agent loop

1. Preflight with `jbx check <file> --json` or `jbx build <file>`.
2. Choose `local`, `portable`, or `native` based on the deployment target.
3. Write output under `build/` or `dist/`, not beside source files.
4. Inspect the generated artifact and run the safest smoke command.

## Agent notes

Native export depends on GraalVM/native-image and is slower. Do not choose it when a jar or portable directory satisfies the task.

## Verification checklist

- Output path exists and contains the expected jar/directory/binary.
- The exported artifact runs a harmless `--help` or equivalent smoke path.
- Native export is preceded by `jbx doctor --json` when native-image availability is uncertain.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx export`

```text
Export runnable JARs

Usage: jbx export <COMMAND>

Commands:
  local     Export a runnable JAR with manifest classpath entries pointing at local paths
  portable  Export a runnable JAR plus lib/ dependencies for portable use
  native    Export a native executable using GraalVM native-image
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx export local`

```text
Export a runnable JAR with manifest classpath entries pointing at local paths

Usage: jbx export local [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file or catalog alias to export

Options:
  -o, --output <OUTPUT>
          Output JAR path (defaults to <script>.jar)
      --force
          Force overwrite of existing output files
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --source <SOURCES>
          Additional source file, same shape as //SOURCES
      --files <FILES>
          Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option, same shape as //JAVA_OPTIONS
      --java <JAVA_VERSION>
          Override //JAVA requested version
      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>
          Override //MAIN / inferred class name
      --cache-dir <CACHE_DIR>
          Override cache directory
      --trust
          Trust this remote script content hash before exporting
  -h, --help
          Print help
```

### `jbx export portable`

```text
Export a runnable JAR plus lib/ dependencies for portable use

Usage: jbx export portable [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file or catalog alias to export

Options:
  -o, --output <OUTPUT>
          Output JAR path (defaults to <script>.jar)
      --force
          Force overwrite of existing output files
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --source <SOURCES>
          Additional source file, same shape as //SOURCES
      --files <FILES>
          Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option, same shape as //JAVA_OPTIONS
      --java <JAVA_VERSION>
          Override //JAVA requested version
      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>
          Override //MAIN / inferred class name
      --cache-dir <CACHE_DIR>
          Override cache directory
      --trust
          Trust this remote script content hash before exporting
  -h, --help
          Print help
```

### `jbx export native`

```text
Export a native executable using GraalVM native-image

Usage: jbx export native [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file or catalog alias to export

Options:
  -o, --output <OUTPUT>
          Output executable path (defaults to <script> with platform executable suffix)
      --force
          Force overwrite of existing output files
      --native-image <NATIVE_IMAGE>
          Path to native-image executable (defaults to JDK bin/native-image or PATH)
      --native-option <NATIVE_OPTIONS>
          Additional native-image option, same shape as //NATIVE_OPTIONS
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --source <SOURCES>
          Additional source file, same shape as //SOURCES
      --files <FILES>
          Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option, same shape as //JAVA_OPTIONS
      --java <JAVA_VERSION>
          Override //JAVA requested version
      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>
          Override //MAIN / inferred class name
      --cache-dir <CACHE_DIR>
          Override cache directory
      --trust
          Trust this remote script content hash before exporting
  -h, --help
          Print help
```

> For exact behavior, prefer the skill bundled with the `jbx` binary on the machine running the task.
