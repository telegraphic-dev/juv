---
title: jbx catalog command
description: Add and list external catalogs in `jbang-catalog.json`.
---

# `catalog`

Add and list external catalogs in `jbang-catalog.json`.

## When to use it

- Share common script aliases across repositories.
- Inspect imported catalogs before resolving an alias.
- Add a team catalog during project setup.

## Common workflows

```bash
jbx catalog list --json
jbx catalog add team https://example.com/jbang-catalog.json
jbx catalog add local ./tools/jbang-catalog.json
```

## Real-life examples

### Repository maintenance

Use `catalog` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-catalog`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Catalog changes affect command discovery. List first, avoid duplicate names, and prefer pinned/reviewed URLs over random raw links.

## JSON and schema

`jbx catalog list --json` returns catalog names, URLs, and local resolution details. Website schema: `/docs/schemas/#catalog-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-catalog
```
