---

title: jbx doctor command
description: Diagnose JDKs, Maven, caches, trust, dependencies, and optional native/publish tools.
---

# `doctor`

Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.

## When to use it

- Diagnose why a script works on one machine but not in CI.
- Check whether a workstation has Java 25, Maven reachability, writable caches, and optional native-image support.
- Before filing a bug report, capture environment facts without exposing secrets.

## Common workflows

```bash
jbx doctor --json
jbx doctor Hello.java --json
jbx doctor https://example.com/tool.java --json
```

## Real-life examples

### Capture environment facts for a failing CI runner

```bash
jbx doctor --json
jbx doctor scripts/Report.java --json
```

Use `doctor` when the problem smells environmental: missing JDK, unwritable cache, Maven Central reachability, formatter fallback, remote trust, or native/publish tooling.

### Agent loop

1. Run `jbx doctor --json` before changing project code.
2. Inspect failing checks and warnings by name.
3. Repair the environment or document the missing optional tool.
4. Re-run the original `jbx` command after the doctor output is healthy enough.

## Agent notes

Doctor output can mention local paths and tool versions. Share summaries, not secrets or full home-directory dumps, unless the user asks.

## JSON and schema

`jbx doctor --json` returns top-level status and named checks. Example output and field notes: [`doctor JSON`](/docs/schemas/#doctor-json).

Example JSON:

```json
{
  "status": "ok",
  "target": null,
  "checks": [
    {
      "name": "jdk",
      "status": "ok",
      "summary": "JDK 25 selected at /home/user/.cache/jbx/jdks/25",
      "detail": null
    },
    {
      "name": "dependency-resolution",
      "status": "skipped",
      "summary": "no local script dependencies to resolve",
      "detail": null
    }
  ]
}
```


## Verification checklist

- Required checks pass for the task at hand.
- Optional native/publish checks are treated as optional unless that workflow is being used.
- The failing original command is rerun after environment repair.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx doctor`

```text
Diagnose the local jbx toolchain and a script when provided

Usage: jbx doctor [OPTIONS] [TARGET]

Arguments:
  [TARGET]  Java source file or remote URL to diagnose

Options:
      --json                   Emit structured JSON
      --cache-dir <CACHE_DIR>  Override cache directory checked for writability and dependency metadata
      --repo <REPOS>           Additional repository for dependency resolution health checks
      --publish                Check GPG signing tools too. By default GPG is skipped unless publishing is requested
      --native                 Check native-image too. By default native-image is skipped unless native export is requested
  -h, --help                   Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-doctor
```
