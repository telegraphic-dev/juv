---
name: jbx-alias
description: Add, remove, and list aliases from `jbang-catalog.json`.
---

# `alias`

Add, remove, and list aliases from nearby `jbang-catalog.json` files.

## When to use it

- Expose a repository script as a stable team command.
- Inspect catalog aliases before deciding what `jbx <alias>` means.
- Clean up renamed scripts while keeping the catalog understandable.

## Common workflows

```bash
jbx alias list --json
jbx alias add hello Hello.java --description "Run the hello script"
jbx alias remove hello
```

## Real-life examples

### Rename a repository script without breaking discovery

```bash
jbx alias list --json
jbx alias add smoke tools/smoke.java --description "Run the local smoke test"
jbx alias remove old-smoke
```

Use `alias` when a repo wants a stable command name in `jbang-catalog.json`. Read the catalog first, add the new alias with a useful description, then remove stale names only after callers have moved.

### Agent loop

1. Read the current catalog with `jbx alias list --json`.
2. Add or remove one alias at a time.
3. Re-read `jbx alias list --json` and confirm the target path and description.
4. Inspect `git diff jbang-catalog.json` before committing.

## Agent notes

Preserve descriptions; they are future discovery text for humans and agents. Do not guess alias targets from filenames if the catalog already says otherwise.

## JSON and schema

`jbx alias list --json` returns aliases and target metadata from the discovered catalog. Example output and field notes: [`alias JSON`](/docs/schemas/#alias-json).

Example JSON:

```json
[
  {
    "name": "hello",
    "scriptRef": "Hello.java",
    "script": "/workspace/Hello.java",
    "description": "Run the greeting script",
    "arguments": [],
    "dependencies": [],
    "repositories": [],
    "sources": [],
    "files": [],
    "classpaths": [],
    "compileOptions": [],
    "runtimeOptions": [],
    "javaAgents": [],
    "javaVersion": null,
    "mainClass": null
  }
]
```

## Verification checklist

- `jbx alias list --json` includes the expected alias name, target, and description.
- `jbang-catalog.json` contains only the intended catalog change.
- Removed aliases no longer appear in list output.
- A follow-up `jbx <alias>` resolves to the intended script when execution is safe.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx alias`

```text
Manage aliases from jbang-catalog.json

Usage: jbx alias <COMMAND>

Commands:
  add     Add alias for a script reference
  remove  Remove an existing alias
  list    List aliases from the nearest jbang-catalog.json
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx alias list`

```text
List aliases from the nearest jbang-catalog.json

Usage: jbx alias list [OPTIONS]

Options:
      --json  Print JSON instead of tab-separated text
  -h, --help  Print help
```

### `jbx alias add`

```text
Add alias for a script reference

Usage: jbx alias add [OPTIONS] <SCRIPT> [ARGS]...

Arguments:
  <SCRIPT>   Script path, URL, or alias reference
  [ARGS]...  Arguments stored in the alias and prepended at run time

Options:
  -g, --global
          Use the global user catalog file (~/.jbang/jbang-catalog.json)
  -f, --file <FILE>
          Path to the catalog file or directory to use
      --name <NAME>
          Alias name (defaults to the script filename stem)
      --description <DESCRIPTION>
          Description for the alias
      --force
          Force overwrite of an existing alias
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --source <SOURCES>
          Additional source file, same shape as //SOURCES
      --files <FILES>
          Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option, same shape as //JAVA_OPTIONS
      --java <JAVA_VERSION>
          Requested Java version
      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>
          Main class for the alias
      --docs <DOCS>
          Documentation reference for the alias
  -h, --help
          Print help
```

### `jbx alias remove`

```text
Remove an existing alias

Usage: jbx alias remove [OPTIONS] <NAME>

Arguments:
  <NAME>  Alias name to remove

Options:
  -g, --global       Use the global user catalog file (~/.jbang/jbang-catalog.json)
  -f, --file <FILE>  Path to the catalog file or directory to use
  -h, --help         Print help
```
