---
name: jbx-template
description: List built-in and imported templates for `jbx init`.
---

# `template`

List built-in and imported templates for `jbx init`.

## When to use it

- Choose a starting point for a CLI, test, or compact script.
- Let an agent discover available project conventions before creating files.
- Document which templates a team catalog provides.

## Common workflows

```bash
jbx template list
jbx template list --json
jbx init --template cli Hello.java
```

## Real-life examples

### Choose a starter before generating a file

```bash
jbx template list --json
jbx init --template cli tools/hello.java
```

Use `template` before `init` when you need to know whether the repo or imported catalogs already provide the right starter.

### Agent loop

1. List templates with `jbx template list --json`.
2. Match the template purpose to the requested output: compact script, CLI, agent tool, or test.
3. Generate with `jbx init --template <name> <path>`.
4. Run the natural gate for the generated file.

## Agent notes

Templates encode conventions. Prefer an existing team/catalog template over inventing a new file shape.

## JSON and schema

`jbx template list --json` returns built-in and catalog template metadata. Example output and field notes: [`template JSON`](/docs/schemas/#template-json).

Example JSON:

```json
[
  {
    "name": "compact",
    "description": "Java 25 compact-source Hello World script"
  },
  {
    "name": "test",
    "description": "JUnit test class"
  }
]
```

## Verification checklist

- Selected template appears in list output.
- Generated file path and template match the task.
- The file passes `jbx check --json`, `jbx test --json`, or `jbx run` as appropriate.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx template`

```text
List init templates

Usage: jbx template <COMMAND>

Commands:
  list  List built-in init templates
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx template list`

```text
List built-in init templates

Usage: jbx template list [OPTIONS]

Options:
      --json  Print JSON instead of tab-separated text
  -h, --help  Print help
```
