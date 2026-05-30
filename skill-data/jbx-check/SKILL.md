---
name: jbx-check
description: Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.
---

# jbx-check

Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-check
```

## Use when

- Gate an agent edit before running tests.
- Collect exact diagnostics for a PR review comment or automated repair loop.
- Check a Java 25 compact script where a plain build tool would not understand the wrapper semantics.

## Quick commands

```sh
jbx check src --json
jbx check src/main/java --json
jbx check Hello.java --no-error-prone --json
jbx check src test --fail-on-warn --json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Gate an agent edit before running tests.
- Collect exact diagnostics for a PR review comment or automated repair loop.
- Check a Java 25 compact script where a plain build tool would not understand the wrapper semantics.

## Agent guidance

Use this as the first quality gate after edits. JSON diagnostics are the contract; make fixes from structured file/line/column fields, then rerun until the status is clean or only accepted warnings remain.

## Structured output

`--json` returns command status and diagnostics with file, line, column, severity, tool, code/message, and suggested next action. Website schema: `/docs/schemas/#check-json`.

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
