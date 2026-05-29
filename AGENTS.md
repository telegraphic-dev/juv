# AGENTS.md - jbx

This repo is `jbx` in user-facing CLI terms: a Rust-native JBang-compatible Java toolbox. The GitHub repository is still `telegraphic-dev/juv`. The product direction is **JBang parity first**, Rust-native implementation second, and novelty last.

## North star

`jbx` should become the Java tool that is:

- as capable for agents as `zerolang.ai` is: scriptable, inspectable, deterministic, and friendly to autonomous coding/debugging loops;
- the Java equivalent of `uv` / `uvx`: fast project/script execution, dependency resolution, tool execution, testing, formatting, packaging, cache management, and environment/JDK handling behind one boringly reliable CLI;
- more than a JBang clone over time, but only by first preserving JBang-compatible behavior where users already expect it.

## Operating mode

- Start from current `origin/main` unless Vlad explicitly asks to continue an open PR.
- Use small, coherent feature PRs. Do not stack PRs by default.
- Implement with tests first for behavior changes. Watch the focused test fail, then make it pass.
- Finish the normal path: implement, verify locally, commit, push, open PR, check CI/review bots.
- Never merge your own PR.
- Never force-push a shared branch. Add normal follow-up commits.
- Do not manually publish crates.io or GHCR artifacts. Releases go through GitHub Actions.
- Do not commit secrets, tokens, local cache paths, or credentials.

## Project facts

- Repository: `telegraphic-dev/juv`.
- Main binary: `jbx`.
- `jbx Hello.java` runs a script, while `jbx group:artifact:version -- args` runs an executable Maven tool.
- Rust crate version in source may intentionally stay as placeholder `0.0.0`; release workflows derive publish versions from tags.
- Java baseline is **25**. If `--java` or `//JAVA` is omitted, preserve Java 25 behavior unless a test proves otherwise.
- The implementation should avoid depending on Coursier or JBang at runtime. Use native Rust resolver/JDK/catalog logic where it already exists.
- The current code lives mostly in:
  - `src/main.rs` - CLI commands and orchestration
  - `src/lib.rs` - script parsing/build/run/catalog/export helpers
  - `src/resolver.rs` - native Maven resolver
  - `src/jdk.rs` - JDK discovery/provisioning
  - `src/maven_tool.rs` - Maven executable-tool runner reused by `jbx <GAV>`
  - `tests/*.rs` - integration/regression coverage

## Required local gates

Before pushing code changes, run the tightest useful focused test, then the full gate:

```bash
source ~/.cargo/env 2>/dev/null || true
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo test --locked
```

For docs-only or website-facing changes, run the docs/website gate and inspect the diff:

```bash
scripts/check-docs-website.sh
git diff --stat
git status --short
```

If Docker/release packaging changes, build or dry-run the exact changed path locally where practical before pushing.

## Feature workflow

1. Inspect the relevant existing command/test pair before editing. Examples:
   - run/build: `tests/run_java.rs`, `tests/build_command.rs`
   - directives: `tests/directives.rs`
   - resolver/fetch/tool execution: `src/resolver.rs`, `src/maven_tool.rs`, `tests/maven_tool_command.rs`
   - JDK commands/provisioning: `src/jdk.rs`, `tests/jdk_command.rs`
   - catalog/alias/template: `tests/catalog_alias_command.rs`, `tests/external_catalog_command.rs`, `tests/init_command.rs`
   - exports: `tests/export_command.rs`, `tests/native_export_command.rs`
   - formatter: `tests/fmt_command.rs`
   - test runner: `tests/test_command.rs`
2. Check upstream JBang behavior/docs/source when adding compatibility behavior. Treat upstream as the spec, but keep implementation native.
3. Write a focused failing test for the exact user-visible behavior.
4. Implement the smallest correct slice.
5. Run the focused test, then full gates.
6. Update README when the user-facing CLI surface changes.
7. Commit with a conventional message and open a PR.
8. After push, check GitHub CI, jean-ci, mergeability, and unresolved review threads before reporting ready.

## Compatibility principles

