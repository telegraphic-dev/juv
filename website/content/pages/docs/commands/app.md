---
title: jbx app command
description: Install, list, or uninstall Java scripts as PATH commands.
---

# `app`

Install, list, or uninstall Java scripts as PATH commands.

## When to use it

- Promote a frequently used Java script into a normal shell command.
- Install a repository-local developer tool in CI images.
- Remove stale wrappers after a script is renamed.

## Common workflows

```bash
jbx app install tools/report.java --name report
jbx app list
jbx app uninstall report
```

## Real-life examples

### Repository maintenance

Use `app` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-app`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Installing modifies user PATH-facing state. Confirm intent unless the task explicitly asks for installation. After install, run the command with `--help` or a harmless argument.

## JSON and schema

No `--json` mode yet. Use `app list` for installed command names and paths.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-app
```
