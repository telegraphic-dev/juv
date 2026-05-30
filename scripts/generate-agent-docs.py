#!/usr/bin/env python3
"""Generate the shared jbx command website pages and bundled skills.

The website and `jbx skill get` should not drift. This script is the single
curated source for command descriptions, examples, JSON notes, and real-world
agent workflows.
"""
from __future__ import annotations

from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[1]

COMMANDS = {
    "run": {
        "desc": "Compile and run one Java source file, including Java 25 compact scripts, with JBang-style directives and CLI overrides.",
        "examples": [
            "jbx run scripts/Report.java -- --month 2026-05",
            "jbx scripts/Report.java --month 2026-05",
            "jbx run --deps info.picocli:picocli:4.7.7 tools/Cli.java -- --help",
        ],
        "use_cases": [
            "Run a one-file maintenance script from a repository without creating a Maven or Gradle project.",
            "Launch a Java 25 compact script that carries `//DEPS`, `//JAVA`, `//SOURCES`, and runtime options in the file.",
            "Smoke-test an executable example after `jbx check --json` has confirmed the source compiles.",
        ],
        "agent": "Treat `run` as the boundary where arbitrary user code executes. For autonomous loops, first inspect with `info`, compile with `build`, or validate with `check --json`; only run after the command and arguments are understood.",
        "json": "No `--json` mode: stdout/stderr belong to the program being run. Use `jbx check --json`, `jbx build`, `jbx info ...`, or `jbx doctor --json` for machine-readable preflight facts before execution.",
    },
    "build": {
        "desc": "Compile a script into the jbx cache without running it.",
        "examples": ["jbx build scripts/Report.java", "jbx build --deps org.slf4j:slf4j-api:2.0.17 tools/Probe.java", "jbx build --java 25 Hello.java"],
        "use_cases": ["Precompile a script during CI so the later run path starts from a warm cache.", "Validate generated Java without executing side effects.", "Check that dependency directives and CLI dependency overrides resolve together."],
        "agent": "Prefer `build` over `run` when the task is “can this script compile?” rather than “execute this script”. Parse the process exit code; do not scrape localized compiler prose when `check --json` would fit better.",
        "json": "No `--json` mode yet. Success/failure is the exit code and compiler diagnostics are printed for humans. Use `jbx check --json` when an agent needs structured diagnostics.",
    },
    "check": {
        "desc": "Run javac `-Xlint:all` and Error Prone by default, optionally as structured diagnostics.",
        "examples": ["jbx check src --json", "jbx check src/main/java --json", "jbx check Hello.java --no-error-prone --json", "jbx check src test --fail-on-warn --json"],
        "use_cases": ["Gate an agent edit before running tests.", "Collect exact diagnostics for a PR review comment or automated repair loop.", "Check a Java 25 compact script where a plain build tool would not understand the wrapper semantics."],
        "agent": "Use this as the first quality gate after edits. JSON diagnostics are the contract; make fixes from structured file/line/column fields, then rerun until the status is clean or only accepted warnings remain.",
        "json": "`--json` returns command status and diagnostics with file, line, column, severity, tool, code/message, and suggested next action. Website schema: `/docs/schemas/#check-json`.",
    },
    "test": {
        "desc": "Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.",
        "examples": ["jbx test src/test/java --json", "jbx test tests/CalculatorTest.java -- --select-method CalculatorTest#adds --json", "jbx test --coverage --json"],
        "use_cases": ["Run a small Java kata or library test suite without creating a full build file.", "Give an agent failing test names and stack traces in a parseable shape.", "Collect coverage during a refactor to prove the edited code path is exercised."],
        "agent": "Start with focused tests when repairing a failure, then broaden to the directory or suite. Preserve non-zero exits for failed tests; do not hide failures behind “JSON parsed successfully”.",
        "json": "`--json` reports status, selected tests, failures, console XML paths, and optional coverage paths/counters. Website schema: `/docs/schemas/#test-json`.",
    },
    "docs": {
        "desc": "Generate Markdown or JSON documentation from local Java sources, directories, docs sidecars, or Maven artifacts.",
        "examples": ["jbx docs src/main/java", "jbx docs com.fasterxml.jackson.core:jackson-databind:2.17.2 --json", "jbx docs docs/my-library-jbx-docs.json"],
        "use_cases": ["Inspect an unfamiliar dependency before writing integration code.", "Publish or consume sidecar docs that agents can read without decompiling jars.", "Generate local API notes for a small script or library as part of CI."],
        "agent": "Use docs before guessing APIs. Prefer JSON when extracting types/methods programmatically; use Markdown for human handoff. If a Maven artifact has a sidecar, trust the sidecar version that matches the artifact coordinate.",
        "json": "`--json` follows the published docs sidecar schema in `/docs/jbx-docs-schema/` and is summarized at `/docs/schemas/#docs-json`.",
    },
    "doctor": {
        "desc": "Check JDK selection, Maven Central, cache writability, formatter fallback, remote trust, dependencies, update drift, and optional publish/native tools.",
        "examples": ["jbx doctor --json", "jbx doctor Hello.java --json", "jbx doctor https://example.com/tool.java --json"],
        "use_cases": ["Diagnose why a script works on one machine but not in CI.", "Check whether a workstation has Java 25, Maven reachability, writable caches, and optional native-image support.", "Before filing a bug report, capture environment facts without exposing secrets."],
        "agent": "Run `doctor --json` before making environment assumptions. Report failed checks and actionable fixes, not a wall of raw environment text.",
        "json": "`--json` returns checks with name, status (`ok`, `warn`, `fail`, `skipped`), summary, details, and remediation hints. Website schema: `/docs/schemas/#doctor-json`.",
    },
    "rewrite": {
        "desc": "Preview or apply OpenRewrite recipes with jbx-managed dependencies and JDKs; discover modules and recipes.",
        "examples": ["jbx rewrite modules --search spring --json", "jbx rewrite recipes org.openrewrite.recipe:rewrite-testing-frameworks:3.8.0 --detail --json", "jbx rewrite patch --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json", "jbx rewrite apply --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json"],
        "use_cases": ["Preview a modernization recipe and review the patch before touching files.", "Discover which recipe module contains a migration an agent wants to run.", "Apply a mechanical cleanup after tests already cover the behavior."],
        "agent": "Default to `patch`, not `apply`. Treat `apply` as a mutating operation that needs an explicit task. After applying, run `jbx check --json` and relevant tests.",
        "json": "JSON modes exist for `patch`, `apply`, `modules`, and `recipes`. Schemas are summarized at `/docs/schemas/#rewrite-json`.",
    },
    "search": {
        "desc": "Search Maven Central artifacts by text or coordinates, with filters.",
        "examples": ["jbx search picocli --json", "jbx search --group org.junit.platform console standalone --json", "jbx search --latest com.fasterxml.jackson.core:jackson-databind --json"],
        "use_cases": ["Find the current coordinate for a library before adding `//DEPS`.", "Resolve ambiguous artifact names when a README only gives a product name.", "Let an agent rank candidate dependencies without scraping Maven Central HTML."],
        "agent": "Prefer exact group/artifact filters once a candidate is known. Do not auto-upgrade production dependencies solely because search shows a newer version.",
        "json": "`--json` returns query metadata, `numFound`, and artifact records. Website schema: `/docs/schemas/#search-json`.",
    },
    "resolve": {
        "desc": "Resolve Maven coordinates to dependency coordinates without running code.",
        "examples": ["jbx resolve com.fasterxml.jackson.core:jackson-databind:2.17.2", "jbx resolve --scope runtime org.slf4j:slf4j-simple:2.0.17", "jbx resolve --tree com.acme:app:1.0.0"],
        "use_cases": ["Inspect the dependency graph before committing a new `//DEPS`.", "Check whether exclusions or runtime scopes change the graph.", "Debug version mediation without executing user code."],
        "agent": "Use `resolve` for metadata questions. It should not be treated as proof that jars are already present locally; use `fetch` for that.",
        "json": "No `--json` mode yet. Output is dependency coordinates or classpath-style text depending on flags. Use `fetch` when artifacts must be downloaded.",
    },
    "fetch": {
        "desc": "Download artifacts and print classpath or dependency coordinates.",
        "examples": ["jbx fetch com.fasterxml.jackson.core:jackson-databind:2.17.2", "jbx fetch --classpath org.junit.platform:junit-platform-console-standalone:1.11.4", "jbx fetch --deps-only com.acme:app:1.0.0"],
        "use_cases": ["Materialize jars before an offline CI step.", "Build a classpath for an external Java command.", "Verify that dependency artifacts are reachable from configured repositories."],
        "agent": "Use `fetch` when file availability matters. Keep cache paths out of committed files and logs unless they are intentionally diagnostic.",
        "json": "No `--json` mode yet. Output is meant for shell composition: classpath strings, paths, or dependency lists.",
    },
    "info": {
        "desc": "Print parsed directives and derived metadata from Java scripts.",
        "examples": ["jbx info deps Hello.java", "jbx info repos Hello.java", "jbx info java Hello.java", "jbx info directives Hello.java"],
        "use_cases": ["Read `//DEPS` before modifying a script.", "Check which Java version or main class a script declares.", "Extract docs/cache metadata for packaging or publishing workflows."],
        "agent": "Prefer `info` over ad-hoc parsing of `//` directives. If multiple facts are needed, call the specific subcommands and keep each output scoped.",
        "json": "No global `--json` mode yet; subcommands return focused text values. Use it for deterministic extraction instead of regexing source.",
    },
    "cache": {
        "desc": "Inspect or clear compiled-script cache paths and entries.",
        "examples": ["jbx cache path", "jbx cache list --json", "jbx cache clear --older-than 30d"],
        "use_cases": ["Locate compiled outputs while debugging why a stale script runs.", "Prune old entries on a constrained CI runner.", "Let an agent verify that a build warmed the expected cache namespace."],
        "agent": "Never clear caches blindly during a debugging task; list first and scope destructive cleanup. Cache paths are machine-local facts, not portable configuration.",
        "json": "`jbx cache list --json` returns cache entries with paths, keys, sizes, and timestamps when available. Other cache subcommands are path/mutation oriented.",
    },
    "trust": {
        "desc": "Pin, list, remove, or clear trusted hashes for remote scripts.",
        "examples": ["jbx trust list", "jbx trust add https://example.com/tool.java", "jbx trust remove https://example.com/tool.java"],
        "use_cases": ["Pin a reviewed remote script before automation runs it.", "Rotate trust after a remote script intentionally changes.", "Audit which URLs are allowed to run without prompting."],
        "agent": "Remote trust changes are security-sensitive. Ask before adding/removing trust unless the user explicitly requested it, and always show the URL/hash being trusted.",
        "json": "No `--json` mode yet. Trust operations are small and human-auditable; use explicit subcommands and verify the listed hash after changes.",
    },
    "app": {
        "desc": "Install, list, or uninstall Java scripts as PATH commands.",
        "examples": ["jbx app install tools/report.java --name report", "jbx app list", "jbx app uninstall report"],
        "use_cases": ["Promote a frequently used Java script into a normal shell command.", "Install a repository-local developer tool in CI images.", "Remove stale wrappers after a script is renamed."],
        "agent": "Installing modifies user PATH-facing state. Confirm intent unless the task explicitly asks for installation. After install, run the command with `--help` or a harmless argument.",
        "json": "No `--json` mode yet. Use `app list` for installed command names and paths.",
    },
    "alias": {
        "desc": "Add, remove, and list aliases from nearby `jbang-catalog.json` files.",
        "examples": ["jbx alias list --json", "jbx alias add hello Hello.java --description \"Run the hello script\"", "jbx alias remove hello"],
        "use_cases": ["Expose a repository script as a stable team command.", "Inspect catalog aliases before deciding what `jbx <alias>` means.", "Clean up renamed scripts while keeping the catalog understandable."],
        "agent": "Read `alias list --json` before changing a catalog. Preserve human descriptions because they become discovery text for future agents.",
        "json": "`jbx alias list --json` returns aliases and target metadata from the discovered catalog. Website schema: `/docs/schemas/#alias-json`.",
    },
    "catalog": {
        "desc": "Add and list external catalogs in `jbang-catalog.json`.",
        "examples": ["jbx catalog list --json", "jbx catalog add team https://example.com/jbang-catalog.json", "jbx catalog add local ./tools/jbang-catalog.json"],
        "use_cases": ["Share common script aliases across repositories.", "Inspect imported catalogs before resolving an alias.", "Add a team catalog during project setup."],
        "agent": "Catalog changes affect command discovery. List first, avoid duplicate names, and prefer pinned/reviewed URLs over random raw links.",
        "json": "`jbx catalog list --json` returns catalog names, URLs, and local resolution details. Website schema: `/docs/schemas/#catalog-json`.",
    },
    "template": {
        "desc": "List built-in and imported templates for `jbx init`.",
        "examples": ["jbx template list", "jbx template list --json", "jbx init --template cli Hello.java"],
        "use_cases": ["Choose a starting point for a CLI, test, or compact script.", "Let an agent discover available project conventions before creating files.", "Document which templates a team catalog provides."],
        "agent": "Run `template list --json` before `init` when the requested shape is vague. Prefer templates from the current repository/catalog over generic guesses.",
        "json": "`jbx template list --json` returns template names, descriptions, origin catalog, and parameters when known.",
    },
    "init": {
        "desc": "Create Java 25+ scripts from built-in or imported templates.",
        "examples": ["jbx init Hello.java", "jbx init --template cli tools/hello.java", "jbx init --template test src/test/java/SmokeTest.java"],
        "use_cases": ["Start a new compact Java script with the expected shebang/directive style.", "Create a small CLI skeleton with Picocli or team defaults.", "Generate a test file that can immediately run through `jbx test`."],
        "agent": "This creates files. If the target exists, do not overwrite without explicit instruction. After creation, inspect the file and run the narrowest validation command.",
        "json": "No `--json` mode yet. The output is a newly created file; verify by checking the file and running `jbx check --json`.",
    },
    "export": {
        "desc": "Export local, portable, or native runnable artifacts.",
        "examples": ["jbx export local Hello.java --output build/hello.jar", "jbx export portable Hello.java --output dist/hello", "jbx export native Hello.java --output dist/hello"],
        "use_cases": ["Package a script for a machine that should not re-resolve dependencies at runtime.", "Create a portable directory with jars and launch metadata for CI artifacts.", "Build a native executable when GraalVM/native-image is available."],
        "agent": "Export is a build artifact operation. Check the output path, run the produced artifact with a harmless argument, and keep native-image failures actionable rather than swallowing tool output.",
        "json": "No `--json` mode yet. Verify produced files directly in the requested output directory.",
    },
    "publish": {
        "desc": "Build Maven Central-ready bundles, local served repositories, or Portal uploads from `jbx.json` and sources.",
        "examples": ["jbx publish --file jbx.json --dry-run", "jbx publish --file jbx.json --local-repo build/repo", "jbx publish --file jbx.json --portal-upload"],
        "use_cases": ["Prepare a library or script artifact for Maven Central review.", "Create a local Maven repository for integration tests.", "Publish documentation sidecars next to Java artifacts so agents can inspect APIs."],
        "agent": "Publishing can be external and irreversible. Use `--dry-run` first, inspect generated POMs/artifacts/signatures, and ask before real Portal upload unless explicitly requested.",
        "json": "No `--json` mode yet. Use dry-run output and generated bundle files as the verification contract.",
    },
    "install": {
        "desc": "Install the current project into a Maven repository layout, usually `~/.m2/repository`.",
        "examples": ["jbx install --file jbx.json", "jbx install --file jbx.json --repo build/local-m2", "jbx install src/main/java/com/acme/Tool.java"],
        "use_cases": ["Make a local artifact available to another script via `//DEPS`.", "Test generated POM metadata before publish.", "Install a snapshot into an isolated local repository for CI."],
        "agent": "Prefer a temporary `--repo` during automated tests to avoid polluting the developer’s real `~/.m2`. Verify the installed coordinates by resolving them.",
        "json": "No `--json` mode yet. Verification is the installed POM/JAR path under the target repository.",
    },
    "fmt": {
        "desc": "Format Java files with Palantir Java Format, including Java 25 compact scripts.",
        "examples": ["jbx fmt src/main/java", "jbx fmt Hello.java", "jbx fmt --check src test"],
        "use_cases": ["Normalize generated Java before committing.", "Format compact scripts that have leading directives/comments.", "Run after OpenRewrite or AST import to keep diffs readable."],
        "agent": "Formatting mutates files unless check mode is used. Inspect the diff afterwards and do not combine formatting noise with semantic edits unless requested.",
        "json": "No `--json` mode yet. Formatting is verified by clean diff or a check-mode exit code when available.",
    },
    "graph": {
        "desc": "Dump JavaParser native AST JSON or import it back to Java source.",
        "examples": ["jbx graph dump Hello.java > Hello.ast.json", "jbx graph import Hello.ast.json --output Hello.java", "jbx graph dump src/main/java/com/acme/App.java | jq ."],
        "use_cases": ["Let an agent perform structural analysis without regexing Java.", "Round-trip a generated AST back to source after a controlled transformation.", "Debug parser behavior around compact Java scripts."],
        "agent": "Prefer AST operations for structural changes, but verify with `jbx check --json` after import. Treat generated source as code changes requiring review.",
        "json": "`dump` emits JavaParser native AST JSON. `import` consumes that JSON and writes Java source. This is an AST interchange format, not a stable semantic schema for public APIs.",
    },
    "skill": {
        "desc": "List and print version-matched bundled agent skills.",
        "examples": ["jbx skill list", "jbx skill list --json", "jbx skill get jbx-check", "jbx skill get jbx"],
        "use_cases": ["Bootstrap an agent with command-specific guidance before touching a Java repo.", "Discover whether the installed jbx release knows a command or workflow.", "Keep offline automation aligned with the exact binary version, not a stale website page."],
        "agent": "This is the first command an agent should run. Fetch the specific command skill, follow it, then use the command page only for broader human context.",
        "json": "`jbx skill list --json` returns installed skill names and descriptions. `skill get` returns Markdown skill content.",
    },
    "jdk": {
        "desc": "List, install, and locate JDKs used by jbx.",
        "examples": ["jbx jdk list", "jbx jdk home 25", "jbx jdk install 25"],
        "use_cases": ["Verify which JDK will run Java 25 compact scripts.", "Install a missing Temurin JDK for a CI runner.", "Debug why JAVA_HOME differs from the JDK selected by jbx."],
        "agent": "JDK installation downloads external binaries and changes local state. Prefer `doctor --json` and `jdk list` first; install only when needed and requested.",
        "json": "No `--json` mode yet. Use `jbx doctor --json` for structured environment checks until JDK subcommands grow dedicated JSON.",
    },
}

