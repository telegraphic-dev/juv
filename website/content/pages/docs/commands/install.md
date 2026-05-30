---

title: jbx install command
description: Install the current project into a Maven repository layout.
---

# `install`

Install the current project into a Maven repository layout, usually `~/.m2/repository`.

## When to use it

- Make a local artifact available to another script via `//DEPS`.
- Test generated POM metadata before publish.
- Install a snapshot into an isolated local repository for CI.

## Common workflows

```bash
jbx install --file jbx.json
jbx install --file jbx.json --destination build/local-m2
jbx install src/main/java/com/acme/Tool.java
```

## Real-life examples

### Install a local artifact into an isolated Maven repo

```bash
jbx install --file jbx.json --destination build/local-m2
jbx resolve --repo local=build/local-m2 com.acme:tool:1.0.0
```

Use `install` to test Maven metadata and local consumption before publishing or wiring another script to the artifact. The descriptor fields are documented in the [`jbx.json` reference](/docs/jbx-json/).

### Agent loop

1. Build or check the project first.
2. Install into an explicit temporary destination for CI/review loops.
3. Resolve the installed coordinate from that destination.
4. Inspect generated POM and metadata when publish behavior is involved.

## Agent notes

Defaulting to `~/.m2/repository` is convenient for humans but noisy for agents. Prefer `--destination build/local-m2` in reproducible workflows.

## Verification checklist

- Jar, sources/javadocs where applicable, POM, and metadata are present under Maven layout.
- A follow-up `jbx resolve --repo local=<dest> <gav>` succeeds.
- No real Maven Central publishing occurs from an install flow.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx install`

```text
Install the current project into a Maven repository layout

Usage: jbx install [OPTIONS] [SCRIPT]

Arguments:
  [SCRIPT]  Java source file to install. Defaults to jbx.json main when --file is used

Options:
      --file <FILE>                jbx descriptor file. Defaults to ./jbx.json when present
      --version <VERSION>          Override version from jbx.json or //GAV
      --destination <DESTINATION>  Destination Maven repository root. Defaults to ~/.m2/repository
      --target-dir <TARGET_DIR>    Working directory for staged install artifacts
      --package <PACKAGE_NAME>     Override package used when staging default-package sources
      --cache-dir <CACHE_DIR>      Override cache directory
  -h, --help                       Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-install
```
