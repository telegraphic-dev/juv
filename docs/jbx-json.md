# jbx.json descriptor

`jbx.json` is the project descriptor for publishing or installing a Java artifact with `jbx`. Keep it small: put durable artifact metadata here, and keep command-specific experiments on the command line.

JSON Schema: `https://jbx.telegraphic.dev/schemas/jbx-json/v1.json`

## Minimal descriptor

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

```bash
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --serve 0
jbx install --file jbx.json --destination build/local-m2
```

## Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `$schema` | string | Optional editor hint. Use `https://jbx.telegraphic.dev/schemas/jbx-json/v1.json`. |
| `main` | string | Main Java source. Relative paths resolve from the descriptor directory. Existing paths win; otherwise `jbx` checks `.java`, `.jsh`, `.jav`, and Java FQN matches. |
| `group` | string | Maven `groupId`. Required for publish/install unless `//GAV` supplies it. |
| `id` | string | Maven `artifactId`. Required for publish/install unless `//GAV` supplies it. |
| `version` | string | Artifact version. Maven Central publishing rejects `-SNAPSHOT`. |
| `package` | string | Java package override used when staging compact or unpackaged sources. |
| `name` | string | Human-readable POM name. Defaults to `group:id`. |
| `description` | string | POM description. Falls back to `//DESCRIPTION` or a generated description. |
| `url` | string | Project URL. Required for Maven Central dry-run/publish. |
| `licenses` | array | License objects with required `name` and `url`. Required for Maven Central dry-run/publish. |
| `developers` | array | Developer objects with required `name` and optional `email`, `organization`, `organizationUrl`. Required for Maven Central dry-run/publish. |
| `scm` | object | SCM object with required `connection` and `url`, optional `developerConnection`. Required for Maven Central dry-run/publish. |
| `java` | string | Requested Java version, equivalent to `//JAVA`. |
| `dependencies` | string array | Compile-time/public Maven coordinates, equivalent to `//DEPS`. |
| `runtimeDependencies` | string array | Runtime-only Maven coordinates, equivalent to `//RUNTIME`; rendered with Maven `runtime` scope. |
| `sources` | string array | Additional source files. Omit to let `jbx` auto-discover local Java sources. |
| `repositories` | string array | Extra Maven repositories as bare URLs or `id=url`. |

## Full example

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
    { "name": "Apache-2.0", "url": "https://www.apache.org/licenses/LICENSE-2.0.txt" }
  ],
  "developers": [
    { "name": "Telegraphic", "organization": "Telegraphic", "organizationUrl": "https://telegraphic.dev" }
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
