---
name: jbx-resolve
description: Resolve Maven coordinates to dependency coordinates or classpaths.
---

# `resolve`

Resolve Maven coordinates to dependency coordinates without running code.

## When to use it

- Inspect the dependency graph before committing a new `//DEPS`.
- Check whether exclusions or runtime scopes change the graph.
- Debug version mediation without executing user code.

## Common workflows

```bash
jbx resolve com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx resolve --repo snapshots=https://repo.example.com/snapshots com.acme:tool:1.0.0-SNAPSHOT
jbx resolve --classpath com.acme:app:1.0.0
```

## Real-life examples

### Review dependency graph before adding `//DEPS`

```bash
jbx resolve com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx resolve --classpath com.acme:app:1.0.0
```

Use `resolve` to understand Maven coordinates and classpaths without executing code.

### Agent loop

1. Resolve the exact candidate coordinate before editing source directives.
2. Check classpath output when a launcher/compiler needs jar paths.
3. Review version mediation and repository warnings.
4. Only then add or change `//DEPS` and run `jbx check --json`.

## Agent notes

Resolution can involve configured repositories. Treat new repositories as supply-chain changes, not harmless flags.

## Verification checklist

- Coordinate resolves to the intended version.
- Classpath output paths exist when `--classpath` is used.
- Dependency changes are reflected in source directives or descriptor files and pass `jbx check --json`.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx resolve`

```text
Resolve Maven dependencies without running

Usage: jbx resolve [OPTIONS] <COORDINATES>...

Arguments:
  <COORDINATES>...  Maven coordinates to resolve (groupId:artifactId:version)

Options:
      --repo <REPOS>           Additional repository (id=url format or bare URL)
      --cache-dir <CACHE_DIR>  Override cache directory
  -c, --classpath              Print classpath (JAR paths) instead of coordinates
  -h, --help                   Print help
```
