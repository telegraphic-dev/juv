---
name: jbx-fmt
description: Format Java files with Palantir Java Format, including Java 25 compact scripts.
---

# jbx-fmt

Format Java files with Palantir Java Format, including Java 25 compact scripts.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-fmt
```

## Use when

- Normalize generated Java before committing.
- Format compact scripts that have leading directives/comments.
- Run after OpenRewrite or AST import to keep diffs readable.

## Quick commands

```sh
jbx fmt src/main/java
jbx fmt Hello.java
jbx fmt --check src test
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Normalize generated Java before committing.
- Format compact scripts that have leading directives/comments.
- Run after OpenRewrite or AST import to keep diffs readable.

## Agent guidance

Formatting mutates files unless check mode is used. Inspect the diff afterwards and do not combine formatting noise with semantic edits unless requested.

## Structured output

No `--json` mode yet. Formatting is verified by clean diff or a check-mode exit code when available.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-fmt` from the target machine.
