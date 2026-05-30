---
name: jbx-run
description: Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.
---

# jbx-run

Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-run
```

## Use when

- Run a one-file maintenance script from a repository without creating a Maven or Gradle project.
- Launch a Java 25 compact script that carries `//DEPS`, `//JAVA`, `//SOURCES`, and runtime options in the file.
- Smoke-test an executable example after `jbx check --json` has confirmed the source compiles.

## Quick commands

```sh
jbx run scripts/Report.java -- --month 2026-05
jbx scripts/Report.java --month 2026-05
jbx run --deps info.picocli:picocli:4.7.7 tools/Cli.java -- --help
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Run a one-file maintenance script from a repository without creating a Maven or Gradle project.
- Launch a Java 25 compact script that carries `//DEPS`, `//JAVA`, `//SOURCES`, and runtime options in the file.
- Smoke-test an executable example after `jbx check --json` has confirmed the source compiles.

## Agent guidance

Treat `run` as the boundary where arbitrary user code executes. For autonomous loops, first inspect with `info`, compile with `build`, or validate with `check --json`; only run after the command and arguments are understood.

## Structured output

No `--json` mode: stdout/stderr belong to the program being run. Use `jbx check --json`, `jbx build`, `jbx info ...`, or `jbx doctor --json` for machine-readable preflight facts before execution.

## Common mistakes

- Do not infer command semantics from old web snippets; this skill reflects the installed release.
- Do not scrape human output when a JSON mode exists.
- Do not widen scope from a single file to the whole repository until the focused command is clean.
- Do not hide non-zero exits behind a successful parser or wrapper script.

## Verification

- Parse JSON output where available and validate required fields.
- For file changes, inspect `git diff --stat` and the exact changed files.
- For generated artifacts, test that the expected output path exists and is usable.
- For environment failures, run `jbx doctor --json` and report the failed checks with remediation.
