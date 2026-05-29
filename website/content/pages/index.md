---
title: jbx — Java tools for agents
description: Rust-native, JBang-compatible Java tooling for autonomous agents and impatient humans.
---

# Java tools for agents.

`jbx` is a Rust-native Java toolbox aiming for JBang compatibility first: script running, Maven tool execution, testing, formatting, publishing, cache management, and JDK handling behind one boringly reliable CLI.

```bash
jbx Hello.java world
jbx dev.telegraphic:hello-tool:1.0.0 -- --help
jbx docs com.fasterxml.jackson.core:jackson-databind --json
jbx doctor Hello.java --json
```

## Why it exists

Agents do not need another clever Java wrapper. They need a tool that is scriptable, inspectable, deterministic, and fast enough to use repeatedly inside coding loops.

- Run Java scripts and Maven executable tools from one command.
- Resolve, fetch, test, format, diagnose, export, publish, and document Java projects.
- Prefer structured output for automation, plain Markdown for reading, and reproducible behavior over magic.
- Stay compatible with JBang where users already have muscle memory.

## Agent-facing surfaces

- [`/llms.txt`](/llms.txt) gives crawlers and agents the canonical short map.
- [`/llms-full.txt`](/llms-full.txt) is the full Markdown corpus.
- Every page is also served as Markdown beside the HTML version.
- [`jbx docs`](/docs/agent-guide/) produces agent-readable Markdown and JSON sidecars for Java libraries.

## Current status

`jbx` is early and moving quickly. The intent is not novelty for novelty’s sake; first the boring parts become reliable, then the interesting parts earn their keep.
