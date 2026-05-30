---
name: jbx-catalog
description: Add and list external catalogs in `jbang-catalog.json`.
---

# jbx-catalog

Add and list external catalogs in `jbang-catalog.json`.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-catalog
```

## Use when

- Share common script aliases across repositories.
- Inspect imported catalogs before resolving an alias.
- Add a team catalog during project setup.

## Quick commands

```sh
jbx catalog list --json
jbx catalog add team https://example.com/jbang-catalog.json
jbx catalog add local ./tools/jbang-catalog.json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Share common script aliases across repositories.
- Inspect imported catalogs before resolving an alias.
- Add a team catalog during project setup.

## Agent guidance

Catalog changes affect command discovery. List first, avoid duplicate names, and prefer pinned/reviewed URLs over random raw links.

## Structured output

`jbx catalog list --json` returns catalog names, URLs, and local resolution details. Website schema: `/docs/schemas/#catalog-json`.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-catalog` from the target machine.
