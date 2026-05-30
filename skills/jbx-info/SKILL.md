---
name: jbx-info
description: Print parsed directives and derived metadata from Java scripts.
---

# `info`

Print parsed directives and derived metadata from Java scripts.

## When to use it

- Read `//DEPS` before modifying a script.
- Check which Java version or main class a script declares.
- Extract docs/cache metadata for packaging or publishing workflows.

## Common workflows

```bash
jbx info deps Hello.java
jbx info repos Hello.java
jbx info java Hello.java
jbx info directives Hello.java
```

## Real-life examples

### Inspect directives before changing a script

```bash
jbx info directives Hello.java
jbx info deps Hello.java
jbx info java Hello.java
```

Use `info` to read what a script declares before editing it: dependencies, repositories, Java version, docs metadata, and runtime directives.

### Agent loop

1. Run `jbx info directives <file>` before modifying directives.
2. Query focused views such as `deps`, `repos`, or `java`.
3. Patch only the relevant directive lines.
4. Re-run `info` plus `jbx check --json` to verify the script still resolves.

## Agent notes

Directive order and comments can carry human meaning. Preserve nearby comments unless the change intentionally removes the directive.

## JSON and schema

`jbx info tools <script>` returns an IDE/tooling JSON snapshot for one Java source. Example output and field notes: [`info tools JSON`](/docs/schemas/#info-tools-json). Other `info` subcommands print focused text values.

Example JSON:

```json
{
  "originalResource": "/workspace/Hello.java",
  "backingResource": "/workspace/Hello.java",
  "applicationClassesDir": "/home/user/.cache/jbx/3149f0f2fec709ad/classes",
  "applicationJar": null,
  "mainClass": "Hello",
  "dependencies": [],
  "repositories": [],
  "resolvedDependencies": [],
  "javaVersion": null,
  "requestedJavaVersion": null,
  "compileOptions": [],
  "runtimeOptions": [],
  "nativeOptions": [],
  "javaAgents": [],
  "manifestOptions": [],
  "files": [],
  "sources": [],
  "description": null,
  "gav": null,
  "module": null,
  "docs": {},
  "enablePreview": false,
  "enableCds": false,
  "disableIntegrations": false
}
```

## Verification checklist

- The queried directive value matches the source file after edits.
- Added dependencies resolve with `jbx check --json` or `jbx build`.
- Repository and Java-version changes are intentional and visible in diff.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx info`

```text
Print parsed JBang directives

Usage: jbx info <COMMAND>

Commands:
  classpath        Print classpath used by the script
  tools            Print a json description for tools/IDEs
  docs             Print documentation references declared by the script
  cache            Print the effective jbx cache directory
  main             Print effective main class
  java             Print requested Java version
  description      Print script description
  gav              Print Maven GAV
  module           Print Java module name
  deps             Print dependency directives
  repos            Print repository directives
  sources          Print source directives
  files            Print file/resource directives
  compile-options  Print compile option directives
  runtime-options  Print runtime/java option directives
  native-options   Print native option directives
  javaagents       Print java agent directives
  manifest         Print manifest directives
  directives       Print parsed JBang directives
  help             Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx info classpath`

```text
Print classpath used by the script

Usage: jbx info classpath [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
      --deps-only
          Only include dependency/classpath entries, not compiled script classes
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --source <SOURCES>
          Additional source file, same shape as //SOURCES
      --files <FILES>
          Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option, same shape as //JAVA_OPTIONS
      --java <JAVA_VERSION>
          Override //JAVA requested version
      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>
          Override //MAIN / inferred class name
      --cache-dir <CACHE_DIR>
          Override cache directory
  -h, --help
          Print help
```

### `jbx info tools`

```text
Print a json description for tools/IDEs

Usage: jbx info tools [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
      --select <SELECT>
          Select a single field from the tools JSON payload
      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS
      --repo <REPOS>
          Additional repository, same shape as //REPOS
      --source <SOURCES>
          Additional source file, same shape as //SOURCES
      --files <FILES>
          Additional file/resource, same shape as //FILES
      --class-path <CLASSPATH>
          Additional classpath entries
      --javac-option <JAVAC_OPTIONS>
          Additional javac option
      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option, same shape as //JAVA_OPTIONS
      --java <JAVA_VERSION>
          Override //JAVA requested version
      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT
      --main <MAIN_CLASS>
          Override //MAIN / inferred class name
      --cache-dir <CACHE_DIR>
          Override cache directory
  -h, --help
          Print help
```

### `jbx info docs`

```text
Print documentation references declared by the script

Usage: jbx info docs <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info cache`

```text
Print the effective jbx cache directory

Usage: jbx info cache [OPTIONS]

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

### `jbx info main`

```text
Print effective main class

Usage: jbx info main <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info java`

```text
Print requested Java version

Usage: jbx info java <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info description`

```text
Print script description

Usage: jbx info description <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info gav`

```text
Print Maven GAV

Usage: jbx info gav <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info module`

```text
Print Java module name

Usage: jbx info module <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info deps`

```text
Print dependency directives

Usage: jbx info deps <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info repos`

```text
Print repository directives

Usage: jbx info repos <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info sources`

```text
Print source directives

Usage: jbx info sources <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info files`

```text
Print file/resource directives

Usage: jbx info files <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info compile-options`

```text
Print compile option directives

Usage: jbx info compile-options <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info runtime-options`

```text
Print runtime/java option directives

Usage: jbx info runtime-options <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info native-options`

```text
Print native option directives

Usage: jbx info native-options <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info javaagents`

```text
Print java agent directives

Usage: jbx info javaagents <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info manifest`

```text
Print manifest directives

Usage: jbx info manifest <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

### `jbx info directives`

```text
Print parsed JBang directives

Usage: jbx info directives <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
  -h, --help  Print help
```

> For exact behavior, prefer the skill bundled with the `jbx` binary on the machine running the task.
