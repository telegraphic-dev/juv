---
title: jbx doctor command
description: Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.
---

# `doctor`

Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.

## When to use it

- Diagnose why a script works on one machine but not in CI.
- Check whether a workstation has Java 25, Maven reachability, writable caches, and optional native-image support.
- Before filing a bug report, capture environment facts without exposing secrets.

## Common workflows

```bash
jbx doctor --json
jbx doctor Hello.java --json
jbx doctor https://example.com/tool.java --json
```

## Real-life examples

### Repository maintenance

Use `doctor` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-doctor`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Run `doctor --json` before making environment assumptions. Report failed checks and actionable fixes, not a wall of raw environment text.

## JSON and schema

`--json` returns checks with name, status (`ok`, `warn`, `fail`, `skipped`), summary, details, and remediation hints. Website schema: `/docs/schemas/#doctor-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-doctor
```
