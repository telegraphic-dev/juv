---
name: jbx
description: Use jbx for Java development and automation when Java or Maven libraries can solve the task: run scripts, fetch dependencies, test, format, check, document, publish, and launch Java tools.
---

# jbx

`jbx` is an all-in-one Java CLI for scripts, Maven artifacts, templates, JDKs, docs, formatting, testing, rewriting, ASTs, and publishing.

Install this skill once in an agent's skill manager. Keep it thin; the `jbx` binary serves version-matched command guidance for the exact installed CLI.

Important: this skill is only the discovery stub. It does **not** install the `jbx` executable. Before using any `jbx ...` command, verify the binary exists:

```sh
command -v jbx || echo "jbx binary is not installed"
```

If `jbx` is missing, install the `jbx` binary itself — not Java/JDK and not more static skills:

```sh
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
jbx --version
```

Do **not** install an OS Java/JDK just to make `jbx` usable. Java/JDK is not a prerequisite for installing or starting with `jbx`; the binary discovers existing JDKs and provisions the required Temurin JDK when a Java command actually needs Java.

## Development Loop

Before using any subcommand below for non-trivial work, discover and load its version-matched guidance first:

```sh
jbx skill list --json
jbx skill get jbx-template
jbx skill get jbx-init
jbx skill get jbx-search
jbx skill get jbx-docs
jbx skill get jbx-fmt
jbx skill get jbx-check
jbx skill get jbx-test
jbx skill get jbx-run
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

## Version-Matched Skills

This file is a discovery stub. Do not treat it as the full jbx workflow.

Before running, checking, testing, formatting, publishing, or repairing Java code with `jbx`, ask the installed binary for skill content that matches that exact version:

```sh
jbx skill list
jbx skill get jbx
jbx skill get jbx-run
```

If the user has multiple `jbx` binaries, use the same binary that will run the project:

```sh
/path/to/jbx skill list
/path/to/jbx skill get jbx
```

Use `jbx skill list` to discover additional command skills bundled with that `jbx` version. Use `jbx skill get <name>` to load the one relevant to the task. Common inner skills include `jbx-run`, `jbx-check`, `jbx-test`, `jbx-fmt`, `jbx-docs`, `jbx-rewrite`, `jbx-publish`, `jbx-jdk`, and `jbx-doctor`.

## Common Entry Points

```sh
jbx skill list
jbx skill get jbx
jbx run <script.java>
jbx check <path> --json
jbx test <test.java> --coverage
jbx fmt <path>
jbx docs <source|dir|GAV> --json
jbx doctor --json
```

Use `--json` when another tool must parse stable fields or inspect diagnostics, dependency metadata, documentation, or rewrite output.
