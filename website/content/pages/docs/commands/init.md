---

title: jbx init command
description: Create Java 25+ scripts from built-in or imported templates.
---

# `init`

Create Java sources from built-in or imported templates.

## When to use it

- Start a new compact Java script with the expected shebang/directive style.
- Create a small CLI skeleton with Picocli or team defaults.
- Generate a test file that can immediately run through `jbx test`.

## Common workflows

```bash
jbx init Hello.java
jbx init --template cli tools/hello.java
jbx init -t test hello_test.java
```

## Real-life examples

### Start a testable compact Java script

```bash
jbx template list --json
jbx init -t test hello_test.java
jbx test hello_test.java --json
```

Use `init` to create a file that already matches jbx conventions instead of hand-writing shebangs, directives, and boilerplate.

### Agent loop

1. Discover templates with `jbx template list --json`.
2. Choose the smallest template that matches the requested artifact.
3. Generate into the intended directory with `--force` only when replacing is deliberate.
4. Run the natural next gate: `jbx check --json`, `jbx test --json`, or `jbx run`.

## Agent notes

Generated files are starting points. Rename packages/classes and descriptions immediately so template names do not leak into user code.

## Verification checklist

- Generated file exists at the requested path.
- `git diff` shows only intended new or replaced files.
- The generated source passes the relevant jbx gate before further editing.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx init`

```text
Initialize a Java script

Usage: jbx init [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file to initialize

Options:
  -t, --template <TEMPLATE>  Init script with the default Java template for now
      --force                Force overwrite of existing files
      --java <JAVA_VERSION>  Java version directive to write
      --deps <DEPS>          Add dependencies, separated by comma, semicolon, or whitespace
  -h, --help                 Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-init
```
