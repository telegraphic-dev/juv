---
title: jbx check command
description: Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.
---

# `check`

Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.

## When to use it

- Gate an agent edit before running tests.
- Collect exact diagnostics for a PR review comment or automated repair loop.
- Check a Java 25 compact script where a plain build tool would not understand the wrapper semantics.

## Common workflows

```bash
jbx check src --json
jbx check src/main/java --json
jbx check Hello.java --no-error-prone --json
jbx check src test --fail-on-warn --json
```

## Real-life examples

### Repository maintenance

Use `check` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-check`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Use this as the first quality gate after edits. JSON diagnostics are the contract; make fixes from structured file/line/column fields, then rerun until the status is clean or only accepted warnings remain.

## JSON and schema

`--json` returns command status and diagnostics with file, line, column, severity, tool, code/message, and suggested next action. Website schema: `/docs/schemas/#check-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-check
```
