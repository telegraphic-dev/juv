---
name: jbx-publish
description: Build Maven Central-ready bundles, local served repositories, or Portal uploads from `jbx.json` and sources.
---

# jbx-publish

Build Maven Central-ready bundles, local served repositories, or Portal uploads from `jbx.json` and sources.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-publish
```

## Use when

- Prepare a library or script artifact for Maven Central review.
- Create a local Maven repository for integration tests.
- Publish documentation sidecars next to Java artifacts so agents can inspect APIs.

## Quick commands

```sh
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --local-repo build/repo
jbx publish --file jbx.json --portal-upload
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Prepare a library or script artifact for Maven Central review.
- Create a local Maven repository for integration tests.
- Publish documentation sidecars next to Java artifacts so agents can inspect APIs.

## Agent guidance

Publishing can be external and irreversible. Use `--dry-run` first, inspect generated POMs/artifacts/signatures, and ask before real Portal upload unless explicitly requested.

## Structured output

No `--json` mode yet. Use dry-run output and generated bundle files as the verification contract.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-publish` from the target machine.
