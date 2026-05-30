---
title: jbx test command
description: Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.
---

# `test`

Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.

## When to use it

- Run a small Java kata or library test suite without creating a full build file.
- Give an agent failing test names and stack traces in a parseable shape.
- Collect coverage during a refactor to prove the edited code path is exercised.

## Common workflows

```bash
jbx test src/test/java --json
jbx test tests/CalculatorTest.java -- --select-method CalculatorTest#adds --json
jbx test --coverage --json
```

## Real-life examples

### Repository maintenance

Use `test` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-test`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Start with focused tests when repairing a failure, then broaden to the directory or suite. Preserve non-zero exits for failed tests; do not hide failures behind “JSON parsed successfully”.

## JSON and schema

`--json` reports status, selected tests, failures, console XML paths, and optional coverage paths/counters. Website schema: `/docs/schemas/#test-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-test
```
