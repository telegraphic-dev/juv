---

title: jbx rewrite command
description: Preview/apply OpenRewrite recipes and discover modules or recipes.
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

### Preview a formatting recipe before applying it

```bash
jbx rewrite modules --search spring --json
jbx rewrite recipes org.openrewrite.recipe:rewrite-testing-frameworks:3.8.0 --detail --json
jbx rewrite patch --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
```

Use `rewrite` for mechanical Java changes where a recipe can explain the diff better than hand edits.

### Agent loop

1. Discover the recipe module and recipe name with JSON commands.
2. Run `patch` first and inspect the proposed diff.
3. Apply only after tests/checks cover the edited area.
4. Run `jbx fmt`, `jbx check --json`, and relevant tests afterward.

## Agent notes

OpenRewrite can touch many files. Keep `--source` narrow, commit recipe changes separately from hand-written behavior changes, and never apply a recipe blindly across an unknown repo.

## JSON and schema

`rewrite modules`, `rewrite recipes`, `rewrite patch`, and `rewrite apply` support `--json` for discovery and change reports. Example output and field notes: [`rewrite JSON`](/docs/schemas/#rewrite-json).

Example JSON:

```json
[
  {
    "short": "spring",
    "coordinate": "org.openrewrite.recipe:rewrite-spring:6.9.0",
    "groupId": "org.openrewrite.recipe",
    "artifactId": "rewrite-spring",
    "version": "6.9.0",
    "versionCount": 123
  }
]
```


## Verification checklist

- Recipe/module coordinates are explicit.
- Patch output is reviewed before apply.
- Final diff contains expected mechanical changes only.
- `jbx check --json` and relevant tests pass after rewriting.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx rewrite`

```text
Run OpenRewrite recipes against Java source trees

Usage: jbx rewrite <COMMAND>

Commands:
  apply    Apply OpenRewrite recipes and modify sources
  patch    Preview OpenRewrite recipes and write rewrite/rewrite.patch without modifying sources
  modules  Search Maven Central for OpenRewrite modules
  recipes  List or search recipes available from an OpenRewrite module
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx rewrite patch`

```text
Preview OpenRewrite recipes and write rewrite/rewrite.patch without modifying sources

Usage: jbx rewrite patch [OPTIONS]

Options:
      --recipe <RECIPES>
          OpenRewrite recipe to run (short alias or fully-qualified recipe name). Repeatable and comma-splittable
      --module <MODULES>
          OpenRewrite module to add (short name for org.openrewrite:rewrite-*, or full GAV). Repeatable and comma-splittable
      --source <SOURCES>
          Java source file or directory. Repeatable; defaults to the current directory
      --option <OPTIONS>
          Recipe option as key=value. For multiple recipes, use RecipeName.key=value
      --report <REPORT>
          Report directory for rewrite.patch [default: rewrite]
      --json
          Print JSON summary after the human summary
      --fail-on-changes
          Exit with code 2 when recipes would make changes
      --no-fail-on-invalid-recipes
          Continue when OpenRewrite reports invalid active recipes
      --cache-dir <CACHE_DIR>
          Override dependency/helper cache directory
      --repo <REPOS>
          Additional repository for recipe modules
      --rewrite-version <REWRITE_VERSION>
          OpenRewrite version for built-in modules [default: 8.56.1]
  -h, --help
          Print help
```

### `jbx rewrite apply`

```text
Apply OpenRewrite recipes and modify sources

Usage: jbx rewrite apply [OPTIONS]

Options:
      --recipe <RECIPES>
          OpenRewrite recipe to run (short alias or fully-qualified recipe name). Repeatable and comma-splittable
      --module <MODULES>
          OpenRewrite module to add (short name for org.openrewrite:rewrite-*, or full GAV). Repeatable and comma-splittable
      --source <SOURCES>
          Java source file or directory. Repeatable; defaults to the current directory
      --option <OPTIONS>
          Recipe option as key=value. For multiple recipes, use RecipeName.key=value
      --report <REPORT>
          Report directory for rewrite.patch [default: rewrite]
      --json
          Print JSON summary after the human summary
      --fail-on-changes
          Exit with code 2 when recipes would make changes
      --no-fail-on-invalid-recipes
          Continue when OpenRewrite reports invalid active recipes
      --cache-dir <CACHE_DIR>
          Override dependency/helper cache directory
      --repo <REPOS>
          Additional repository for recipe modules
      --rewrite-version <REWRITE_VERSION>
          OpenRewrite version for built-in modules [default: 8.56.1]
  -h, --help
          Print help
```

### `jbx rewrite modules`

```text
Search Maven Central for OpenRewrite modules

Usage: jbx rewrite modules [OPTIONS]

Options:
      --search <SEARCH>
          Filter Maven Central modules by recipe/module name
      --group <GROUPS>
          Maven groupId to search. Defaults to org.openrewrite.recipe and org.openrewrite
      --limit <LIMIT>
          Maximum number of modules to print
      --json
          Print machine-readable JSON
      --rewrite-version <REWRITE_VERSION>
          OpenRewrite version used when expanding short module names
  -h, --help
          Print help
```

### `jbx rewrite recipes`

```text
List or search recipes available from an OpenRewrite module

Usage: jbx rewrite recipes [OPTIONS] <MODULE>

Arguments:
  <MODULE>  OpenRewrite module to inspect (short name or full GAV)

Options:
      --search <SEARCH>
          Filter recipes by short name, fully-qualified name, display name, or description
      --limit <LIMIT>
          Maximum number of recipes to print
      --detail
          Include recipe descriptions and options
      --json
          Print machine-readable JSON
      --cache-dir <CACHE_DIR>
          Override dependency/helper cache directory
      --repo <REPOS>
          Additional repository for recipe modules
      --rewrite-version <REWRITE_VERSION>
          OpenRewrite version for built-in modules [default: 8.56.1]
  -h, --help
          Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-rewrite
```
