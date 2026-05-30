---
title: jbx fetch command
description: Download artifacts and print classpath or dependency coordinates.
---

# `fetch`

Download artifacts and print classpath or dependency coordinates.

## When to use it

- Materialize jars before an offline CI step.
- Build a classpath for an external Java command.
- Verify that dependency artifacts are reachable from configured repositories.

## Common workflows

```bash
jbx fetch com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx fetch --classpath org.junit.platform:junit-platform-console-standalone:1.11.4
jbx fetch --deps-only com.acme:app:1.0.0
```

## Real-life examples

### Repository maintenance

Use `fetch` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-fetch`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Use `fetch` when file availability matters. Keep cache paths out of committed files and logs unless they are intentionally diagnostic.

## JSON and schema

No `--json` mode yet. Output is meant for shell composition: classpath strings, paths, or dependency lists.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-fetch
```
