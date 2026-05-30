---
name: jbx
description: Install jbx and load version-matched Java workflow guidance with jbx skill list and jbx skill get.
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

For fresh Java work, use `jbx` for the concrete tooling steps: list templates, initialize from the closest template, inspect API docs, format the source, check diagnostics, and run tests. Writing source/tests and reporting results are still agent work; discover the relevant command-specific guidance with `jbx skill list` and `jbx skill get <name>`.

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