ORDER = list(COMMANDS.keys())
MOTTO = "Single agent-friendly entry point to the Java ecosystem."


def bullets(items: list[str]) -> str:
    return "\n".join(f"- {item}" for item in items)


def fenced(examples: list[str], lang: str = "bash") -> str:
    return f"```{lang}\n" + "\n".join(examples) + "\n```"


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content.rstrip() + "\n")


def as_list(data: dict[str, Any], key: str) -> list[str]:
    return cast(list[str], data[key])


def as_str(data: dict[str, Any], key: str) -> str:
    return cast(str, data[key])


def command_page(name: str, data: dict[str, Any]) -> str:
    desc = as_str(data, "desc")
    examples = as_list(data, "examples")
    use_cases = as_list(data, "use_cases")
    agent = as_str(data, "agent")
    json_note = as_str(data, "json")
    return f"""---
title: jbx {name} command
description: {desc}
---

# `{name}`

{desc}

## When to use it

{bullets(use_cases)}

## Common workflows

{fenced(examples)}

## Real-life examples

### Repository maintenance

Use `{name}` as part of a repeatable repository workflow rather than a one-off shell trick. Start from the smallest safe command, inspect its output, then widen the scope only after the result is clear.

### Agent loop

1. Discover guidance with `jbx skill get jbx-{name}`.
2. Run the command in the narrowest scope that answers the task.
3. Prefer JSON/structured output when this command exposes it.
4. Verify the claimed result with files, exit codes, or the next quality gate.

## Agent notes

{agent}

## JSON and schema

{json_note}

## Verification checklist

- Confirm the command exit code matches the intended gate.
- For mutating commands, inspect `git diff` or the generated artifact path.
- For JSON modes, parse the output instead of scraping the human form.
- For dependency/JDK/network behavior, run `jbx doctor --json` when the environment is suspect.

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-{name}
```
"""


