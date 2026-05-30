---
name: jbx-export
description: Export local, portable, or native runnable artifacts.
---

# jbx-export

Export local, portable, or native runnable artifacts.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-export
```

## Use when

- Package a script for a machine that should not re-resolve dependencies at runtime.
- Create a portable directory with jars and launch metadata for CI artifacts.
- Build a native executable when GraalVM/native-image is available.

## Quick commands

```sh
jbx export local Hello.java --output build/hello.jar
jbx export portable Hello.java --output dist/hello
jbx export native Hello.java --output dist/hello
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Package a script for a machine that should not re-resolve dependencies at runtime.
- Create a portable directory with jars and launch metadata for CI artifacts.
- Build a native executable when GraalVM/native-image is available.

## Agent guidance

Export is a build artifact operation. Check the output path, run the produced artifact with a harmless argument, and keep native-image failures actionable rather than swallowing tool output.

## Structured output

No `--json` mode yet. Verify produced files directly in the requested output directory.

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
