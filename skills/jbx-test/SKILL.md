---
name: jbx-test
description: Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.
---

# jbx-test

Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-test
```

## Use when

- Run a small Java kata or library test suite without creating a full build file.
- Give an agent failing test names and stack traces in a parseable shape.
- Collect coverage during a refactor to prove the edited code path is exercised.

## Quick commands

```sh
jbx test src/test/java --json
jbx test tests/CalculatorTest.java -- --select-method CalculatorTest#adds --json
jbx test --coverage --json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Run a small Java kata or library test suite without creating a full build file.
- Give an agent failing test names and stack traces in a parseable shape.
- Collect coverage during a refactor to prove the edited code path is exercised.

## Agent guidance

Start with focused tests when repairing a failure, then broaden to the directory or suite. Preserve non-zero exits for failed tests; do not hide failures behind “JSON parsed successfully”.

## Structured output

`--json` reports status, selected tests, failures, console XML paths, and optional coverage paths/counters. Website schema: `/docs/schemas/#test-json`.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-test` from the target machine.
