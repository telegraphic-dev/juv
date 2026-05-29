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

## Documentation sidecars

`jbx docs` can read local sources or fetch published Markdown/JSON sidecars from Maven repositories:

```bash
jbx docs src/main/java
jbx docs com.fasterxml.jackson.core:jackson-databind
jbx docs com.fasterxml.jackson.core:jackson-databind --json
```
