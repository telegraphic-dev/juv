---
title: jbx search command
description: Search Maven Central artifacts by text or coordinates, with filters.
---

# `search`

Search Maven Central artifacts by text or coordinates, with filters.

## When to use it

- Find the current coordinate for a library before adding `//DEPS`.
- Resolve ambiguous artifact names when a README only gives a product name.
- Let an agent rank candidate dependencies without scraping Maven Central HTML.

## Common workflows

```bash
jbx search picocli --json
jbx search --group org.junit.platform console standalone --json
jbx search --latest com.fasterxml.jackson.core:jackson-databind --json
```

## Real-life examples

### Repository maintenance

Use `search` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-search`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Prefer exact group/artifact filters once a candidate is known. Do not auto-upgrade production dependencies solely because search shows a newer version.

## JSON and schema

`--json` returns query metadata, `numFound`, and artifact records. Website schema: `/docs/schemas/#search-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-search
```
