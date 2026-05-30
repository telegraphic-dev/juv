---
title: jbx template command
description: List built-in and imported templates for `jbx init`.
---

# `template`

List built-in and imported templates for `jbx init`.

## When to use it

- Choose a starting point for a CLI, test, or compact script.
- Let an agent discover available project conventions before creating files.
- Document which templates a team catalog provides.

## Common workflows

```bash
jbx template list
jbx template list --json
jbx init --template cli Hello.java
```

## Real-life examples

### Repository maintenance

Use `template` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-template`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Run `template list --json` before `init` when the requested shape is vague. Prefer templates from the current repository/catalog over generic guesses.

## JSON and schema

`jbx template list --json` returns template names, descriptions, origin catalog, and parameters when known.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-template
```
