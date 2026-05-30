---
title: jbx rewrite command
description: Preview or apply OpenRewrite recipes with jbx-managed dependencies and JDKs; discover modules and recipes.
---

# `rewrite`

Preview or apply OpenRewrite recipes with jbx-managed dependencies and JDKs; discover modules and recipes.

## When to use it

- Preview a modernization recipe and review the patch before touching files.
- Discover which recipe module contains a migration an agent wants to run.
- Apply a mechanical cleanup after tests already cover the behavior.

## Common workflows

```bash
jbx rewrite modules --search spring --json
jbx rewrite recipes org.openrewrite.recipe:rewrite-testing-frameworks:3.8.0 --detail --json
jbx rewrite patch --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
jbx rewrite apply --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
```

## Real-life examples

### Repository maintenance

Use `rewrite` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-rewrite`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

Default to `patch`, not `apply`. Treat `apply` as a mutating operation that needs an explicit task. After applying, run `jbx check --json` and relevant tests.

## JSON and schema

JSON modes exist for `patch`, `apply`, `modules`, and `recipes`. Schemas are summarized at `/docs/schemas/#rewrite-json`.

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-rewrite
```
