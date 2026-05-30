---
name: jbx-docs
description: Generate Markdown or JSON documentation from local Java sources, directories, docs sidecars, or Maven artifacts.
---

# jbx-docs

Generate Markdown or JSON documentation from local Java sources, directories, docs sidecars, or Maven artifacts.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-docs
```

## Use when

- Inspect an unfamiliar dependency before writing integration code.
- Publish or consume sidecar docs that agents can read without decompiling jars.
- Generate local API notes for a small script or library as part of CI.

## Quick commands

```sh
jbx docs src/main/java
jbx docs com.fasterxml.jackson.core:jackson-databind:2.17.2 --json
jbx docs docs/my-library-jbx-docs.json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Inspect an unfamiliar dependency before writing integration code.
- Publish or consume sidecar docs that agents can read without decompiling jars.
- Generate local API notes for a small script or library as part of CI.

## Agent guidance

Use docs before guessing APIs. Prefer JSON when extracting types/methods programmatically; use Markdown for human handoff. If a Maven artifact has a sidecar, trust the sidecar version that matches the artifact coordinate.

## Structured output

`--json` follows the published docs sidecar schema in `/docs/jbx-docs-schema/` and is summarized at `/docs/schemas/#docs-json`.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-docs` from the target machine.
