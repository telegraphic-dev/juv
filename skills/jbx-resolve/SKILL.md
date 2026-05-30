---
name: jbx-resolve
description: Resolve Maven coordinates to dependency coordinates without running code.
---

# jbx-resolve

Resolve Maven coordinates to dependency coordinates without running code.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-resolve
```

## Use when

- Inspect the dependency graph before committing a new `//DEPS`.
- Check whether exclusions or runtime scopes change the graph.
- Debug version mediation without executing user code.

## Quick commands

```sh
jbx resolve com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx resolve --scope runtime org.slf4j:slf4j-simple:2.0.17
jbx resolve --tree com.acme:app:1.0.0
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Inspect the dependency graph before committing a new `//DEPS`.
- Check whether exclusions or runtime scopes change the graph.
- Debug version mediation without executing user code.

## Agent guidance

Use `resolve` for metadata questions. It should not be treated as proof that jars are already present locally; use `fetch` for that.

## Structured output

No `--json` mode yet. Output is dependency coordinates or classpath-style text depending on flags. Use `fetch` when artifacts must be downloaded.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-resolve` from the target machine.
