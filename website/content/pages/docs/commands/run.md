---
title: jbx run command
description: Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.
---

# `run`

Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.

## When to use it

- Run a one-file maintenance script from a repository without creating a Maven or Gradle project.
- Launch a Java 25 compact script that carries `//DEPS`, `//JAVA`, `//SOURCES`, and runtime options in the file.
- Smoke-test an executable example after `jbx check --json` has confirmed the source compiles.

## Common workflows

```bash
jbx run scripts/Report.java -- --month 2026-05
jbx scripts/Report.java --month 2026-05
jbx run --deps info.picocli:picocli:4.7.7 tools/Cli.java -- --help
```

## Real-life examples

### Repository maintenance

Use `run` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-run`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Treat `run` as the boundary where arbitrary user code executes. For autonomous loops, first inspect with `info`, compile with `build`, or validate with `check --json`; only run after the command and arguments are understood.

## JSON and schema

No `--json` mode: stdout/stderr belong to the program being run. Use `jbx check --json`, `jbx build`, `jbx info ...`, or `jbx doctor --json` for machine-readable preflight facts before execution.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-run
```
