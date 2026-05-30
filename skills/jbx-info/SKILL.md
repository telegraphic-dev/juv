---
name: jbx-info
description: Print parsed directives and derived metadata from Java scripts.
---

# jbx-info

Print parsed directives and derived metadata from Java scripts.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-info
```

## Use when

- Read `//DEPS` before modifying a script.
- Check which Java version or main class a script declares.
- Extract docs/cache metadata for packaging or publishing workflows.

## Quick commands

```sh
jbx info deps Hello.java
jbx info repos Hello.java
jbx info java Hello.java
jbx info directives Hello.java
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Read `//DEPS` before modifying a script.
- Check which Java version or main class a script declares.
- Extract docs/cache metadata for packaging or publishing workflows.

## Agent guidance

Prefer `info` over ad-hoc parsing of `//` directives. If multiple facts are needed, call the specific subcommands and keep each output scoped.

## Structured output

No global `--json` mode yet; subcommands return focused text values. Use it for deterministic extraction instead of regexing source.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-info` from the target machine.
