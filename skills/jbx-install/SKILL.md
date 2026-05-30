---
name: jbx-install
description: Install the current project into a Maven repository layout, usually `~/.m2/repository`.
---

# jbx-install

Install the current project into a Maven repository layout, usually `~/.m2/repository`.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-install
```

## Use when

- Make a local artifact available to another script via `//DEPS`.
- Test generated POM metadata before publish.
- Install a snapshot into an isolated local repository for CI.

## Quick commands

```sh
jbx install --file jbx.json
jbx install --file jbx.json --repo build/local-m2
jbx install src/main/java/com/acme/Tool.java
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Make a local artifact available to another script via `//DEPS`.
- Test generated POM metadata before publish.
- Install a snapshot into an isolated local repository for CI.

## Agent guidance

Prefer a temporary `--repo` during automated tests to avoid polluting the developer’s real `~/.m2`. Verify the installed coordinates by resolving them.

## Structured output

No `--json` mode yet. Verification is the installed POM/JAR path under the target repository.

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

> Tip: for exact release behavior, rerun `jbx skill get jbx-install` from the target machine.
