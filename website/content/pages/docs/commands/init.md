---
title: jbx init command
description: Create Java 25+ scripts from built-in or imported templates.
---

# `init`

Create Java 25+ scripts from built-in or imported templates.

## When to use it

- Start a new compact Java script with the expected shebang/directive style.
- Create a small CLI skeleton with Picocli or team defaults.
- Generate a test file that can immediately run through `jbx test`.

## Common workflows

```bash
jbx init Hello.java
jbx init --template cli tools/hello.java
jbx init --template test src/test/java/SmokeTest.java
```

## Real-life examples

### Repository maintenance

Use `init` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-init`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

This creates files. If the target exists, do not overwrite without explicit instruction. After creation, inspect the file and run the narrowest validation command.

## JSON and schema

No `--json` mode yet. The output is a newly created file; verify by checking the file and running `jbx check --json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-init
```
