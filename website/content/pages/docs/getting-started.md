---
title: Getting started with jbx
description: Basic command examples for jbx.
---

# Getting started

`jbx` accepts either a Java script path or a Maven coordinate.

Install it:

```bash
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
```

The installer downloads a native Linux or macOS binary. Rust is only needed when building `jbx` from source.

```bash
jbx run Hello.java world
jbx build Hello.java
jbx check src --json
jbx fmt src
jbx rewrite patch --recipe auto-format --source src/main/java --json
jbx test
jbx skill get
```

## Java scripts

Java 25 compact scripts are the default target. A minimal script can be as small as this:

```java
void main(String[] args) {
  IO.println("hello " + args[0]);
}
```

Run it:

```bash
jbx Hello.java world
```

## Maven tools

Executable Maven artifacts can be invoked directly:

```bash
jbx dev.telegraphic:hello-tool:1.0.0 -- --help
```

Use `--main` when an artifact exposes more than one entry point.

## OpenRewrite recipes

`jbx rewrite` runs OpenRewrite recipes through jbx-managed dependencies and JDKs. Start with a patch preview:

```bash
jbx rewrite patch --recipe auto-format --source src/main/java
jbx rewrite patch --module yaml --recipe org.openrewrite.yaml.format.AutoFormat --source config
jbx rewrite patch --recipe change-package --option old=com.old --option new=com.new --source src --report rewrite --json
```

Inspect `rewrite/rewrite.patch`, then apply when the change is wanted:

```bash
jbx rewrite apply --recipe cleanup --source src/main/java
```

For CI-style checks, add `--fail-on-changes`. For custom recipe modules, add `--repo id=url`, `--cache-dir dir`, or `--rewrite-version version` when you need a specific repository, isolated cache, or OpenRewrite release.

Discovery commands are machine-readable for agents:

```bash
jbx rewrite modules --search yaml --group org.openrewrite.recipe --rewrite-version 8.60.0 --json
jbx rewrite recipes yaml --search format --detail --json
```

## Publishing metadata

`jbx publish --file jbx.json --dry-run` stages Maven-ready artifacts before any real release. Keep the descriptor lean: use `dependencies` (or `//DEPS` in a script) for libraries needed to compile the artifact, and `runtimeDependencies` (or `//RUNTIME`) for runtime-only implementations such as logging backends or recipe helper providers.

```json
{
  "main": "src/main/java/dev/telegraphic/demo/HelloTool.java",
  "group": "dev.telegraphic.demo",
  "id": "hello-tool",
  "version": "1.0.0",
  "dependencies": ["info.picocli:picocli:4.7.7"],
  "runtimeDependencies": ["org.slf4j:slf4j-nop:2.0.17"]
}
```

Runtime dependencies are emitted to Maven metadata with `runtime` scope and are not required on the compile classpath.

## Documentation sidecars

`jbx docs` can read local sources or fetch published Markdown/JSON sidecars from Maven repositories:

```bash
jbx docs src/main/java
jbx docs com.fasterxml.jackson.core:jackson-databind
jbx docs com.fasterxml.jackson.core:jackson-databind --json
```