- Preserve JBang command shape and directive names unless there is a deliberate reason not to.
- Prefer concrete compatibility slices over vague “edge cases”. Good slices are commands, directives, resources, catalog semantics, export modes, or resolver scenarios with executable tests.
- Keep errors useful and deterministic. If behavior is unsupported, fail clearly rather than silently doing a partial approximation.
- Support Java 25 compact/unnamed-class scripts properly; many regressions hide around imports, helper types, comments, and top-level members before `void main`.
- When formatting/wrapping Java source, preserve leading comments/imports/static imports and detect top-level constructs structurally, not with a happy-path regex.

## Resolver rules

- `resolve()` is metadata-only. Do not download JARs there.
- `resolve_classpath()` downloads artifacts and returns paths.
- `fetch --deps-only` should not download JARs.
- Support Maven semantics deliberately: parent POMs, dependency management/BOMs, relocations, classifiers, exclusions, optional/scope filtering, version ranges, latest/release metadata, intrinsic/chained properties, and plugin-dependency exclusion.
- Cache JARs using group-path directories, not flat filenames; `artifactId-version.jar` collisions are real.
- Sanitize coordinate-derived filesystem path segments. Do not sanitize repository URL path components.
- Reuse local Maven/Gradle/Coursier caches when possible; on Windows copy when symlinks are unsafe.
- Integration tests that need an HTTP repo should use an in-process `TcpListener` bound to port `0`, not `python3 -m http.server` or fixed ports.

## JDK rules

- Discovery order should favor existing local JDKs before downloads: cache, `JAVA_HOME`, `PATH`, JBang, SDKMAN, mise, Gradle, system JVM dirs.
- Validate JDK roots strictly: require real `bin/java`, `bin/javac`, and a JDK `release` file where possible.
- Resolve `/usr/bin/java` symlinks before accepting a PATH candidate; never cache `/usr` as a JDK.
- Bad symlink cache entries must be removed/repaired instead of trusted forever.
- Auto-provision from Adoptium/Eclipse Temurin only when needed, stream downloads, and verify SHA-256 before install.
- macOS `.jdk` bundles use `Contents/Home`; keep platform logic explicit.

## Export and tooling rules

- Runnable JAR export must use URL/percent-encoded manifest `Class-Path` entries and correct 72-byte manifest folding.
- Portable export must reject duplicate portable dependency filenames instead of overwriting in `lib/`.
- Native export should build through normal Java compilation first, then invoke GraalVM `native-image` with classpath, `//NATIVE_OPTIONS`, and CLI `--native-option` values.
- Use fake external tools in integration tests when testing invocation shape (`native-image`, formatters, launchers) rather than requiring the real tool for every CI run.
- `jbx test` uses JUnit Platform Console Standalone by default. Preserve failing-test exit codes. `--json` should convert JUnit XML, not scrape console text.
- Tool execution via Maven coordinates is available through `jbx <GAV> -- [args...]`; keep it tested in `tests/maven_tool_command.rs`.

## Release workflow

- Do not manually push images to GHCR or manually publish crates to crates.io.
- Release tags are annotated tags like `v0.2.1` and should point at current `main` after the release PR is merged.
- Existing tags/releases must be inspected and reused carefully; do not delete/recreate unless explicitly instructed.
- `scripts/set-release-version.py` is the release-version bridge. Source-controlled package version can remain `0.0.0`; do not “fix” that without checking the release design.
- If a tag-triggered workflow fails because the workflow is wrong, fix the workflow in a normal PR, merge, then rerun through the supported workflow path.
- Verify published artifacts from the registry/release asset after green Actions, not just from logs.

## Bot review habits

- jean-ci can report real issues after local gates pass. Read them, fix them, and push normal commits.
- Query unresolved review threads before reporting clean.
- Common review-caught problems in this repo: Windows path/extension handling, stale generated/wrapped Java source assumptions, unresolved review threads, shell quoting in generated scripts, release idempotency, and hardcoded toolchain versions.

## Current backlog shape

README is the quick source for implemented features and explicit gaps. As of this file, known future slices include:

- catalog remove for nested/imported catalogs
- multi-file template expansion
- template property CLI overrides
- export `mavenrepo`, `jlink`, and project variants
- edit/editor integration
- more JBang compatibility edge cases, but only when turned into concrete failing tests

Do not chase the whole museum at once. Pick the next useful exhibit, put it behind tests, and ship it cleanly.
