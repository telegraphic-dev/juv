---
title: jbx install command
description: Install the current project into a Maven repository layout, usually `~/.m2/repository`.
---

# `install`

Install the current project into a Maven repository layout, usually `~/.m2/repository`.

## When to use it

- Make a local artifact available to another script via `//DEPS`.
- Test generated POM metadata before publish.
- Install a snapshot into an isolated local repository for CI.

## Common workflows

```bash
jbx install --file jbx.json
jbx install --file jbx.json --repo build/local-m2
jbx install src/main/java/com/acme/Tool.java
```

## Real-life examples

### Repository maintenance

Use `install` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-install`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Prefer a temporary `--repo` during automated tests to avoid polluting the developer’s real `~/.m2`. Verify the installed coordinates by resolving them.

## JSON and schema

No `--json` mode yet. Verification is the installed POM/JAR path under the target repository.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-install
```
