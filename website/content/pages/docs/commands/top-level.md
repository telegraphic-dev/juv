---
title: jbx top-level command
description: Single agent-friendly entry point to the Java ecosystem.
---

# `jbx`

Single agent-friendly entry point to the Java ecosystem.

The top-level command stays intentionally thin. It gives humans a fast run path and gives agents a discovery path; detailed behavior lives in dedicated subcommands and version-matched skills.

## Start here

```bash
jbx skill list
jbx skill list --json
jbx skill get jbx
jbx skill get jbx-check
```

## Common thin entry-point workflows

```bash
jbx Hello.java world
jbx dev.telegraphic:hello-tool:1.0.0 -- --help
jbx skill get jbx-run
jbx doctor --json
```

## Command pages

- [`run`](/docs/commands/run/) — Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.
- [`build`](/docs/commands/build/) — Compile a script into the jbx cache without running it.
- [`check`](/docs/commands/check/) — Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.
- [`test`](/docs/commands/test/) — Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.
- [`docs`](/docs/commands/docs/) — Generate Markdown or JSON documentation from local Java sources, directories, docs sidecars, or Maven artifacts.
- [`doctor`](/docs/commands/doctor/) — Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.
- [`rewrite`](/docs/commands/rewrite/) — Preview or apply OpenRewrite recipes with jbx-managed dependencies and JDKs; discover modules and recipes.
- [`search`](/docs/commands/search/) — Search Maven Central artifacts by text or coordinates, with filters.
- [`resolve`](/docs/commands/resolve/) — Resolve Maven coordinates to dependency coordinates without running code.
- [`fetch`](/docs/commands/fetch/) — Download artifacts and print classpath or dependency coordinates.
- [`info`](/docs/commands/info/) — Print parsed directives and derived metadata from Java scripts.
- [`cache`](/docs/commands/cache/) — Inspect or clear compiled-script cache paths and entries.
- [`trust`](/docs/commands/trust/) — Pin, list, remove, or clear trusted hashes for remote scripts.
- [`app`](/docs/commands/app/) — Install, list, or uninstall Java scripts as PATH commands.
- [`alias`](/docs/commands/alias/) — Add, remove, and list aliases from nearby `jbang-catalog.json` files.
- [`catalog`](/docs/commands/catalog/) — Add and list external catalogs in `jbang-catalog.json`.
- [`template`](/docs/commands/template/) — List built-in and imported templates for `jbx init`.
- [`init`](/docs/commands/init/) — Create Java 25+ scripts from built-in or imported templates.
- [`export`](/docs/commands/export/) — Export local, portable, or native runnable artifacts.
- [`publish`](/docs/commands/publish/) — Build Maven Central-ready bundles, local served repositories, or Portal uploads from `jbx.json` and sources.
- [`install`](/docs/commands/install/) — Install the current project into a Maven repository layout, usually `~/.m2/repository`.
- [`fmt`](/docs/commands/fmt/) — Format Java files with Palantir Java Format, including Java 25 compact scripts.
- [`graph`](/docs/commands/graph/) — Dump JavaParser native AST JSON or import it back to Java source.
- [`skill`](/docs/commands/skill/) — List and print version-matched bundled agent skills.
- [`jdk`](/docs/commands/jdk/) — List, install, and locate JDKs used by jbx.

## Agent notes

- Use `jbx skill list --json` for discovery.
- Fetch the command-specific skill before running non-trivial commands.
- Prefer explicit subcommands over relying on top-level shorthand in automation.
- Treat top-level script/tool execution as code execution; inspect before running unknown inputs.

## JSON and schema

No top-level JSON mode. The top-level entry point forwards humans to common run paths and agents to `jbx skill`. Use dedicated JSON modes such as `jbx skill list --json`, `jbx doctor --json`, `jbx check --json`, `jbx docs --json`, `jbx search --json`, `jbx test --json`, and `jbx rewrite ... --json`.
