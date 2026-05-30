---
name: jbx-build
description: Compile a script into the jbx cache without running it.
---

# jbx-build

Compile a script into the jbx cache without running it.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-build
```

## Use when

- Precompile a script during CI so the later run path starts from a warm cache.
- Validate generated Java without executing side effects.
- Check that dependency directives and CLI dependency overrides resolve together.

## Quick commands

```sh
jbx build scripts/Report.java
jbx build --deps org.slf4j:slf4j-api:2.0.17 tools/Probe.java
jbx build --java 25 Hello.java
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Precompile a script during CI so the later run path starts from a warm cache.
- Validate generated Java without executing side effects.
- Check that dependency directives and CLI dependency overrides resolve together.

## Agent guidance

Prefer `build` over `run` when the task is “can this script compile?” rather than “execute this script”. Parse the process exit code; do not scrape localized compiler prose when `check --json` would fit better.

## Structured output

No `--json` mode yet. Success/failure is the exit code and compiler diagnostics are printed for humans. Use `jbx check --json` when an agent needs structured diagnostics.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-build` from the target machine.
