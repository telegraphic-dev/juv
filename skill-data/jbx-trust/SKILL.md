---
name: jbx-trust
description: Pin, list, remove, or clear trusted hashes for remote scripts.
---

# jbx-trust

Pin, list, remove, or clear trusted hashes for remote scripts.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-trust
```

## Use when

- Pin a reviewed remote script before automation runs it.
- Rotate trust after a remote script intentionally changes.
- Audit which URLs are allowed to run without prompting.

## Quick commands

```sh
jbx trust list
jbx trust add https://example.com/tool.java
jbx trust remove https://example.com/tool.java
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Pin a reviewed remote script before automation runs it.
- Rotate trust after a remote script intentionally changes.
- Audit which URLs are allowed to run without prompting.

## Agent guidance

Remote trust changes are security-sensitive. Ask before adding/removing trust unless the user explicitly requested it, and always show the URL/hash being trusted.

## Structured output

No `--json` mode yet. Trust operations are small and human-auditable; use explicit subcommands and verify the listed hash after changes.

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
