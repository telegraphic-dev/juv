---
title: jbx export command
description: Export local, portable, or native runnable artifacts.
---

# `export`

Export local, portable, or native runnable artifacts.

## When to use it

- Package a script for a machine that should not re-resolve dependencies at runtime.
- Create a portable directory with jars and launch metadata for CI artifacts.
- Build a native executable when GraalVM/native-image is available.

## Common workflows

```bash
jbx export local Hello.java --output build/hello.jar
jbx export portable Hello.java --output dist/hello
jbx export native Hello.java --output dist/hello
```

## Real-life examples

### Repository maintenance

Use `export` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-export`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Export is a build artifact operation. Check the output path, run the produced artifact with a harmless argument, and keep native-image failures actionable rather than swallowing tool output.

## JSON and schema

No `--json` mode yet. Verify produced files directly in the requested output directory.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-export
```
