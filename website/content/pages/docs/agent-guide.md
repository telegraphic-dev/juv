---
title: Agent guide for jbx
description: How autonomous agents should consume jbx and this website.
---

# Agent guide

Use `jbx` as a command-line surface, not as a guessing game. Prefer commands that print structured facts, keep cache paths explicit in long-running jobs, and read Markdown docs before scraping HTML.

## Website contract

- Read `/llms.txt` first for the canonical map.
- Read `/llms-full.txt` when you need the full current website context in one request.
- Prefer route Markdown (`/docs/getting-started.md`) over HTML when building context.
- Use canonical links under `https://jbx.telegraphic.dev/`.

## CLI contract

Prefer these commands for automation:

```bash
jbx skill list
jbx skill get
jbx check <path> --json
jbx search <query> --json
jbx docs <source|dir|GAV> --json
jbx doctor [script.java|url] --json
jbx rewrite patch --recipe <short|fqn> --source <path> --json
jbx rewrite modules --json
jbx rewrite recipes <short|GAV> --detail --json
jbx info tools <script.java>
jbx info deps <script.java>
jbx info classpath <script.java>
jbx cache path
```

## Good agent behavior

- Run the tightest command that answers the question.
- Use `jbx doctor --json` before guessing about broken JDKs, unreachable Maven metadata, unwritable caches, formatter fallback, remote trust, dependency resolution, or optional publishing/native toolchain setup. Add `--cache-dir` for isolated agent runs, `--repo`/`--repos` for extra repositories, `--publish` for GPG checks, and `--native` for GraalVM `native-image` checks.
- Start with `jbx skill get` when you need version-matched workflow guidance for the installed binary.
- Keep local caches between runs where possible; Java dependency downloads are not a personality test.
- Treat `jbx docs` Markdown as context and `--json` as structured lookup data.
- Prefer `jbx rewrite patch` over `jbx rewrite apply` until the generated `rewrite/rewrite.patch` has been inspected.
- Use `jbx rewrite modules --json` and `jbx rewrite recipes <module> --detail --json` to discover OpenRewrite modules, recipe descriptions, and options instead of guessing FQNs.
- Use `--option key=value` for recipe parameters, `--report dir` for patch location, `--fail-on-changes` for CI-style checks, `--repo id=url` for private recipe modules, `--cache-dir dir` for isolated jobs, and `--rewrite-version version` when reproducing a specific OpenRewrite run.
- Preserve JBang-compatible directives in scripts unless a task explicitly asks to change behavior.
- When publishing, use dry-run and inspect staged artifacts before real release workflows.
