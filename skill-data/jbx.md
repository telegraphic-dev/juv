---
name: jbx
description: Use jbx for Java development and automation when Java or Maven libraries can solve the task: run scripts, fetch dependencies, test, format, check, document, publish, and launch Java tools.
---

# `jbx`

Use `jbx` for Java development and automation when Java or Maven libraries can solve the task: run scripts, fetch dependencies, test, format, check, document, publish, and launch Java tools.

The top-level command stays intentionally thin. It gives humans a fast run path and gives agents a discovery path; detailed behavior lives in dedicated subcommands and version-matched skills.

## Start here

```bash
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
jbx skill list
jbx skill list --json
```

## Common thin entry-point workflows

```bash
jbx Hello.java --name Jay --verbose
jbx dev.telegraphic:hello-tool:1.0.0 --help
jbx dev.telegraphic:hello-tool:1.0.0 -- --literal-double-dash
jbx doctor --json
```

## Passing arguments

For the top-level shorthand, `jbx` options go before the Java file or Maven coordinate. After the target, arguments belong to the launched program, including options such as `--help` or Picocli flags.

Use an explicit `--` only when the launched program needs to receive a literal double-dash argument.

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
- [`init`](/docs/commands/init/) — Create Java sources from built-in or imported templates.
- [`export`](/docs/commands/export/) — Export local, portable, or native runnable artifacts.
- [`publish`](/docs/commands/publish/) — Publish Java projects to Maven repositories, including Maven Central.
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

## Development Loop

Before using any subcommand below for non-trivial work, discover and load its version-matched guidance first:

```sh
jbx skill list --json
```

1. For fresh Java work, use `jbx` with Java 25 that is provisioned automatically. List available templates with `jbx template list --json` for your task or choose `java` if none match:

```sh
jbx template list --json
jbx init -t java hello.java
```

2. Generate the implementation code. You can search for the unknown details and latest versions using `jbx search`. You can get structured documentation using `jbx docs`.

```sh
jbx search picocli --json
jbx docs info.picocli:picocli --type CommandLine --json
```

3. Format and check the code.

```sh
jbx fmt hello.java
jbx check hello.java --json
```

4. Fix the violations and repeat if required.

5. Write and execute tests.

```sh
jbx init -t test hello_test.java
# generate the tests
jbx test hello_test.java
```

6. Fix the failed tests and repeat if required.

7. Execute the script.

```sh
jbx run hello.java
```

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx`

```text
jbx: one-stop Java toolbox for scripts, tools, and agents

Usage: jbx [OPTIONS] [SCRIPT] [ARGS]... [COMMAND]

Commands:
  run       Compile and run a Java source file
  build     Compile and store script in the cache without running it
  publish   Prepare Maven Central publishing artifacts
  install   Install the current project into a Maven repository layout
  docs      Print agent-friendly documentation for source, directories, or Maven artifacts
  check     Check Java source files with javac diagnostics and Error Prone by default
  init      Initialize a Java script
  cache     Manage compiled script cache
  trust     Manage trusted remote scripts
  info      Print parsed JBang directives
  doctor    Diagnose the local jbx toolchain and a script when provided
  app       Manage scripts installed as commands on PATH
  alias     Manage aliases from jbang-catalog.json
  catalog   Manage external catalogs from jbang-catalog.json
  export    Export runnable JARs
  template  List init templates
  resolve   Resolve Maven dependencies without running
  fetch     Fetch Maven dependency artifacts and print classpath
  search    Search Maven Central for artifacts
  test      Run JUnit tests with the standalone console launcher
  fmt       Format Java source files with Palantir Java Format
  graph     Convert Java source to/from JavaParser's native JSON serialization
  rewrite   Run OpenRewrite recipes against Java source trees
  skill     Print version-matched agent skills bundled with this jbx release
  jdk       Manage installed JDKs
  help      Print this message or the help of the given subcommand(s)

Arguments:
  [SCRIPT]   Script to run, or Maven coordinates to launch as a Java tool
  [ARGS]...  Arguments passed to the script/tool when no subcommand is given

Options:
      --repo <REPOS>           Additional repository for Maven executable shorthand (id=url format or bare URL)
      --cache-dir <CACHE_DIR>  Override dependency cache directory for Maven executable shorthand
      --main <MAIN_CLASS>      Main class for Maven executable shorthand instead of java -jar
  -h, --help                   Print help
  -V, --version                Print version
```