def command_skill(name: str, data: dict[str, Any], installable: bool = False) -> str:
    desc = as_str(data, "desc")
    examples = as_list(data, "examples")
    use_cases = as_list(data, "use_cases")
    agent = as_str(data, "agent")
    json_note = as_str(data, "json")
    suffix = "\n> Tip: for exact release behavior, rerun `jbx skill get jbx-{}` from the target machine.\n".format(name) if installable else ""
    return f"""---
name: jbx-{name}
description: {desc}
---

# jbx-{name}

{desc}

This skill is bundled with `jbx` so agents can get guidance that matches the installed binary:

```sh
jbx skill get jbx-{name}
```

## Use when

{bullets(use_cases)}

## Quick commands

{fenced(examples, 'sh')}

## Practical workflow

1. Read the current repository state and identify the smallest target: one file, one directory, one coordinate, or one catalog entry.
2. Run the safest inspection form first. If a JSON mode exists, use it and parse it as data.
3. Make the requested change only after the command output supports it.
4. Verify with the command itself plus the next higher gate (`jbx check --json`, `jbx test --json`, artifact inspection, or `git diff`).

## Real-life use cases

{bullets(use_cases)}

## Agent guidance

{agent}

## Structured output

{json_note}

## Common mistakes

- Do not infer command semantics from old web snippets; this skill reflects the installed release.
- Do not scrape human output when a JSON mode exists.
- Do not widen scope from a single file to the whole repository until the focused command is clean.
- Do not hide non-zero exits behind a successful parser or wrapper script.

## Verification

- Parse JSON output where available and validate required fields.
- For file changes, inspect `git diff --stat` and the exact changed files.
- For generated artifacts, test that the expected output path exists and is usable.
- For environment failures, run `jbx doctor --json` and report the failed checks with remediation.
{suffix}"""


