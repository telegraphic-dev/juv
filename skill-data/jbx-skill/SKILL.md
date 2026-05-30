---
name: jbx-skill
description: List and print version-matched bundled agent skills.
---

# jbx-skill

List and print version-matched bundled agent skills.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-skill
```

## Use when

- Bootstrap an agent with command-specific guidance before touching a Java repo.
- Discover whether the installed jbx release knows a command or workflow.
- Keep offline automation aligned with the exact binary version, not a stale website page.

## Quick commands

```sh
jbx skill list
jbx skill list --json
jbx skill get jbx-check
jbx skill get jbx
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Bootstrap an agent with command-specific guidance before touching a Java repo.
- Discover whether the installed jbx release knows a command or workflow.
- Keep offline automation aligned with the exact binary version, not a stale website page.

## Agent guidance

This is the first command an agent should run. Fetch the specific command skill, follow it, then use the command page only for broader human context.

## Structured output

`jbx skill list --json` returns installed skill names and descriptions. `skill get` returns Markdown skill content.

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
