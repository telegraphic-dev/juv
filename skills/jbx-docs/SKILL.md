---
name: jbx-docs
description: Generate Markdown or JSON documentation from local sources or Maven artifacts.
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

### Inspect a dependency before coding against it

```bash
jbx docs com.fasterxml.jackson.core:jackson-databind:2.17.2 --json
jbx docs src/main/java
```

Use `docs` to put API facts into the agent context without opening a browser or decompiling jars. Prefer remote sidecars when available; generate local docs fresh for local sources.

### Agent loop

1. Fetch docs for the exact source path or Maven coordinate.
2. Use Markdown for reading and `--json` for structured lookup.
3. Verify the documented type/member exists before editing caller code.
4. Cache only remote artifact docs; regenerate local source docs after edits.

## Agent notes

Docs output is context, not a build result. If docs reveal a method signature, still compile or test the caller after using it.

## JSON and schema

`jbx docs ... --json` returns docs metadata, generated source info, and structured type/member details. Example output and downloadable sample: [`docs JSON`](/docs/schemas/#docs-json).

Example JSON:

```json
{
  "schema": "https://jbx.telegraphic.dev/schemas/jbx-docs/v1.json",
  "target": "Hello.java",
  "sources": [
    {
      "path": "Hello.java",
      "name": "Hello.java",
      "description": "Prints a greeting.",
      "docs": [],
      "dependencies": []
    }
  ],
  "types": [],
  "generatedFrom": {
    "source": "jbx-directives",
    "jbxVersion": "0.0.0"
  }
}
```

## Verification checklist

- Coordinate includes a version when reproducibility matters.
- Local source docs are regenerated after source changes.
- Markdown sidecar and JSON sidecar agree for published artifacts.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx docs`

```text
Print agent-friendly documentation for source, directories, or Maven artifacts

Usage: jbx docs [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Maven GAV, Java source file, docs sidecar, or directory to document

Options:
      --json                   Print JSON instead of Markdown
      --repo <REPOS>           Additional repository for remote Maven docs sidecars (id=url format or bare URL)
      --type <TYPES>           Limit structured output to matching type names. Repeatable; accepts simple or fully-qualified names
      --cache-dir <CACHE_DIR>  Override remote docs cache directory
  -h, --help                   Print help
```

> For exact behavior, prefer the skill bundled with the `jbx` binary on the machine running the task.
