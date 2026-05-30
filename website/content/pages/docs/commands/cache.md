---
title: jbx cache command
description: Inspect or clear compiled-script cache paths and entries.
---

# `cache`

Inspect or clear compiled-script cache paths and entries.

## When to use it

- Locate compiled outputs while debugging why a stale script runs.
- Prune old entries on a constrained CI runner.
- Let an agent verify that a build warmed the expected cache namespace.

## Common workflows

```bash
jbx cache path
jbx cache list --json
jbx cache clear --older-than 30d
```

## Real-life examples

### Repository maintenance

Use `cache` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-cache`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Never clear caches blindly during a debugging task; list first and scope destructive cleanup. Cache paths are machine-local facts, not portable configuration.

## JSON and schema

`jbx cache list --json` returns cache entries with paths, keys, sizes, and timestamps when available. Other cache subcommands are path/mutation oriented.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-cache
```
