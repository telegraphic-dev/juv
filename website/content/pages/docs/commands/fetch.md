---

title: jbx fetch command
description: Download artifacts and print classpath or dependency coordinates.
---

# `fetch`

Download artifacts and print classpath or dependency coordinates.

## When to use it

- Materialize jars before an offline CI step.
- Build a classpath for an external Java command.
- Verify that dependency artifacts are reachable from configured repositories.

## Common workflows

```bash
jbx fetch com.fasterxml.jackson.core:jackson-databind:2.17.2
jbx fetch org.junit.platform:junit-platform-console-standalone:1.11.4
jbx fetch --deps-only com.acme:app:1.0.0
```

## Real-life examples

### Materialize a classpath for an external Java command

```bash
jbx fetch org.junit.platform:junit-platform-console-standalone:1.11.4
jbx fetch --deps-only com.fasterxml.jackson.core:jackson-databind:2.17.2
```

Use `fetch` when another tool needs jars or dependency coordinates but should not run project code.

### Agent loop

1. Resolve the exact coordinate or dependency directive first.
2. Fetch normally when another process needs jar paths; stdout is the classpath by default.
3. Use `--deps-only` when you only need coordinates for documentation or lockstep review.
4. Confirm the returned paths exist before invoking the external command.

## Agent notes

Fetching proves artifacts are reachable, not that the dependency graph is semantically correct. Use `resolve` when graph shape matters.

## Verification checklist

- All printed jar paths exist locally when classpath output is used.
- The coordinate includes the intended version.
- Extra repositories are explicit and reviewed before fetching from them.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx fetch`

```text
Fetch Maven dependency artifacts and print classpath

Usage: jbx fetch [OPTIONS] <COORDINATES>...

Arguments:
  <COORDINATES>...  Maven coordinates to fetch (groupId:artifactId:version)

Options:
      --repo <REPOS>           Additional repository (id=url format or bare URL)
      --cache-dir <CACHE_DIR>  Override cache directory
      --deps-only              Print resolved coordinates instead of classpath
  -h, --help                   Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-fetch
```
