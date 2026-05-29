---
name: jbx
description: One-stop shop Java toolbox for agents and humans. Inspired by JBang, uv and zerolang.
---

# jbx

jbx is a Rust-native Java toolbox for agents and humans: JBang-compatible script running, Maven tool execution, testing, formatting, publishing, cache management, documentation sidecars, and JDK handling behind one CLI.

Use this skill when working in a repository that uses `jbx`, when authoring Java scripts intended to run with `jbx`, or when an agent needs deterministic Java tooling commands.

## Install

Install the current native release:

```sh
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
jbx --version
```

In a jbx repository checkout, prefer the local binary so behavior matches the code under edit:

```sh
cargo run --locked -- --version
cargo run --locked -- <command>
```

## Common Entry Points

```sh
jbx <GAV|script.java> [args...]
jbx run <script.java> [args...]
jbx build <script.java>
jbx check [path...] [--json]
jbx test [script.java|directory]
jbx fmt [path...]
jbx doctor [script.java|url] [--json] [--cache-dir dir] [--repo id=url] [--publish] [--native]
jbx rewrite patch --recipe <short|fqn> [--module <short|GAV>] [--source path] [--option key=value] [--report dir] [--json]
jbx rewrite apply --recipe <short|fqn> [--module <short|GAV>] [--source path] [--option key=value] [--report dir] [--json]
jbx rewrite modules [--search term] [--limit n] [--json] [--rewrite-version version]
jbx rewrite recipes <short|GAV> [--search term] [--limit n] [--detail] [--json]
jbx docs <GAV|source|dir> [--json]
jbx search <text|group:artifact[:version]> [--json]
jbx resolve <coordinates...>
jbx resolve --classpath <coordinates...>
jbx fetch <coordinates...>
jbx export local <script.java|alias> [-o app.jar]
jbx export portable <script.java|alias> [-o app.jar]
jbx export native <script.java|alias> [-o app]
jbx publish [script.java] --file jbx.json --dry-run --gpg-key <key-id>
CENTRAL_TOKEN_USERNAME=... CENTRAL_TOKEN_PASSWORD=... jbx publish [script.java] --file jbx.json --publish --gpg-key <key-id>
jbx install [script.java] --file jbx.json
jbx jdk list
jbx jdk home [version]
jbx jdk install <version>
```

Use `--json` when another tool or agent needs stable machine-readable output. Run `jbx doctor --json` to diagnose JDK selection, Maven Central reachability, cache writability, formatter fallback, remote trust, dependency resolution/version drift, and optional publishing/native tools.

## Source Rewriting

Use `jbx rewrite patch` before mutating sources. It resolves OpenRewrite with jbx-managed dependencies, scans the requested sources, and writes a preview patch to `rewrite/rewrite.patch` without editing files:

```sh
jbx rewrite patch --recipe auto-format --source src/main/java
jbx rewrite patch --module yaml --recipe org.openrewrite.yaml.format.AutoFormat --source config
jbx rewrite patch --recipe change-package --option old=com.old --option new=com.new --source src --json
```

Use `jbx rewrite apply` only after inspecting the patch or when the task explicitly asks for mutation:

```sh
jbx rewrite apply --recipe cleanup --source src/main/java
jbx rewrite apply --module org.openrewrite.recipe:rewrite-migrate-java:RELEASE --recipe org.openrewrite.java.migrate.UpgradeToJava21 --source src
```

Run-mode options: `--option key=value` passes recipe options, `--report dir` changes where `rewrite.patch` is written, `--json` prints a machine-readable summary, `--fail-on-changes` exits 2 when a recipe would change files, `--no-fail-on-invalid-recipes` continues past invalid active recipes, `--cache-dir dir` changes the helper/dependency cache, `--repo id=url` adds recipe repositories, and `--rewrite-version version` pins the OpenRewrite version used for built-in modules.

Useful discovery commands:

```sh
jbx rewrite modules --search yaml --json
jbx rewrite recipes yaml --search format --detail --json
```

Known recipe aliases include `auto-format`, `format`, `cleanup`, `remove-unused-imports`, and `change-package`. Known module aliases are `java`, `java-21`, `xml`, `yaml`, `properties`, `json`, `maven`, `gradle`, `groovy`, `kotlin`, `protobuf`, and `hcl`. Java recipe support is built in; extra modules are resolved only when supplied with `--module`.

Publishing requires signing plus Maven Central Portal credentials. Use `--gpg-key <key-id>` for signed Central-ready bundles. Supply either `CENTRAL_TOKEN_USERNAME` plus `CENTRAL_TOKEN_PASSWORD`, or `CENTRAL_PORTAL_TOKEN` as `base64(username:password)`. Use `--skip-signing` only for local inspection, not real publishing.

## Agent Workflow

Before editing, checking, testing, or repairing Java code with jbx:

1. Prefer the repository-local `jbx` binary when inside the jbx checkout.
2. Inspect the relevant source/script and nearby `jbx.json`, `jbang-catalog.json`, or `.jbang/jbang-catalog.json` metadata.
3. Use the tightest command first (`jbx check`, `jbx test`, `jbx docs`, or a focused `cargo test` in the jbx repo).
4. For behavior changes in jbx itself, add or update Rust integration tests before changing implementation.
5. Run the repo's full required gate before reporting work as done.

## jbx Repository Development Gate

When changing jbx itself, run:

```sh
source ~/.cargo/env 2>/dev/null || true
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo test --locked
```

For docs-only or website-facing changes, run the docs/website gate:

```sh
scripts/check-docs-website.sh
git diff --stat
git status --short
```

## Compatibility Notes

- Treat upstream JBang behavior and docs as the compatibility spec unless jbx deliberately documents a difference.
- Preserve Java 25 compact/unnamed-class behavior unless a test proves otherwise.
- Prefer clear deterministic errors over silent partial compatibility.
- Keep agent-facing output parseable; use JSON modes for automation.
- Do not require Coursier or JBang at runtime when jbx has native Rust logic for the path.
