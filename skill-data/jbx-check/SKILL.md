---
name: jbx-check
description: Check Java source with structured diagnostics.
---

# `check`

Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.

## When to use it

- Gate an agent edit before running tests.
- Collect exact diagnostics for a PR review comment or automated repair loop.
- Check a Java 25 compact script where a plain build tool would not understand the wrapper semantics.

## Common workflows

```bash
jbx check src --json
jbx check src/main/java --json
jbx check Hello.java --no-error-prone --json
jbx check src test --warnings-as-errors --json
```

## Real-life examples

### Gate an agent edit before tests

```bash
jbx check src/main/java --json
jbx check src test --warnings-as-errors --json
```

Use `check` immediately after editing Java. It catches compiler, lint, and Error Prone diagnostics before a slower test run burns time.

### Agent loop

1. Run `jbx check <changed paths> --json`.
2. Parse diagnostics by file, line, severity, and message.
3. Patch the smallest source region that explains the diagnostic.
4. Re-run the same check until JSON reports no blocking diagnostics.

## Agent notes

Keep the checked path narrow while repairing, then widen to `src test` before reporting. Do not scrape human diagnostics when `--json` is available.

## JSON and schema

`jbx check --json` returns structured diagnostics and command status. Example output and field notes: [`check JSON`](/docs/schemas/#check-json).

Example JSON:

```json
{
  "ok": true,
  "diagnostics": [],
  "compilerOutput": ""
}
```

## Verification checklist

- JSON parses and includes the expected checked files.
- No `error` diagnostics remain before running executable code.
- `--warnings-as-errors` is used when the repository treats warnings as CI failures.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx check`

```text
Check Java source files with javac diagnostics and Error Prone by default

Usage: jbx check [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Java source files or directories. Defaults to the current directory [default: .]

Options:
      --json
          Emit structured diagnostics JSON
      --no-error-prone
          Disable Error Prone checks and run only javac/-Xlint diagnostics
      --error-prone-version <ERROR_PRONE_VERSION>
          Error Prone version to use when Error Prone is enabled [default: 2.39.0]
      --warnings-as-errors
          Treat javac and Error Prone warnings as errors
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --java <JAVA_VERSION>
          Override requested Java version
      --cache-dir <CACHE_DIR>
          Override cache directory
  -h, --help
          Print help
```
