---
name: jbx-graph
description: Dump JavaParser native AST JSON or import it back to Java source.
---

# jbx-graph

Dump JavaParser native AST JSON or import it back to Java source.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-graph
```

## Use when

- Let an agent perform structural analysis without regexing Java.
- Round-trip a generated AST back to source after a controlled transformation.
- Debug parser behavior around compact Java scripts.

## Quick commands

```sh
jbx graph dump Hello.java > Hello.ast.json
jbx graph import Hello.ast.json --output Hello.java
jbx graph dump src/main/java/com/acme/App.java | jq .
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Let an agent perform structural analysis without regexing Java.
- Round-trip a generated AST back to source after a controlled transformation.
- Debug parser behavior around compact Java scripts.

## Agent guidance

Prefer AST operations for structural changes, but verify with `jbx check --json` after import. Treat generated source as code changes requiring review.

## Structured output

`dump` emits JavaParser native AST JSON. `import` consumes that JSON and writes Java source. This is an AST interchange format, not a stable semantic schema for public APIs.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-graph` from the target machine.
