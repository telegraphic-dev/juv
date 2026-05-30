---
title: jbx jdk command
description: List, install, and locate JDKs used by jbx.
---

# `jdk`

List, install, and locate JDKs used by jbx.

## When to use it

- Verify which JDK will run Java 25 compact scripts.
- Install a missing Temurin JDK for a CI runner.
- Debug why JAVA_HOME differs from the JDK selected by jbx.

## Common workflows

```bash
jbx jdk list
jbx jdk home 25
jbx jdk install 25
```

## Real-life examples

### Repository maintenance

Use `jdk` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-jdk`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

JDK installation downloads external binaries and changes local state. Prefer `doctor --json` and `jdk list` first; install only when needed and requested.

## JSON and schema

No `--json` mode yet. Use `jbx doctor --json` for structured environment checks until JDK subcommands grow dedicated JSON.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-jdk
```
