---
name: jbx-build
description: Compile a script into the jbx cache without running it.
---

# `build`

Compile a script into the jbx cache without running it.

## When to use it

- Precompile a script during CI so the later run path starts from a warm cache.
- Validate generated Java without executing side effects.
- Check that dependency directives and CLI dependency overrides resolve together.

## Common workflows

```bash
jbx build scripts/Report.java
jbx build --deps org.slf4j:slf4j-api:2.0.17 tools/Probe.java
jbx build --java 25 Hello.java
```

## Real-life examples

### Preflight a generated script before running it

```bash
jbx build generated/Report.java
jbx check generated/Report.java --json
```

Use `build` when execution would be unsafe or noisy but compilation must be proven: generated code, release helpers, migration scripts, or examples copied into docs.

### Agent loop

1. Read dependencies and Java version with `jbx info directives <file>`.
2. Compile the smallest target with `jbx build <file>`.
3. If it fails, fix compile errors before trying `jbx run`.
4. For warning detail, follow with `jbx check <file> --json`.

## Agent notes

`build` proves resolution and compilation, not behavior. It is the safe gate before `run` for scripts with network, file, or deployment side effects.

## Verification checklist

- Exit code is zero for the exact source file or directory being gated.
- Dependency resolution output does not point at unexpected repositories.
- The next command (`run`, `test`, or `export`) uses the same file and Java version assumptions.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx build`

```text
Compile and store script in the cache without running it

Usage: jbx build [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
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
          Trust this remote script content hash before building
  -h, --help
          Print help
```

> For exact behavior, prefer the skill bundled with the `jbx` binary on the machine running the task.
