---
title: jbx.json descriptor
description: Schema and field reference for jbx project descriptors used by publish and install.
---

# `jbx.json`

`jbx.json` is the project descriptor for publishing or installing a Java artifact with `jbx`. Keep it small: put durable artifact metadata here, and keep command-specific experiments on the command line.

JSON Schema: [`/schemas/jbx-json/v1.json`](/schemas/jbx-json/v1.json)

## Minimal descriptor

Use this when you only need local install/serve workflows or when Git metadata can fill in Maven Central metadata later:

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

Try it locally before publishing:

```bash
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --serve 0
jbx install --file jbx.json --destination build/local-m2
```

## Maven Central-ready descriptor

Real publishing needs normal Maven metadata: project URL, license, developer, and SCM. `jbx` can infer some values from GitHub, but explicit metadata is better for repeatable releases.

```json
{
  "$schema": "https://jbx.telegraphic.dev/schemas/jbx-json/v1.json",
  "main": "dev.telegraphic.demo.HelloTool",
  "group": "dev.telegraphic.demo",
  "id": "hello-tool",
  "version": "1.0.0",
  "name": "Hello Tool",
  "description": "Small CLI published with jbx.",
  "url": "https://github.com/telegraphic-dev/hello-tool",
  "licenses": [
    {
      "name": "Apache-2.0",
      "url": "https://www.apache.org/licenses/LICENSE-2.0.txt"
    }
  ],
  "developers": [
    {
      "name": "Telegraphic",
      "organization": "Telegraphic",
      "organizationUrl": "https://telegraphic.dev"
    }
  ],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/hello-tool.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/hello-tool.git",
    "url": "https://github.com/telegraphic-dev/hello-tool"
  },
  "sources": ["src/main/java/dev/telegraphic/demo/HelloTool.java"],
  "repositories": ["snapshots=https://repo.example.test/maven"]
}
```

Run a dry run first. Real publishing still needs Central Portal and signing credentials configured in the environment or command options.

```bash
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --publish
```

## Fields

- `$schema` — optional editor hint. Use `https://jbx.telegraphic.dev/schemas/jbx-json/v1.json`. `jbx` ignores unknown fields, so this is safe.
- `main` — main Java source. Relative paths resolve from the descriptor directory. Existing paths win; otherwise `jbx` also checks `.java`, `.jsh`, `.jav`, and Java FQN matches under the descriptor directory.
- `group` — Maven `groupId`. Required for publish/install unless `//GAV` supplies it.
- `id` — Maven `artifactId`. Required for publish/install unless `//GAV` supplies it.
- `version` — artifact version. Maven Central publishing rejects `-SNAPSHOT`. `--version` can override it for one command.
- `package` — Java package override used when staging compact or unpackaged sources for publishing.
- `name` — human-readable POM name. Defaults to `group:id`.
- `description` — POM description. Falls back to `//DESCRIPTION` or a generated description.
- `url` — project URL. Required for Maven Central dry-run/publish.
- `licenses` — license objects with required `name` and `url`. Required for Maven Central dry-run/publish.
- `developers` — developer objects with required `name` and optional `email`, `organization`, `organizationUrl`. Required for Maven Central dry-run/publish.
- `scm` — SCM object with required `connection` and `url`, optional `developerConnection`. Required for Maven Central dry-run/publish.
- `java` — requested Java version, equivalent to `//JAVA`.
- `dependencies` — compile-time/public Maven coordinates, equivalent to `//DEPS`. Rendered as normal Maven dependencies.
- `runtimeDependencies` — runtime-only Maven coordinates, equivalent to `//RUNTIME`. Rendered with Maven `runtime` scope and not required on the compile classpath.
- `sources` — additional source files to include. Omit the field to let `jbx` auto-discover local Java sources; set it explicitly when you need a controlled source list.
- `repositories` — extra Maven repositories for resolution. Entries can be bare URLs or `id=url`.

## Real-life patterns

### Publish a small CLI with Picocli

```json
{
  "$schema": "https://jbx.telegraphic.dev/schemas/jbx-json/v1.json",
  "main": "src/main/java/acme/tools/CleanImports.java",
  "group": "com.acme.tools",
  "id": "clean-imports",
  "version": "0.3.0",
  "dependencies": ["info.picocli:picocli:4.7.7"]
}
```

```bash
jbx publish --file jbx.json --dry-run
```

### Keep parser/logging providers runtime-only

Use `runtimeDependencies` for implementation libraries needed when the tool runs but not when callers compile against the artifact:

```json
{
  "$schema": "https://jbx.telegraphic.dev/schemas/jbx-json/v1.json",
  "main": "dev.acme.RewriteTool",
  "group": "dev.acme",
  "id": "rewrite-tool",
  "version": "1.2.0",
  "dependencies": ["org.openrewrite:rewrite-java:8.56.1"],
  "runtimeDependencies": [
    "org.openrewrite:rewrite-java-21:8.56.1",
    "org.slf4j:slf4j-nop:2.0.17"
  ]
}
```

### Pin the published source set

If the repository has helpers, examples, or generated files you do not want in the artifact, set `sources` explicitly:

```json
{
  "$schema": "https://jbx.telegraphic.dev/schemas/jbx-json/v1.json",
  "main": "src/main/java/dev/acme/App.java",
  "group": "dev.acme",
  "id": "app",
  "version": "1.0.0",
  "sources": [
    "src/main/java/dev/acme/App.java",
    "src/main/java/dev/acme/AppSupport.java"
  ]
}
```

## Descriptor and directives

Descriptor fields override or replace matching source directives where both exist:

- `group`, `id`, `version` replace `//GAV`.
- `description` replaces `//DESCRIPTION`.
- `java` replaces `//JAVA`.
- `dependencies` replaces `//DEPS` when non-empty.
- `runtimeDependencies` replaces `//RUNTIME` when non-empty.
- `sources` replaces `//SOURCES` when non-empty.
- `repositories` replaces `//REPOS` when non-empty.

That keeps a library release reproducible without forcing all metadata into the Java file.

## Verification checklist

- `main` resolves from the descriptor directory.
- `group`, `id`, and `version` produce the intended Maven coordinate.
- Compile-time libraries are in `dependencies`; runtime-only providers are in `runtimeDependencies`.
- `jbx publish --file jbx.json --dry-run` succeeds before any real publish.
- For local-only checks, `jbx install --file jbx.json --destination build/local-m2` writes the expected Maven layout.
