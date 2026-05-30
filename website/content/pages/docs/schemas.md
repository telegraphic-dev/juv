---
title: jbx JSON schemas
description: Stable JSON output shapes for agent-facing jbx commands.
---

# JSON schemas

Commands and subcommands documented here emit JSON that agents can parse directly. Commands with no JSON output do not have a schema section; use their exit code, filesystem effects, or a follow-up JSON-capable command instead.

Descriptor schemas live here too, so editors and agents can discover the same public contracts the website documents.

## jbx.json descriptor schema

`jbx.json` describes publish/install metadata: main source, Maven coordinates, dependency buckets, source set, repositories, and Maven Central metadata. Documentation page: [`jbx.json descriptor`](/docs/jbx-json/). JSON Schema: [`/schemas/jbx-json/v1.json`](/schemas/jbx-json/v1.json).

```json
{
  "$schema": "https://jbx.telegraphic.dev/schemas/jbx-json/v1.json",
  "main": "src/main/java/dev/telegraphic/demo/HelloTool.java",
  "group": "dev.telegraphic.demo",
  "id": "hello-tool",
  "version": "1.0.0",
  "java": "25",
  "dependencies": ["info.picocli:picocli:4.7.7"],
  "runtimeDependencies": ["org.slf4j:slf4j-nop:2.0.17"]
}
```

## check JSON

`jbx check --json` returns compile status, diagnostics, and compiler output.

```json
{
  "ok": true,
  "diagnostics": [],
  "compilerOutput": ""
}
```

Diagnostics include source path, line, column, severity, tool, code when available, and message. Empty diagnostics with `ok: true` means the source passed the configured javac/Error Prone gate.

## test JSON

`jbx test --json` returns JUnit execution counts and test cases. With `--coverage`, it also includes a `coverage` object containing JaCoCo exec/html/xml paths and aggregate counters.

```json
{
  "tests": 1,
  "failures": 0,
  "errors": 0,
  "skipped": 0,
  "testCases": [
    {
      "className": "CalculatorTest",
      "name": "adds()",
      "time": "0.025",
      "status": "passed"
    }
  ]
}
```

## docs JSON

`jbx docs --json` follows the published sidecar schema: package/module metadata, documented types, methods/fields/constructors, signatures, descriptions, examples, and source provenance. Full schema: [`jbx-docs-schema.md`](/docs/jbx-docs-schema/). Download a complete small sample: [`jbx-docs-local.json`](/samples/jbx-docs-local.json).

```json
{
  "schema": "https://jbx.telegraphic.dev/schemas/jbx-docs/v1.json",
  "target": "Hello.java",
  "sources": [
    {
      "path": "Hello.java",
      "name": "Hello.java",
      "description": "Prints a greeting.",
      "docs": [],
      "dependencies": []
    }
  ],
  "types": [],
  "generatedFrom": {
    "source": "jbx-directives",
    "jbxVersion": "0.0.0"
  }
}
```

## doctor JSON

`jbx doctor --json` returns overall status, optional target, and named environment checks. Check status values are `ok`, `warn`, `fail`, or `skipped`.

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

Agents should fail closed on `fail`, surface `warn`, and ignore `skipped` unless the skipped capability was explicitly requested.

## rewrite JSON

`jbx rewrite modules --json`, `jbx rewrite recipes --json`, `jbx rewrite patch --json`, and `jbx rewrite apply --json` return discovery or change reports for OpenRewrite workflows.

```json
[
  {
    "short": "spring",
    "coordinate": "org.openrewrite.recipe:rewrite-spring:6.9.0",
    "groupId": "org.openrewrite.recipe",
    "artifactId": "rewrite-spring",
    "version": "6.9.0",
    "versionCount": 123
  }
]
```

Patch/apply reports include requested recipes/modules, scanned sources, patch/report paths, whether changes were produced/applied, and process status.

## search JSON

`jbx search --json` returns the normalized query, Maven Central hit count, and artifact records.

```json
{
  "query": "g:org.junit.platform AND a:junit-platform-console-standalone",
  "numFound": 1,
  "artifacts": [
    {
      "coordinate": "org.junit.platform:junit-platform-console-standalone:1.13.0-M3",
      "groupId": "org.junit.platform",
      "artifactId": "junit-platform-console-standalone",
      "version": "1.13.0-M3",
      "packaging": "jar",
      "repositoryId": "central"
    }
  ]
}
```

Artifact records can also include latest version, version count, timestamp, classifiers, and raw Maven Central fields.

## cache JSON

`jbx cache list --json` returns compiled-script cache entries.

```json
[
  {
    "script": "/workspace/Hello.java",
    "classesDir": "/workspace/.jbx-cache/e2db3fcb2d67e4ce/classes",
    "cacheDir": "/workspace/.jbx-cache/e2db3fcb2d67e4ce"
  }
]
```

## catalog JSON

`jbx catalog list --json` returns catalog records from `jbang-catalog.json`.

```json
[
  {
    "name": "team",
    "catalogRef": "https://example.com/jbang-catalog.json",
    "catalog": "https://example.com/jbang-catalog.json",
    "description": null,
    "import": false
  }
]
```

## alias JSON

`jbx alias list --json` returns alias records from `jbang-catalog.json`.

```json
[
  {
    "name": "hello",
    "scriptRef": "Hello.java",
    "script": "/workspace/Hello.java",
    "description": "Run the greeting script",
    "arguments": [],
    "dependencies": [],
    "repositories": [],
    "sources": [],
    "files": [],
    "classpaths": [],
    "compileOptions": [],
    "runtimeOptions": [],
    "javaAgents": [],
    "javaVersion": null,
    "mainClass": null
  }
]
```

## template JSON

`jbx template list --json` returns built-in and imported templates for `jbx init`.

```json
[
  {
    "name": "compact",
    "description": "Java 25 compact-source Hello World script"
  },
  {
    "name": "test",
    "description": "JUnit test class"
  }
]
```

## info tools JSON

`jbx info tools <script>` returns an IDE/tooling snapshot for one Java source.

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

## graph JSON

`jbx graph dump <script>` emits JavaParser's native AST JSON serialization. It can be large even for small files, so avoid pasting it into prompts unless you need the tree. Download a sample: [`jbx-graph-dump.json`](/samples/jbx-graph-dump.json).

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

Nodes include a `!` discriminator with the JavaParser class name, source `range`, `tokenRange`, and node-specific fields such as imports, types, members, expressions, and statements. This is a structural AST exchange format, not a simplified jbx schema.

## skill JSON

`jbx skill list --json` returns bundled skill names and descriptions.

```json
{
  "skills": [
    {
      "name": "jbx-check",
      "description": "Check Java source with structured diagnostics."
    }
  ]
}
```

`jbx skill get <name>` returns Markdown skill content with YAML frontmatter containing `name` and `description`, followed by command-specific guidance generated from the matching website command page.
