---
title: jbx alias command
description: Add, remove, and list aliases from nearby `jbang-catalog.json` files.
---

# `alias`

Add, remove, and list aliases from nearby `jbang-catalog.json` files.

## When to use it

- Expose a repository script as a stable team command.
- Inspect catalog aliases before deciding what `jbx <alias>` means.
- Clean up renamed scripts while keeping the catalog understandable.

## Common workflows

```bash
jbx alias list --json
jbx alias add hello Hello.java --description "Run the hello script"
jbx alias remove hello
```

## Real-life examples

### Repository maintenance

Use `alias` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-alias`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Read `alias list --json` before changing a catalog. Preserve human descriptions because they become discovery text for future agents.

## JSON and schema

`jbx alias list --json` returns aliases and target metadata from the discovered catalog. Website schema: `/docs/schemas/#alias-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-alias
```
