---
title: jbx build command
description: Compile a script into the jbx cache without running it.
---

# `build`

Compile a script into the jbx cache without running it.

## When to use it

- Precompile a script during CI so the later run path starts from a warm cache.
- Validate generated Java without executing side effects.
- Check that dependency directives and CLI dependency overrides resolve together.

## Common workflows

```bash
jbx build scripts/Report.java
jbx build --deps org.slf4j:slf4j-api:2.0.17 tools/Probe.java
jbx build --java 25 Hello.java
```

## Real-life examples

### Repository maintenance

Use `build` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-build`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Prefer `build` over `run` when the task is “can this script compile?” rather than “execute this script”. Parse the process exit code; do not scrape localized compiler prose when `check --json` would fit better.

## JSON and schema

No `--json` mode yet. Success/failure is the exit code and compiler diagnostics are printed for humans. Use `jbx check --json` when an agent needs structured diagnostics.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-build
```
