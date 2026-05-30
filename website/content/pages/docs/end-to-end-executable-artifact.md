---
title: End-to-end executable artifact guide
description: Build a Java CLI with the jbx development loop, publish it to Maven Central, and run it back from Maven coordinates.
---

# End-to-end executable artifact guide

This walkthrough takes one small Java CLI from source to `jbx group:artifact:version`. The example is deliberately mundane: a word-count tool with options, file input, standard input, useful exit codes, and one real dependency. That is representative enough to exercise the parts that matter without turning the guide into a framework cosplay convention.

Use your own Maven namespace below. `dev.acme.tools` is a placeholder.

## 1. Create the script project

```bash
mkdir word-stats
cd word-stats
mkdir -p src/main/java/dev/acme/tools
```

Create `src/main/java/dev/acme/tools/WordStats.java`:

```java
//JAVA 21
//DEPS info.picocli:picocli:4.7.7
package dev.acme.tools;

import java.io.BufferedReader;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.Callable;
import picocli.CommandLine;
import picocli.CommandLine.Command;
import picocli.CommandLine.Option;
import picocli.CommandLine.Parameters;

/** Counts lines, words, and characters for command-line use. */
@Command(
    name = "word-stats",
    mixinStandardHelpOptions = true,
    version = "word-stats 1.0.0",
    description = "Counts lines, words, and characters in text files or stdin.")
public final class WordStats implements Callable<Integer> {
  /** Creates a word-stats command. */
  public WordStats() {}

  @Option(names = "--min-length", description = "Only count words at least this long.")
  int minLength = 1;

  @Option(names = "--json", description = "Print one JSON object instead of text.")
  boolean json;

  @Parameters(arity = "0..*", paramLabel = "FILE", description = "Files to read. Omit for stdin.")
  List<Path> files = new ArrayList<>();

  /**
   * Runs the command.
   *
   * @param args command-line arguments
   */
  public static void main(String[] args) {
    int exit = new CommandLine(new WordStats()).execute(args);
    System.exit(exit);
  }

  @Override
  public Integer call() throws Exception {
    Counts total = new Counts();
    if (files.isEmpty()) {
      total.add(read(new BufferedReader(new java.io.InputStreamReader(System.in, StandardCharsets.UTF_8))));
    } else {
      for (Path file : files) {
        try (BufferedReader reader = Files.newBufferedReader(file)) {
          total.add(read(reader));
        }
      }
    }

    if (json) {
      System.out.printf("{\"lines\":%d,\"words\":%d,\"characters\":%d}%n",
          total.lines, total.words, total.characters);
    } else {
      System.out.printf("lines=%d words=%d characters=%d%n",
          total.lines, total.words, total.characters);
    }
    return 0;
  }

  private Counts read(BufferedReader reader) throws IOException {
    Counts counts = new Counts();
    String line;
    while ((line = reader.readLine()) != null) {
      counts.lines++;
      counts.characters += line.length();
      counts.words += countWords(line);
    }
    return counts;
  }

  private long countWords(String line) {
    long count = 0;
    int length = 0;
    for (int i = 0; i < line.length(); i++) {
      if (Character.isLetterOrDigit(line.charAt(i)) || line.charAt(i) == '_') {
        length++;
      } else {
        if (length >= minLength) {
          count++;
        }
        length = 0;
      }
    }
    if (length >= minLength) {
      count++;
    }
    return count;
  }

  static final class Counts {
    long lines;
    long words;
    long characters;

    void add(Counts other) {
      lines += other.lines;
      words += other.words;
      characters += other.characters;
    }
  }
}
```

Create `jbx.json` next to it:

