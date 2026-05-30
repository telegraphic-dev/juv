---
name: jbx-alias
description: Add, remove, and list aliases from nearby `jbang-catalog.json` files.
---

# jbx-alias

Add, remove, and list aliases from nearby `jbang-catalog.json` files.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-alias
```

## Use when

- Expose a repository script as a stable team command.
- Inspect catalog aliases before deciding what `jbx <alias>` means.
- Clean up renamed scripts while keeping the catalog understandable.

## Quick commands

```sh
jbx alias list --json
jbx alias add hello Hello.java --description "Run the hello script"
jbx alias remove hello
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Expose a repository script as a stable team command.
- Inspect catalog aliases before deciding what `jbx <alias>` means.
- Clean up renamed scripts while keeping the catalog understandable.

## Agent guidance

Read `alias list --json` before changing a catalog. Preserve human descriptions because they become discovery text for future agents.

## Structured output

`jbx alias list --json` returns aliases and target metadata from the discovered catalog. Website schema: `/docs/schemas/#alias-json`.

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
