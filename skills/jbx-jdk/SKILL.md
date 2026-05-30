---
name: jbx-jdk
description: List, install, and locate JDKs used by jbx.
---

# jbx-jdk

List, install, and locate JDKs used by jbx.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-jdk
```

## Use when

- Verify which JDK will run Java 25 compact scripts.
- Install a missing Temurin JDK for a CI runner.
- Debug why JAVA_HOME differs from the JDK selected by jbx.

## Quick commands

```sh
jbx jdk list
jbx jdk home 25
jbx jdk install 25
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Verify which JDK will run Java 25 compact scripts.
- Install a missing Temurin JDK for a CI runner.
- Debug why JAVA_HOME differs from the JDK selected by jbx.

## Agent guidance

JDK installation downloads external binaries and changes local state. Prefer `doctor --json` and `jdk list` first; install only when needed and requested.

## Structured output

No `--json` mode yet. Use `jbx doctor --json` for structured environment checks until JDK subcommands grow dedicated JSON.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-jdk` from the target machine.
