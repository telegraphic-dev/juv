---
title: jbx docs command
description: Generate Markdown or JSON documentation from local Java sources, directories, docs sidecars, or Maven artifacts.
---

# `docs`

Generate Markdown or JSON documentation from local Java sources, directories, docs sidecars, or Maven artifacts.

## When to use it

- Inspect an unfamiliar dependency before writing integration code.
- Publish or consume sidecar docs that agents can read without decompiling jars.
- Generate local API notes for a small script or library as part of CI.

## Common workflows

```bash
jbx docs src/main/java
jbx docs com.fasterxml.jackson.core:jackson-databind:2.17.2 --json
jbx docs docs/my-library-jbx-docs.json
```

## Real-life examples

### Repository maintenance

Use `docs` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-docs`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Use docs before guessing APIs. Prefer JSON when extracting types/methods programmatically; use Markdown for human handoff. If a Maven artifact has a sidecar, trust the sidecar version that matches the artifact coordinate.

## JSON and schema

`--json` follows the published docs sidecar schema in `/docs/jbx-docs-schema/` and is summarized at `/docs/schemas/#docs-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-docs
```
