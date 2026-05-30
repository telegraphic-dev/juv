---
name: jbx-template
description: List built-in and imported templates for `jbx init`.
---

# jbx-template

List built-in and imported templates for `jbx init`.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-template
```

## Use when

- Choose a starting point for a CLI, test, or compact script.
- Let an agent discover available project conventions before creating files.
- Document which templates a team catalog provides.

## Quick commands

```sh
jbx template list
jbx template list --json
jbx init --template cli Hello.java
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Choose a starting point for a CLI, test, or compact script.
- Let an agent discover available project conventions before creating files.
- Document which templates a team catalog provides.

## Agent guidance

Run `template list --json` before `init` when the requested shape is vague. Prefer templates from the current repository/catalog over generic guesses.

## Structured output

`jbx template list --json` returns template names, descriptions, origin catalog, and parameters when known.

## Common mistakes

- Do not infer command semantics from old web snippets; this skill reflects the installed release.
- Do not scrape human output when a JSON mode exists.
- Do not widen scope from a single file to the whole repository until the focused command is clean.
- Do not hide non-zero exits behind a successful parser or wrapper script.

## Verification

- Parse JSON output where available and validate required fields.
- For file changes, inspect `git diff --stat` and the exact changed files.
- For generated artifacts, test that the expected output path exists and is usable.
- For environment failures, run `jbx doctor --json` and report the failed checks with remediation.

> Tip: for exact release behavior, rerun `jbx skill get jbx-template` from the target machine.
