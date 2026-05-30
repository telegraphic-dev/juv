---

title: jbx catalog command
description: Add and list external catalogs in `jbang-catalog.json`.
---

# `catalog`

Add and list external catalogs in `jbang-catalog.json`.

## When to use it

- Share common script aliases across repositories.
- Inspect imported catalogs before resolving an alias.
- Add a team catalog during project setup.

## Common workflows

```bash
jbx catalog list --json
jbx catalog add team https://example.com/jbang-catalog.json
jbx catalog add local ./tools/jbang-catalog.json
```

## Real-life examples

### Add a shared team catalog

```bash
jbx catalog list --json
jbx catalog add team https://example.com/jbang-catalog.json
jbx catalog list --json
```

Use `catalog` when a repository imports shared aliases or templates. List first so you do not add duplicate names or shadow a local catalog entry by accident.

### Agent loop

1. Read existing catalogs with `jbx catalog list --json`.
2. Add one catalog with a clear name and URL/path.
3. Re-read list output and verify the new entry.
4. Inspect `git diff jbang-catalog.json` before relying on imported aliases.

## Agent notes

Catalog URLs are part of the repository's executable discovery path. Prefer stable HTTPS URLs or checked-in relative paths; avoid personal scratch URLs.

## JSON and schema

`jbx catalog list --json` returns imported catalog names and locations. Example output and field notes: [`catalog JSON`](/docs/schemas/#catalog-json).

Example JSON:

```json
[
  {
    "name": "team",
    "catalogRef": "https://example.com/jbang-catalog.json",
    "catalog": "https://example.com/jbang-catalog.json",
    "description": null,
    "import": false
  }
]
```


## Verification checklist

- New catalog entry has the intended name and URL/path.
- Existing catalog entries are unchanged.
- Imported aliases/templates resolve only after the catalog is trusted by the repository owner.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx catalog`

```text
Manage external catalogs from jbang-catalog.json

Usage: jbx catalog <COMMAND>

Commands:
  add   Add an external catalog reference
  list  List external catalog references
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx catalog list`

```text
List external catalog references

Usage: jbx catalog list [OPTIONS]

Options:
      --json  Print JSON instead of tab-separated text
  -h, --help  Print help
```

### `jbx catalog add`

```text
Add an external catalog reference

Usage: jbx catalog add [OPTIONS] <NAME> <CATALOG_REF>

Arguments:
  <NAME>         Catalog name
  <CATALOG_REF>  Catalog path, URL, or directory

Options:
  -g, --global                     Use the global user catalog file (~/.jbang/jbang-catalog.json)
  -f, --file <FILE>                Path to the catalog file or directory to use
      --description <DESCRIPTION>  Description for the catalog
      --import                     Import aliases and templates from this catalog into local lookup
      --force                      Force overwrite of an existing catalog reference
  -h, --help                       Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-catalog
```
