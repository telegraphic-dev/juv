---
name: jbx-rewrite
description: Preview or apply OpenRewrite recipes with jbx-managed dependencies and JDKs; discover modules and recipes.
---

# jbx-rewrite

Preview or apply OpenRewrite recipes with jbx-managed dependencies and JDKs; discover modules and recipes.

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-rewrite
```

## Use when

- Preview a modernization recipe and review the patch before touching files.
- Discover which recipe module contains a migration an agent wants to run.
- Apply a mechanical cleanup after tests already cover the behavior.

## Quick commands

```sh
jbx rewrite modules --search spring --json
jbx rewrite recipes org.openrewrite.recipe:rewrite-testing-frameworks:3.8.0 --detail --json
jbx rewrite patch --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
jbx rewrite apply --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
```

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

- Preview a modernization recipe and review the patch before touching files.
- Discover which recipe module contains a migration an agent wants to run.
- Apply a mechanical cleanup after tests already cover the behavior.

## Agent guidance

Default to `patch`, not `apply`. Treat `apply` as a mutating operation that needs an explicit task. After applying, run `jbx check --json` and relevant tests.

## Structured output

JSON modes exist for `patch`, `apply`, `modules`, and `recipes`. Schemas are summarized at `/docs/schemas/#rewrite-json`.

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
