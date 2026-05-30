---
title: jbx info command
description: Print parsed directives and derived metadata from Java scripts.
---

# `info`

Print parsed directives and derived metadata from Java scripts.

## When to use it

- Read `//DEPS` before modifying a script.
- Check which Java version or main class a script declares.
- Extract docs/cache metadata for packaging or publishing workflows.

## Common workflows

```bash
jbx info deps Hello.java
jbx info repos Hello.java
jbx info java Hello.java
jbx info directives Hello.java
```

## Real-life examples

### Repository maintenance

Use `info` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-info`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Prefer `info` over ad-hoc parsing of `//` directives. If multiple facts are needed, call the specific subcommands and keep each output scoped.

## JSON and schema

No global `--json` mode yet; subcommands return focused text values. Use it for deterministic extraction instead of regexing source.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-info
```
