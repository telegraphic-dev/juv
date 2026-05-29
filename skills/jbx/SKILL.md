---
name: jbx
description: One-stop shop Java toolbox for agents and humans. Inspired by JBang, uv and zerolang.
---

# jbx

jbx is the one-stop shop Java toolbox for agents and humans. Inspired by JBang, uv and zerolang.

Install this discovery skill once in an agent's skill manager. Keep it thin; the `jbx` binary serves version-matched workflow guidance for each installed release.

Install the latest native release:

```sh
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
jbx --version
```

## Version-Matched Skills

This file is a discovery stub. Do not treat it as the full jbx workflow.

Before editing, checking, testing, publishing, or repairing Java code with jbx, ask the installed binary for the skill content that matches that exact version:

```sh
jbx skill list
jbx skill get
```

If the user has multiple jbx binaries, use the same binary that will run the project:

```sh
/path/to/jbx skill list
/path/to/jbx skill get
```

## Common Entry Points

```sh
jbx <GAV|script.java> [args...]
jbx check [path...] [--json]
jbx test [script.java|directory]
jbx rewrite patch --recipe <short|fqn> [--module <short|GAV>] [--source path] [--json]
jbx rewrite apply --recipe <short|fqn> [--module <short|GAV>] [--source path] [--json]
jbx rewrite modules [--search term] [--limit n] [--json]
jbx rewrite recipes <short|GAV> [--search term] [--limit n] [--detail] [--json]
jbx docs <GAV|source|dir> [--json]
jbx skill list
jbx skill get
```
