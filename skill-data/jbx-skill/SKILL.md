---
name: jbx-skill
description: List and print version-matched bundled agent skills.
---

# `skill`

List and print version-matched bundled agent skills.

## When to use it

- Bootstrap an agent with command-specific guidance before touching a Java repo.
- Discover whether the installed jbx release knows a command or workflow.
- Keep offline automation aligned with the exact binary version, not a stale website page.

## Common workflows

```bash
jbx skill list
jbx skill list --json
```

## Real-life examples

### Bootstrap an agent for one command

```bash
jbx skill list --json
```

Use `skill` at the start of an automated workflow so the agent uses guidance bundled with the installed binary, not stale memory.

### Agent loop

1. Discover available skills with `jbx skill list --json`.
3. Follow the command-specific notes and examples.
## Agent notes

## JSON and schema

`jbx skill list --json` returns bundled skill names and descriptions. Example output and field notes: [`skill JSON`](/docs/schemas/#skill-json).

Example JSON:

```json
{
  "skills": [
    {
      "name": "jbx-check",
      "description": "Check Java source with structured diagnostics."
    }
  ]
}
```

## Verification checklist

- The requested skill name exists in `jbx skill list --json`.
- The fetched skill matches the installed jbx version used by the workflow.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx skill`

```text
Print version-matched agent skills bundled with this jbx release

Usage: jbx skill <COMMAND>

Commands:
  list  List version-matched skills bundled with this jbx binary
  get   Print a bundled skill. Defaults to the main jbx skill
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx skill list`

```text
List version-matched skills bundled with this jbx binary

Usage: jbx skill list [OPTIONS]

Options:
      --json  Emit structured JSON for agents
  -h, --help  Print help
```

```text
Print a bundled skill. Defaults to the main jbx skill

Arguments:
  [NAME]  Skill name to print. Defaults to jbx

Options:
  -h, --help  Print help
```
