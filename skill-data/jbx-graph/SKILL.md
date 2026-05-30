---
name: jbx-graph
description: Dump JavaParser native AST JSON or import it back to Java source.
---

# `graph`

Dump JavaParser native AST JSON or import it back to Java source.

## When to use it

- Let an agent perform structural analysis without regexing Java.
- Round-trip a generated AST back to source after a controlled transformation.
- Debug parser behavior around compact Java scripts.

## Common workflows

```bash
jbx graph dump Hello.java > Hello.ast.json
jbx graph import Hello.ast.json --output Hello.java
jbx graph dump src/main/java/com/acme/App.java | jq .
```

## Real-life examples

### Inspect Java structurally before a precise edit

```bash
jbx graph dump src/main/java/com/acme/App.java > App.ast.json
jq '.types // .' App.ast.json
```

Use `graph` when regex would be reckless: compact scripts, nested classes, imports, annotations, or literal edits.

### Agent loop

1. Dump the AST for the exact file being edited.
2. Locate the structural node and confirm the current value.
3. Make the source or AST change with stale-context checks.
4. Re-run `jbx graph dump` and `jbx check --json`.

## Agent notes

Prefer source edits unless AST import is explicitly needed. If importing JSON, preserve parser metadata and verify formatting afterward.

## JSON and schema

`jbx graph dump <script>` emits JavaParser-native AST JSON. Output can be large; see the compact example and downloadable sample in [`graph JSON`](/docs/schemas/#graph-json).

Example JSON:

```json
{
  "!": "com.github.javaparser.ast.CompilationUnit",
  "range": {
    "beginLine": 1,
    "beginColumn": 1,
    "endLine": 3,
    "endColumn": 2
  },
  "imports": [],
  "types": [
    {
      "!": "com.github.javaparser.ast.body.ClassOrInterfaceDeclaration",
      "isCompact": "true",
      "members": [
        {
          "!": "com.github.javaparser.ast.body.MethodDeclaration",
          "name": { "identifier": "main" }
        }
      ]
    }
  ]
}
```

## Verification checklist

- Dump output parses as JSON.
- Imported source compiles with `jbx check --json`.
- `git diff` shows the intended structural change, not a full-file rewrite surprise.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx graph`

```text
Convert Java source to/from JavaParser's native JSON serialization

Usage: jbx graph <COMMAND>

Commands:
  dump    Convert a Java source file to JavaParser's native JSON serialization
  import  Convert JavaParser's native JSON serialization back to Java source
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx graph dump`

```text
Convert a Java source file to JavaParser's native JSON serialization

Usage: jbx graph dump [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Java source file

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

### `jbx graph import`

```text
Convert JavaParser's native JSON serialization back to Java source

Usage: jbx graph import [OPTIONS] <JSON>

Arguments:
  <JSON>  JavaParser JSON file

Options:
  -o, --output <OUTPUT>        Write Java source to this file instead of stdout
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```
