---
name: jbx-app
description: Install, list, or uninstall Java scripts as PATH commands.
---

# jbx-app

Install, list, or uninstall Java scripts as PATH commands.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-app
```

## Use when

- Promote a frequently used Java script into a normal shell command.
- Install a repository-local developer tool in CI images.
- Remove stale wrappers after a script is renamed.

## Quick commands

```sh
jbx app install tools/report.java --name report
jbx app list
jbx app uninstall report
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Promote a frequently used Java script into a normal shell command.
- Install a repository-local developer tool in CI images.
- Remove stale wrappers after a script is renamed.

## Agent guidance

Installing modifies user PATH-facing state. Confirm intent unless the task explicitly asks for installation. After install, run the command with `--help` or a harmless argument.

## Structured output

No `--json` mode yet. Use `app list` for installed command names and paths.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-app` from the target machine.