def generate() -> None:
    for name, data in COMMANDS.items():
        write(ROOT / f"website/content/pages/docs/commands/{name}.md", command_page(name, data))
        write(ROOT / f"skill-data/jbx-{name}/SKILL.md", command_skill(name, data))
        write(ROOT / f"skills/jbx-{name}/SKILL.md", command_skill(name, data, installable=True))

    command_links = "\n".join(f"- [`{name}`](/docs/commands/{name}/) — {as_str(COMMANDS[name], 'desc')}" for name in ORDER)
    top_page = f"""---
title: jbx top-level command
description: {MOTTO}
---

# `jbx`

{MOTTO}

The top-level command stays intentionally thin. It gives humans a fast run path and gives agents a discovery path; detailed behavior lives in dedicated subcommands and version-matched skills.

## Start here

```bash
jbx skill list
jbx skill list --json
jbx skill get jbx
jbx skill get jbx-check
```

## Common thin entry-point workflows

```bash
jbx Hello.java world
jbx dev.telegraphic:hello-tool:1.0.0 -- --help
jbx skill get jbx-run
jbx doctor --json
```

## Command pages

{command_links}

## Agent notes

- Use `jbx skill list --json` for discovery.
- Fetch the command-specific skill before running non-trivial commands.
- Prefer explicit subcommands over relying on top-level shorthand in automation.
- Treat top-level script/tool execution as code execution; inspect before running unknown inputs.

## JSON and schema

No top-level JSON mode. The top-level entry point forwards humans to common run paths and agents to `jbx skill`. Use dedicated JSON modes such as `jbx skill list --json`, `jbx doctor --json`, `jbx check --json`, `jbx docs --json`, `jbx search --json`, `jbx test --json`, and `jbx rewrite ... --json`.
"""
    write(ROOT / "website/content/pages/docs/commands/top-level.md", top_page)

    top_skill = f"""---
name: jbx
description: {MOTTO}
---

# jbx

{MOTTO}

`jbx` is the Java toolbox entry point for scripts, Maven tools, tests, formatting, publishing, dependency lookup, docs sidecars, diagnostics, cache management, and JDK handling. Keep the top-level usage thin: discover skills, then use the dedicated command.

## Install

```sh
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
jbx --version
```

## First commands for agents

```sh
jbx skill list --json
jbx skill get jbx
jbx skill get jbx-check
jbx doctor --json
jbx check [path...] --json
```

## Real-life workflows

### Repair a Java script after an edit

```sh
jbx skill get jbx-check
jbx check Hello.java --json
jbx build Hello.java
jbx run Hello.java -- --help
```

### Explore an unfamiliar dependency

```sh
jbx search picocli --json
jbx docs info.picocli:picocli:4.7.7 --json
jbx resolve info.picocli:picocli:4.7.7
```

### Prepare a safe modernization

```sh
jbx skill get jbx-rewrite
jbx rewrite modules --search format --json
jbx rewrite patch --recipe org.openrewrite.java.format.AutoFormat --source src/main/java --json
jbx check src/main/java --json
```

### Publish or install locally

```sh
jbx publish --file jbx.json --dry-run
jbx install --file jbx.json --repo build/local-m2
jbx docs com.acme:tool:1.0.0 --json
```

## Command-specific skills

Every command has a bundled skill named `jbx-<command>`. Examples:

```sh
jbx skill get jbx-run
jbx skill get jbx-test
jbx skill get jbx-docs
jbx skill get jbx-publish
```

## Agent operating rules

1. Run `jbx skill list --json` to discover installed guidance.
2. Fetch the specific skill for the command you need.
3. Prefer JSON modes when they exist; parse JSON rather than scraping human text.
4. Use `jbx doctor --json` before guessing about JDKs, caches, Maven reachability, remote trust, formatter fallback, dependency drift, publishing, or native-image setup.
5. Verify generated artifacts directly: files for mutating commands, schemas for JSON commands, and exit codes for gates.

## Compatibility notes

- Preserve JBang-compatible command shape and directives unless a task explicitly asks for a difference.
- Preserve Java 25 compact/unnamed-class behavior unless a test proves otherwise.
- Prefer clear deterministic errors over silent partial compatibility.
- Keep agent-facing output parseable and documented.
"""
    write(ROOT / "skill-data/jbx/SKILL.md", top_skill)
    write(ROOT / "skills/jbx/SKILL.md", top_skill + "\n> Tip: for exact release behavior, rerun `jbx skill get jbx` from the target machine.\n")


if __name__ == "__main__":
    generate()
    print(f"generated {len(COMMANDS)} command pages and skills")
