---
title: jbx trust command
description: Pin, list, remove, or clear trusted hashes for remote scripts.
---

# `trust`

Pin, list, remove, or clear trusted hashes for remote scripts.

## When to use it

- Pin a reviewed remote script before automation runs it.
- Rotate trust after a remote script intentionally changes.
- Audit which URLs are allowed to run without prompting.

## Common workflows

```bash
jbx trust list
jbx trust add https://example.com/tool.java
jbx trust remove https://example.com/tool.java
```

## Real-life examples

### Repository maintenance

Use `trust` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-trust`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Remote trust changes are security-sensitive. Ask before adding/removing trust unless the user explicitly requested it, and always show the URL/hash being trusted.

## JSON and schema

No `--json` mode yet. Trust operations are small and human-auditable; use explicit subcommands and verify the listed hash after changes.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-trust
```
