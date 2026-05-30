---
name: jbx
description: Single agent-friendly entry point to the Java ecosystem.
---

# jbx

Single agent-friendly entry point to the Java ecosystem.

`jbx` is the Java toolbox entry point for scripts, Maven tools, tests, formatting, publishing, dependency lookup, docs sidecars, diagnostics, cache management, and JDK handling. Keep the top-level usage thin: discover skills, then use the dedicated command.

## Install

```sh
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
jbx --version
```

## First commands for agents

```sh
jbx skill list --json
jbx skill get jbx
jbx skill get jbx-check
jbx doctor --json
jbx check [path...] --json
```

## Real-life workflows

### Repair a Java script after an edit

```sh
jbx skill get jbx-check
jbx check Hello.java --json
jbx build Hello.java
jbx run Hello.java -- --help
```

### Explore an unfamiliar dependency

```sh
jbx search picocli --json
jbx docs info.picocli:picocli:4.7.7 --json
jbx resolve info.picocli:picocli:4.7.7
```

### Prepare a safe modernization

```sh
jbx skill get jbx-rewrite
jbx rewrite modules --search format --json
jbx rewrite patch --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
jbx check src/main/java --json
```

### Publish or install locally

```sh
jbx publish --file jbx.json --dry-run
jbx install --file jbx.json --repo build/local-m2
jbx docs com.acme:tool:1.0.0 --json
```

## Command-specific skills

Every command has a bundled skill named `jbx-<command>`. Examples:

```sh
jbx skill get jbx-run
jbx skill get jbx-test
jbx skill get jbx-docs
jbx skill get jbx-publish
```

## Agent operating rules

1. Run `jbx skill list --json` to discover installed guidance.
2. Fetch the specific skill for the command you need.
3. Prefer JSON modes when they exist; parse JSON rather than scraping human text.
4. Use `jbx doctor --json` before guessing about JDKs, caches, Maven reachability, remote trust, formatter fallback, dependency drift, publishing, or native-image setup.
5. Verify generated artifacts directly: files for mutating commands, schemas for JSON commands, and exit codes for gates.

## Compatibility notes

- Preserve JBang-compatible command shape and directives unless a task explicitly asks for a difference.
- Preserve Java 25 compact/unnamed-class behavior unless a test proves otherwise.
- Prefer clear deterministic errors over silent partial compatibility.
- Keep agent-facing output parseable and documented.
