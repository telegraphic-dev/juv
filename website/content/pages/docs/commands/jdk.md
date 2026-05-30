---

title: jbx jdk command
description: List, install, and locate JDKs used by jbx.
---

# `jdk`

List, install, and locate JDKs used by jbx.

## When to use it

- Verify which JDK will run Java 25 compact scripts.
- Install a missing Temurin JDK for a CI runner.
- Debug why JAVA_HOME differs from the JDK selected by jbx.

## Common workflows

```bash
jbx jdk list
jbx jdk home 25
jbx jdk install 25
```

## Real-life examples

### Verify the Java 25 runtime for compact scripts

```bash
jbx jdk list
jbx jdk home 25
jbx doctor --json
```

Use `jdk` when `JAVA_HOME`, CI image defaults, or compact-source support are in doubt.

### Agent loop

1. List available JDKs with `jbx jdk list`.
2. Locate the required major version with `jbx jdk home <version>`.
3. Install only when the requested version is missing and installation is acceptable.
4. Re-run `jbx doctor --json` or the original command.

## Agent notes

Installing a JDK changes machine state. In shared CI images or developer machines, prefer reporting the missing version unless installation is explicitly part of the task.

## Verification checklist

- Required Java version has a concrete home path.
- The selected JDK matches the script's `//JAVA` or CLI `--java` requirement.
- The original compile/test/run command succeeds with that JDK.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx jdk`

```text
Manage installed JDKs

Usage: jbx jdk <COMMAND>

Commands:
  list     List discovered and installed JDKs
  install  Install a JDK from Adoptium (Eclipse Temurin)
  home     Show JDK home directory for a given version
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx jdk list`

```text
List discovered and installed JDKs

Usage: jbx jdk list

Options:
  -h, --help  Print help
```

### `jbx jdk install`

```text
Install a JDK from Adoptium (Eclipse Temurin)

Usage: jbx jdk install <VERSION>

Arguments:
  <VERSION>  JDK version to install (e.g. 21, 25, 25+)

Options:
  -h, --help  Print help
```

### `jbx jdk home`

```text
Show JDK home directory for a given version

Usage: jbx jdk home [VERSION]

Arguments:
  [VERSION]  JDK version (defaults to 25) [default: 25]

Options:
  -h, --help  Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-jdk
```
