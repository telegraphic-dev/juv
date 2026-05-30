---
name: jbx-fetch
description: Download artifacts and print classpath or dependency coordinates.
---

# jbx-fetch

Download artifacts and print classpath or dependency coordinates.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-fetch
```

## Use when

- Materialize jars before an offline CI step.
- Build a classpath for an external Java command.
- Verify that dependency artifacts are reachable from configured repositories.

## Quick commands

```sh
jbx fetch com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx fetch --classpath org.junit.platform:junit-platform-console-standalone:1.11.4
jbx fetch --deps-only com.acme:app:1.0.0
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Materialize jars before an offline CI step.
- Build a classpath for an external Java command.
- Verify that dependency artifacts are reachable from configured repositories.

## Agent guidance

Use `fetch` when file availability matters. Keep cache paths out of committed files and logs unless they are intentionally diagnostic.

## Structured output

No `--json` mode yet. Output is meant for shell composition: classpath strings, paths, or dependency lists.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-fetch` from the target machine.
