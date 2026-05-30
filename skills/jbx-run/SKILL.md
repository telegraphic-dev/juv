---
name: jbx-run
description: Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.
---

# `run`

Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.

## When to use it

- Run a self-contained Java script without creating a Maven or Gradle project.
- Launch a compact script that carries `//DEPS`, `//JAVA`, `//SOURCES`, and runtime options in the file.
- Smoke-test an executable example after `jbx check --json` has confirmed the source compiles.

## Common workflows

```bash
jbx run report.java --month 2026-05
jbx com.example:report-cli --month 2026-05
```

## Passing arguments

`run` options go before the script path. After the script path, arguments belong to the Java program, including Picocli-style options such as `--help`, `--input`, or `--verbose`.

Use an explicit `--` only when the Java program needs to receive a literal double-dash argument.

## Real-life examples

### Start NanoCode Agent

```bash
gh repo clone glaforge/nanocode
cd nanocode
export GOOGLE_AI_GEMINI_API_KEY="***"
jbx nanocode_basic.java
```

### Generate OpenAPI from the specification file

```bash
jbx org.openapitools:openapi-generator-cli generate \
  -i openapi.yaml \
  -g java \
  -o generated/petstore-client \
  --api-package com.example.petstore.api \
  --model-package com.example.petstore.model \
  --invoker-package com.example.petstore.client
```

### Start H2 In-Memory Database

```bash
jbx com.h2database:h2
```

### Agent loop

1. Inspect the target with `jbx info directives <script>` or `jbx search <artifact> --json`.
2. Preflight source with `jbx check --json` or `jbx build` when execution has side effects.
3. Run with program arguments after the script/artifact target, without adding `--` unless the program needs a literal double dash.
3. Verify outputs, generated files, or service readiness directly.

## Agent notes

`run` executes arbitrary user code. Prefer `info`, `build`, `check --json`, or `doctor --json` first when the command source, dependencies, or environment are not understood.

## Verification checklist

- The target script or artifact coordinate is the intended one.
- Source targets compile before execution when safety matters.
- Program arguments are passed after the target and are visible to the Java program.
- Generated files, network listeners, or exit codes are checked with a second command.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx run`

```text
Compile and run a Java source file

Usage: jbx run [OPTIONS] <SCRIPT> [ARGS]...

Arguments:
  <SCRIPT>   Java source file
  [ARGS]...  Arguments passed to the script

Options:
      --deps <DEPS>                       Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>                      Additional repository, same shape as //REPOS
      --source <SOURCES>                  Additional source file, same shape as //SOURCES
      --files <FILES>                     Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>            Additional classpath entries
      --javac-option <JAVAC_OPTIONS>      Additional javac option
      --runtime-option <RUNTIME_OPTIONS>  Additional java runtime option
      --java <JAVA_VERSION>               Override //JAVA requested version
      --javaagent <JAVA_AGENTS>           Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>                 Override //MAIN / inferred class name
      --cache-dir <CACHE_DIR>             Override cache directory
      --trust                             Trust this remote script content hash before running
  -h, --help                              Print help
```

> For exact behavior, prefer the skill bundled with the `jbx` binary on the machine running the task.
