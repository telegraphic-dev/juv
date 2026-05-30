---
name: jbx-doctor
description: Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.
---

# jbx-doctor

Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-doctor
```

## Use when

- Diagnose why a script works on one machine but not in CI.
- Check whether a workstation has Java 25, Maven reachability, writable caches, and optional native-image support.
- Before filing a bug report, capture environment facts without exposing secrets.

## Quick commands

```sh
jbx doctor --json
jbx doctor Hello.java --json
jbx doctor https://example.com/tool.java --json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Diagnose why a script works on one machine but not in CI.
- Check whether a workstation has Java 25, Maven reachability, writable caches, and optional native-image support.
- Before filing a bug report, capture environment facts without exposing secrets.

## Agent guidance

Run `doctor --json` before making environment assumptions. Report failed checks and actionable fixes, not a wall of raw environment text.

## Structured output

`--json` returns checks with name, status (`ok`, `warn`, `fail`, `skipped`), summary, details, and remediation hints. Website schema: `/docs/schemas/#doctor-json`.

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
