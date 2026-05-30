---
name: jbx-init
description: Create Java 25+ scripts from built-in or imported templates.
---

# jbx-init

Create Java 25+ scripts from built-in or imported templates.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-init
```

## Use when

- Start a new compact Java script with the expected shebang/directive style.
- Create a small CLI skeleton with Picocli or team defaults.
- Generate a test file that can immediately run through `jbx test`.

## Quick commands

```sh
jbx init Hello.java
jbx init --template cli tools/hello.java
jbx init --template test src/test/java/SmokeTest.java
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Start a new compact Java script with the expected shebang/directive style.
- Create a small CLI skeleton with Picocli or team defaults.
- Generate a test file that can immediately run through `jbx test`.

## Agent guidance

This creates files. If the target exists, do not overwrite without explicit instruction. After creation, inspect the file and run the narrowest validation command.

## Structured output

No `--json` mode yet. The output is a newly created file; verify by checking the file and running `jbx check --json`.

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
