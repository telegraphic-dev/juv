---

title: jbx search command
description: Search Maven Central artifacts by text or coordinates.
---

# `search`

Search Maven Central artifacts by text or coordinates, with filters.

## When to use it

- Find the current coordinate for a library before adding `//DEPS`.
- Resolve ambiguous artifact names when a README only gives a product name.
- Let an agent rank candidate dependencies without scraping Maven Central HTML.

## Common workflows

```bash
jbx search picocli --json
jbx search --group org.junit.platform --id junit-platform-console-standalone --json
jbx search --group com.fasterxml.jackson.core --id jackson-databind --version 2.17.2 --json
```

## Real-life examples

### Find the exact coordinate before adding a dependency

```bash
jbx search --group org.junit.platform --id junit-platform-console-standalone --json
jbx search --group com.fasterxml.jackson.core --id jackson-databind --version 2.17.2 --json
```

Use `search` when you know a library name but need the Maven coordinate, latest version, or exact artifactId.

### Agent loop

1. Search with `--json` and the narrowest known filters.
2. Prefer exact group/artifact matches over popularity guesses.
3. Copy the full coordinate into `//DEPS` or `jbx resolve`.
4. Resolve or compile before treating the dependency choice as correct.

## Agent notes

Maven Central search is discovery, not endorsement. Do not pick a dependency only because it appears first; check group, artifactId, version, and project legitimacy.

## JSON and schema

`jbx search --json` returns the query, total count, and normalized artifact candidates. Example output and field notes: [`search JSON`](/docs/schemas/#search-json).

Example JSON:

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


## Verification checklist

- Result includes the expected groupId and artifactId.
- Version is explicit when adding a dependency.
- Follow-up `jbx resolve <gav>` or `jbx check --json` succeeds.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx search`

```text
Search Maven Central for artifacts

Usage: jbx search [OPTIONS] [QUERY]...

Arguments:
  [QUERY]...  Search text, Solr query, or Maven coordinate (group:artifact[:version])

Options:
      --group <GROUP>      Solr groupId filter (maps to g:<group>)
      --id <ID>            Solr artifactId filter (maps to a:<id>)
      --version <VERSION>  Solr version filter (maps to v:<version> and searches the gav core)
  -n, --limit <LIMIT>      Maximum number of results to return [default: 20]
      --json               Return structured JSON for agent/tool consumption
  -h, --help               Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-search
```