```json
{
  "$schema": "https://jbx.telegraphic.dev/schemas/jbx-json/v1.json",
  "main": "src/main/java/dev/acme/tools/WordStats.java",
  "group": "dev.acme.tools",
  "id": "word-stats",
  "version": "1.0.0",
  "name": "word-stats",
  "description": "Small text statistics CLI published with jbx.",
  "url": "https://github.com/acme/word-stats",
  "licenses": [
    { "name": "Apache-2.0", "url": "https://www.apache.org/licenses/LICENSE-2.0.txt" }
  ],
  "developers": [
    { "name": "Acme", "organization": "Acme", "organizationUrl": "https://github.com/acme" }
  ],
  "scm": {
    "connection": "scm:git:https://github.com/acme/word-stats.git",
    "developerConnection": "scm:git:ssh://git@github.com/acme/word-stats.git",
    "url": "https://github.com/acme/word-stats"
  },
  "java": "21",
  "dependencies": ["info.picocli:picocli:4.7.7"]
}
```

## 2. Use the development loop

Start with the fast checks:

```bash
jbx check src/main/java
jbx build src/main/java/dev/acme/tools/WordStats.java
jbx fmt src/main/java
```

Run the tool locally while editing:

```bash
printf 'one two three\nsmall words are useful\n' > sample.txt
jbx src/main/java/dev/acme/tools/WordStats.java sample.txt
jbx src/main/java/dev/acme/tools/WordStats.java --min-length 5 --json sample.txt
```

Expected shape:

```text
lines=2 words=7 characters=35
{"lines":2,"words":4,"characters":35}
```

Before publishing, verify the Maven bundle without touching Maven Central:

```bash
jbx publish --file jbx.json --dry-run --skip-signing --output target/central-bundle.zip
```

Inspect the ZIP if anything looks odd:

```bash
unzip -l target/central-bundle.zip
```

For a local end-to-end rehearsal, serve the artifact from a temporary Maven repository:

```bash
jbx publish --file jbx.json --serve 0 --skip-signing
```

`--serve 0` prints a loopback repository URL. In another terminal, use that URL to run the tool from coordinates:

```bash
jbx --repo local="http://127.0.0.1:<printed-port>/" \
  dev.acme.tools:word-stats:1.0.0 \
  --main dev.acme.tools.WordStats \
  -- sample.txt
```

Stop the server with `Ctrl-C` when the rehearsal is done.

That proves the important path before a public release: source compiles, metadata is usable, the artifact is laid out like Maven expects, and `jbx` can execute it by coordinates.

## 3. Publish to Maven Central

Do real publishing from CI or a release workflow, not from random laptops. The safe split is:

- PR CI: `jbx publish --dry-run --skip-signing`
- Release workflow: import the GPG key, set Central Portal credentials, then run `jbx publish --publish`

A minimal release command looks like this:

```bash
export CENTRAL_TOKEN_USERNAME='...'
export CENTRAL_TOKEN_PASSWORD='...'
export GPG_KEY_ID='...'

jbx publish \
  --file jbx.json \
  --version 1.0.0 \
  --gpg-key "$GPG_KEY_ID" \
  --output target/central-bundle.zip \
  --target-dir target/publish \
  --cache-dir .jbx-cache \
  --publish
```

`--publish` uploads the signed Central bundle through the Maven Central Portal API and waits until the deployment is `PUBLISHED` or `FAILED`. Keep credentials in environment variables or CI secrets; do not put them in `jbx.json`.

## 4. Run the published tool from Maven coordinates

Once Central has indexed the artifact, anyone can run it directly:

```bash
jbx dev.acme.tools:word-stats:1.0.0 --main dev.acme.tools.WordStats -- --help
jbx dev.acme.tools:word-stats:1.0.0 --main dev.acme.tools.WordStats -- sample.txt
jbx dev.acme.tools:word-stats:1.0.0 --main dev.acme.tools.WordStats -- --min-length 5 --json sample.txt
```

`--main` names the entry point inside the artifact. If a future artifact is packaged as an executable JAR with a `Main-Class` manifest, it can omit that flag.

For users, this is the payoff: no generated project, no wrapper script, no manual classpath. The Maven coordinate is the executable handle.
