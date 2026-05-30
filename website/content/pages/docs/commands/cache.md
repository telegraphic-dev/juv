---

title: jbx cache command
description: Inspect or clear compiled-script cache paths and entries.
---

# `cache`

Inspect or clear compiled-script cache paths and entries.

## When to use it

- Locate compiled outputs while debugging why a stale script runs.
- Prune old entries on a constrained CI runner.
- Let an agent verify that a build warmed the expected cache namespace.

## Common workflows

```bash
jbx cache path
jbx cache list --json
jbx cache clear
```

## Real-life examples

### Debug a stale compiled script on CI

```bash
jbx cache path
jbx cache list --json
jbx cache clear
```

Use `cache` to answer “what did jbx compile and where did it put it?” before deleting anything. Clearing cache is a repair step, not a first reflex.

### Agent loop

1. Locate the active cache with `jbx cache path`.
2. Inspect entries with `jbx cache list --json`.
3. Clear only when stale or corrupt entries explain the failure.
4. Re-run the original `build`, `check`, or `run` command to recreate evidence.

## Agent notes

Cache state is machine-local. Do not treat it as project state, and do not commit generated cache files.

## JSON and schema

`jbx cache list --json` returns compiled-script cache entries. Example output and field notes: [`cache JSON`](/docs/schemas/#cache-json).

Example JSON:

```json
[
  {
    "script": "/workspace/Hello.java",
    "classesDir": "/workspace/.jbx-cache/e2db3fcb2d67e4ce/classes",
    "cacheDir": "/workspace/.jbx-cache/e2db3fcb2d67e4ce"
  }
]
```


## Verification checklist

- `jbx cache path` points to the expected user/cache directory.
- `jbx cache list --json` parses successfully before and after cleanup.
- The original command succeeds after clearing only if cache corruption was the cause.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx cache`

```text
Manage compiled script cache

Usage: jbx cache <COMMAND>

Commands:
  clear  Clear the jbx cache directory
  path   Print the effective jbx cache directory
  list   List cached script entries
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx cache path`

```text
Print the effective jbx cache directory

Usage: jbx cache path [OPTIONS]

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

### `jbx cache list`

```text
List cached script entries

Usage: jbx cache list [OPTIONS]

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
      --json                   Print cache entries as JSON
  -h, --help                   Print help
```

### `jbx cache clear`

```text
Clear the jbx cache directory

Usage: jbx cache clear [OPTIONS]

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-cache
```
