---
title: jbx publish command
description: Build Maven Central-ready bundles, local served repositories, or Portal uploads from `jbx.json` and sources.
---

# `publish`

Build Maven Central-ready bundles, local served repositories, or Portal uploads from `jbx.json` and sources.

## When to use it

- Prepare a library or script artifact for Maven Central review.
- Create a local Maven repository for integration tests.
- Publish documentation sidecars next to Java artifacts so agents can inspect APIs.

## Common workflows

```bash
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --local-repo build/repo
jbx publish --file jbx.json --portal-upload
```

## Real-life examples

### Repository maintenance

Use `publish` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-publish`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Publishing can be external and irreversible. Use `--dry-run` first, inspect generated POMs/artifacts/signatures, and ask before real Portal upload unless explicitly requested.

## JSON and schema

No `--json` mode yet. Use dry-run output and generated bundle files as the verification contract.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-publish
```
