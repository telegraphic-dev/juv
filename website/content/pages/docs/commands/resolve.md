---
title: jbx resolve command
description: Resolve Maven coordinates to dependency coordinates without running code.
---

# `resolve`

Resolve Maven coordinates to dependency coordinates without running code.

## When to use it

- Inspect the dependency graph before committing a new `//DEPS`.
- Check whether exclusions or runtime scopes change the graph.
- Debug version mediation without executing user code.

## Common workflows

```bash
jbx resolve com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx resolve --scope runtime org.slf4j:slf4j-simple:2.0.17
jbx resolve --tree com.acme:app:1.0.0
```

## Real-life examples

### Repository maintenance

Use `resolve` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-resolve`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Use `resolve` for metadata questions. It should not be treated as proof that jars are already present locally; use `fetch` for that.

## JSON and schema

No `--json` mode yet. Output is dependency coordinates or classpath-style text depending on flags. Use `fetch` when artifacts must be downloaded.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-resolve
```
