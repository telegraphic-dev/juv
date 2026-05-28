# jbx docs JSON schema

`jbx docs --json` sidecars use an artifact-oriented schema for agents and tools. Markdown remains the primary reading format; JSON is for structured lookup.

## Source of truth

Do not invent guidance in generated output.

Every document should state where each part came from:

```json
{
  "generatedFrom": {
    "source": "javadoc",
    "jbxVersion": "0.0.0"
  }
}
```

Valid `generatedFrom.source` values include:

- `javadoc` — converted from Javadoc/class documentation.
- `jbx-directives` — generated from `//DESCRIPTION`, `//DOCS`, and related JBang-style directives.
- `manual` — written by a maintainer.
- `mixed` — combines multiple documented sources; entries should carry their own source metadata where practical.

Agent guidance is only valid when it is explicitly present in the source material, for example a `//DOCS agent-note=...` directive or a maintainer-written sidecar section. Generated docs must not hallucinate best practices from library reputation.

## Naming

Use `types`, not `symbols`. `symbols` is too broad for artifact docs; a library sidecar primarily documents public API types and their members.

Qualified names are the default:

- `qualifiedName` is required for packages, types, methods, fields, constructors, annotations, and enum constants when the name can be qualified.
- Short names may appear as `name` for display.
- Type references should be qualified unless the referenced type is in the same package or `java.lang`.

## Recommended top-level shape

```json
{
  "schema": "https://telegraphic.dev/schemas/jbx-docs/v1.json",
  "artifact": {
    "group": "com.fasterxml.jackson.core",
    "id": "jackson-databind",
    "version": "2.17.2",
    "coordinate": "com.fasterxml.jackson.core:jackson-databind:2.17.2"
  },
  "summary": "JSON object mapping library for Java.",
  "packages": [
    {
      "name": "com.fasterxml.jackson.databind",
      "summary": "Data-binding entry points."
    }
  ],
  "types": [
    {
      "kind": "class",
      "name": "ObjectMapper",
      "qualifiedName": "com.fasterxml.jackson.databind.ObjectMapper",
      "package": "com.fasterxml.jackson.databind",
      "visibility": "public",
      "modifiers": ["public"],
      "annotations": [],
      "extends": "java.lang.Object",
      "implements": [],
      "summary": "Main JSON reader/writer and data-binding entry point.",
      "documentation": "Markdown converted from Javadoc.",
      "constructors": [
        {
          "name": "ObjectMapper",
          "qualifiedName": "com.fasterxml.jackson.databind.ObjectMapper.ObjectMapper",
          "visibility": "public",
          "modifiers": ["public"],
          "annotations": [],
          "parameters": [],
          "documentation": "Default constructor documentation."
        }
      ],
      "methods": [
        {
          "name": "readValue",
          "qualifiedName": "com.fasterxml.jackson.databind.ObjectMapper.readValue",
          "visibility": "public",
          "modifiers": ["public"],
          "annotations": [],
          "typeParameters": ["T"],
          "parameters": [
            { "name": "content", "type": "java.lang.String" },
            { "name": "valueType", "type": "java.lang.Class<T>" }
          ],
          "returnType": "T",
          "throws": ["java.io.IOException"],
          "summary": "Deserialize JSON content into a Java value.",
          "documentation": "Markdown converted from Javadoc."
        }
      ],
      "fields": [
        {
          "name": "DEFAULT_ANNOTATION_INTROSPECTOR",
          "qualifiedName": "com.fasterxml.jackson.databind.ObjectMapper.DEFAULT_ANNOTATION_INTROSPECTOR",
          "visibility": "protected",
          "modifiers": ["protected", "static", "final"],
          "annotations": [],
          "type": "com.fasterxml.jackson.databind.AnnotationIntrospector",
          "summary": "Field summary from Javadoc."
        }
      ]
    }
  ],
  "dependencies": [],
  "generatedFrom": {
    "source": "javadoc",
    "jbxVersion": "0.0.0"
  }
}
```

## Member rules

- `visibility`: one of `public`, `protected`, `package`, `private`.
- `modifiers`: Java modifiers as separate strings, for example `static`, `final`, `abstract`, `default`, `sealed`.
- `annotations`: structured objects when parsed, not flattened strings when possible:

```json
{
  "qualifiedName": "java.lang.Deprecated",
  "values": { "since": "2.0", "forRemoval": false }
}
```

- `parameters`: preserve source order and include names when available from Javadoc/source.
- `returnType`, `type`, `throws`, `extends`, and `implements` should use qualified names following the naming rule above.
- Keep raw signature strings out of the primary schema. They may be included as optional display-only `declaration` fields, but tools should rely on structured fields.

## Current generator status

The initial `jbx docs` implementation can:

- generate local docs from JBang-style source directives;
- fetch and cache remote sidecars;
- publish sidecar artifacts.

It does not yet convert arbitrary Javadoc/JAR APIs into full `types`/member JSON. That should be a separate generator step, because doing it correctly needs real Java API extraction rather than regex over source text.
