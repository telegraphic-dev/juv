---
title: jbx graph command
description: Dump JavaParser native AST JSON or import it back to Java source.
---

# `graph`

Dump JavaParser native AST JSON or import it back to Java source.

## When to use it

- Let an agent perform structural analysis without regexing Java.
- Round-trip a generated AST back to source after a controlled transformation.
- Debug parser behavior around compact Java scripts.

## Common workflows

```bash
jbx graph dump Hello.java > Hello.ast.json
jbx graph import Hello.ast.json --output Hello.java
jbx graph dump src/main/java/com/acme/App.java | jq .
```

## Real-life examples

### Repository maintenance

Use `graph` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-graph`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Prefer AST operations for structural changes, but verify with `jbx check --json` after import. Treat generated source as code changes requiring review.

## JSON and schema

`dump` emits JavaParser native AST JSON. `import` consumes that JSON and writes Java source. This is an AST interchange format, not a stable semantic schema for public APIs.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-graph
```
