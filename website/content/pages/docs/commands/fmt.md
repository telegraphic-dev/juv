---
title: jbx fmt command
description: Format Java files with Palantir Java Format, including Java 25 compact scripts.
---

# `fmt`

Format Java files with Palantir Java Format, including Java 25 compact scripts.

## When to use it

- Normalize generated Java before committing.
- Format compact scripts that have leading directives/comments.
- Run after OpenRewrite or AST import to keep diffs readable.

## Common workflows

```bash
jbx fmt src/main/java
jbx fmt Hello.java
jbx fmt --check src test
```

## Real-life examples

### Repository maintenance

Use `fmt` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-fmt`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Formatting mutates files unless check mode is used. Inspect the diff afterwards and do not combine formatting noise with semantic edits unless requested.

## JSON and schema

No `--json` mode yet. Formatting is verified by clean diff or a check-mode exit code when available.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-fmt
```
