---
name: jbx-fmt
description: Format Java files with Palantir Java Format.
---

# `fmt`

Format Java files with Palantir Java Format, including Java 25 compact scripts.

## When to use it

- Normalize generated Java before committing.
- Format compact scripts that have leading directives/comments.
- Run after OpenRewrite or AST import to keep diffs readable.

## Common workflows

```bash
jbx fmt src/main/java
jbx fmt Hello.java
jbx fmt --check src test
```

## Real-life examples

### Format generated Java before committing

```bash
jbx fmt generated/Report.java
jbx fmt --check src test
```

Use `fmt` after code generation, OpenRewrite, or AST import so review diffs are about behavior, not whitespace.

### Agent loop

1. Run `jbx fmt --check <paths>` to see whether formatting is needed.
2. Run `jbx fmt <paths>` only on files in scope.
3. Inspect `git diff` for unintended semantic movement.
4. Re-run `jbx fmt --check <paths>` before committing.

## Agent notes

Formatting can touch many files. Keep paths narrow during repair and avoid formatting unrelated source trees in the same PR.

## Verification checklist

- `jbx fmt --check <paths>` exits zero after formatting.
- Diff contains formatting-only changes for intended files.
- Follow-up `jbx check --json` still passes for edited Java.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx fmt`

```text
Format Java source files with Palantir Java Format

Usage: jbx fmt [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...
          Java source files or directories. Defaults to the current directory

          [default: .]

Options:
      --check
          Check formatting without rewriting files

      --formatter-version <FORMATTER_VERSION>
          Palantir Java Format version to use.

          Defaults to the cached latest Maven Central release, refreshed periodically.

      --cache-dir <CACHE_DIR>
          Override cache directory

  -h, --help
          Print help (see a summary with '-h')
```

> For exact behavior, prefer the skill bundled with the `jbx` binary on the machine running the task.
