---

title: jbx app command
description: Install, list, or uninstall Java scripts as PATH commands.
---

# `app`

Install, list, or uninstall Java scripts as PATH commands.

## When to use it

- Promote a frequently used Java script into a normal shell command.
- Install a repository-local developer tool in CI images.
- Remove stale wrappers after a script is renamed.

## Common workflows

```bash
jbx app install tools/report.java --name report
jbx app list
jbx app uninstall report
```

## Real-life examples

### Install a checked-in tool as a shell command

```bash
jbx app install tools/report.java --name report
jbx app list
report --help
```

Use `app` for scripts the team runs repeatedly: release helpers, report generators, local maintenance tools. Keep the installed command name boring and stable.

### Agent loop

1. Inspect the script with `jbx info directives tools/report.java`.
2. Install with an explicit `--name`.
3. Confirm `jbx app list` shows the wrapper.
4. Run the wrapper's `--help` or another harmless command before using it in automation.

## Agent notes

Installing an app changes the user's PATH-visible tool surface. Prefer explicit names and remove old wrappers with `jbx app uninstall` when a script is renamed.

## Verification checklist

- `jbx app list` shows the installed command.
- The wrapper command resolves on PATH in the intended shell or CI image.
- `jbx app uninstall <name>` removes stale wrappers when cleaning up.
- The underlying script still passes `jbx check --json` if it is source-controlled.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx app`

```text
Manage scripts installed as commands on PATH

Usage: jbx app <COMMAND>

Commands:
  install    Install a script as a command on PATH
  uninstall  Remove an installed command
  list       List installed script commands
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx app install`

```text
Install a script as a command on PATH

Usage: jbx app install [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file to install

Options:
  -n, --name <NAME>  Command name (defaults to the script filename stem)
      --force        Force overwrite an existing command
  -h, --help         Print help
```

### `jbx app list`

```text
List installed script commands

Usage: jbx app list

Options:
  -h, --help  Print help
```

### `jbx app uninstall`

```text
Remove an installed command

Usage: jbx app uninstall <NAME>

Arguments:
  <NAME>  Command name to remove

Options:
  -h, --help  Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-app
```
