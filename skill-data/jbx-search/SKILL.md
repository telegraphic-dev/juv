---
name: jbx-search
description: Search Maven Central artifacts by text or coordinates, with filters.
---

# jbx-search

Search Maven Central artifacts by text or coordinates, with filters.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-search
```

## Use when

- Find the current coordinate for a library before adding `//DEPS`.
- Resolve ambiguous artifact names when a README only gives a product name.
- Let an agent rank candidate dependencies without scraping Maven Central HTML.

## Quick commands

```sh
jbx search picocli --json
jbx search --group org.junit.platform console standalone --json
jbx search --latest com.fasterxml.jackson.core:jackson-databind --json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Find the current coordinate for a library before adding `//DEPS`.
- Resolve ambiguous artifact names when a README only gives a product name.
- Let an agent rank candidate dependencies without scraping Maven Central HTML.

## Agent guidance

Prefer exact group/artifact filters once a candidate is known. Do not auto-upgrade production dependencies solely because search shows a newer version.

## Structured output

`--json` returns query metadata, `numFound`, and artifact records. Website schema: `/docs/schemas/#search-json`.

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
