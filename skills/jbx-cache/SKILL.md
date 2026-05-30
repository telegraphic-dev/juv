---
name: jbx-cache
description: Inspect or clear compiled-script cache paths and entries.
---

# jbx-cache

Inspect or clear compiled-script cache paths and entries.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-cache
```

## Use when

- Locate compiled outputs while debugging why a stale script runs.
- Prune old entries on a constrained CI runner.
- Let an agent verify that a build warmed the expected cache namespace.

## Quick commands

```sh
jbx cache path
jbx cache list --json
jbx cache clear --older-than 30d
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Locate compiled outputs while debugging why a stale script runs.
- Prune old entries on a constrained CI runner.
- Let an agent verify that a build warmed the expected cache namespace.

## Agent guidance

Never clear caches blindly during a debugging task; list first and scope destructive cleanup. Cache paths are machine-local facts, not portable configuration.

## Structured output

`jbx cache list --json` returns cache entries with paths, keys, sizes, and timestamps when available. Other cache subcommands are path/mutation oriented.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-cache` from the target machine.
