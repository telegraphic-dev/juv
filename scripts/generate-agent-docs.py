#!/usr/bin/env python3
"""Build bundled jbx skills from the website command Markdown.

The website command pages are the curated source of truth. `jbx skill get`
returns a skill-shaped projection of those pages so command docs and bundled
agent guidance cannot drift.
"""
from __future__ import annotations

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
COMMAND_DIR = ROOT / "website/content/pages/docs/commands"
SKILL_DATA_DIR = ROOT / "skill-data"
INSTALLABLE_SKILL_DIR = ROOT / "skills/jbx"

ORDER = [
    "top-level", "run", "build", "check", "test", "docs", "doctor", "rewrite",
    "search", "resolve", "fetch", "info", "cache", "trust", "app", "alias",
    "catalog", "template", "init", "export", "publish", "install", "fmt", "graph",
    "skill", "jdk",
]


def parse_page(path: Path) -> tuple[dict[str, str], str]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        raise SystemExit(f"{path}: missing frontmatter")
    _, frontmatter, body = text.split("---", 2)
    meta: dict[str, str] = {}
    for line in frontmatter.strip().splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            meta[key.strip()] = value.strip()
    return meta, body.lstrip()


def strip_section(body: str, heading: str) -> str:
    pattern = re.compile(rf"(?ms)^## {re.escape(heading)}\n.*?(?=^## |\Z)")
    return pattern.sub("", body)


def strip_skill_bootstrap(body: str) -> str:
    lines: list[str] = []
    skip_empty_after_removed = False
    for line in body.splitlines():
        if "jbx skill get" in line:
            skip_empty_after_removed = True
            continue
        if skip_empty_after_removed and not line.strip():
            skip_empty_after_removed = False
            continue
        skip_empty_after_removed = False
        # Renumber simple ordered-list gaps created by removing discovery steps.
        line = re.sub(r"^2\. Run the command", "1. Run the command", line)
        line = re.sub(r"^3\. Prefer JSON", "2. Prefer JSON", line)
        line = re.sub(r"^4\. Verify", "3. Verify", line)
        lines.append(line)
    text = "\n".join(lines)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip() + "\n"


def skill_name(page_stem: str) -> str:
    return "jbx" if page_stem == "top-level" else f"jbx-{page_stem}"


def skill_markdown(page_stem: str) -> str:
    page = COMMAND_DIR / f"{page_stem}.md"
    meta, body = parse_page(page)
    name = skill_name(page_stem)
    description = meta.get("description", "jbx command guidance")
    body = strip_section(body, "Skill")
    body = strip_skill_bootstrap(body)
    return f"---\nname: {name}\ndescription: {description}\n---\n\n{body}".rstrip() + "\n"


def installable_jbx_skill() -> str:
    return """---
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
"""


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def generate() -> None:
    if INSTALLABLE_SKILL_DIR.parent.exists():
        for child in INSTALLABLE_SKILL_DIR.parent.iterdir():
            if child.is_dir() and child.name != INSTALLABLE_SKILL_DIR.name:
                for nested in sorted(child.rglob("*"), reverse=True):
                    if nested.is_file():
                        nested.unlink()
                    elif nested.is_dir():
                        nested.rmdir()
                child.rmdir()
    for page_stem in ORDER:
        page = COMMAND_DIR / f"{page_stem}.md"
        if not page.exists():
            raise SystemExit(f"Missing command page: {page}")
        name = skill_name(page_stem)
        write(SKILL_DATA_DIR / name / "SKILL.md", skill_markdown(page_stem))
    write(INSTALLABLE_SKILL_DIR / "SKILL.md", installable_jbx_skill())


if __name__ == "__main__":
    generate()
