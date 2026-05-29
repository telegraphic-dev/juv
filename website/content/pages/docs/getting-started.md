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
jbx rewrite patch --recipe auto-format --source src/main/java
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
```

Inspect `rewrite/rewrite.patch`, then apply when the change is wanted:

```bash
jbx rewrite apply --recipe cleanup --source src/main/java
```

Discovery commands are machine-readable for agents:

```bash
jbx rewrite modules --search yaml --json
jbx rewrite recipes yaml --search format --json
```

## Documentation sidecars

`jbx docs` can read local sources or fetch published Markdown/JSON sidecars from Maven repositories:

```bash
jbx docs src/main/java
jbx docs com.fasterxml.jackson.core:jackson-databind
jbx docs com.fasterxml.jackson.core:jackson-databind --json
```
