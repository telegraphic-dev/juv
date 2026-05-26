# doj

`doj` means **do Java**: a Rust-native port of JBang, starting with the execution path.

This repository is intentionally aiming for JBang compatibility rather than inventing a new script format.

## Current slice

Implemented now:

- `doj run <script.java> [args...]`
- `doj build <script.java>`
- `doj init <script.java>` default Java template generation
- `doj cache clear`
- `doj cache path [--cache-dir ...]`
- `doj cache list [--json] [--cache-dir ...]`
- `doj info classpath <script.java>`
- `doj info tools <script.java>` with `--select`
- `doj info docs <script.java>`
- `doj info cache [--cache-dir ...]`
- `doj info main|java|description|gav|module <script.java>`
- `doj info deps|repos|sources|files <script.java>`
- `doj info compile-options|runtime-options|native-options|javaagents|manifest <script.java>`
- `doj <script.java> [args...]` JBang-style shorthand
- directive parsing for:
  - `//JAVA`
  - `//DEPS`
  - `//REPOS`
  - `//SOURCES`
  - `//FILES`
  - `//PREVIEW`
  - `//JAVAC_OPTIONS` / `//COMPILE_OPTIONS`
  - `//RUNTIME_OPTIONS` / `//JAVA_OPTIONS`
  - `//MAIN`
  - `//DESCRIPTION`
  - `//GAV`
  - `//MODULE`
  - `//DOCS`
  - `//MANIFEST`
  - `//JAVAAGENT`
  - `//NATIVE_OPTIONS`
  - `//CDS`
  - `//NOINTEGRATIONS`
- compile/run cache under the OS cache directory
- `doj build` compiles scripts into cache without running them
- `doj init` creates default Java scripts, supports `--deps`, `--java`, and `--force`
- `doj cache clear` clears the compiled-script cache
- `doj cache path` prints the effective compiled-script cache directory
- `doj cache list` lists cached script entries with their classes/cache directories and supports `--json`
- `doj info classpath` prints the script runtime classpath, with `--deps-only`
- `doj info tools` prints JSON metadata for tooling and supports selecting a single field
- `doj info docs` prints description and documentation references
- `doj info main`, `java`, `description`, `gav`, and `module` print single metadata values
- `doj info deps`, `repos`, `sources`, and `files` print directive collections one-per-line
- `doj info compile-options`, `runtime-options`, `native-options`, `javaagents`, and `manifest` print advanced directive collections
- `doj info cache` prints the effective cache directory
- CLI overrides for `--deps`, `--repo`, `--source`, `--files`, `--java`, `--main`, `--compile-option`, `--java-option`, and `--javaagent`
- Java package-aware main-class inference
- `//FILES` resources copied onto the runtime classpath
- non-coordinate `//DEPS` treated as source dependencies
- `//PREVIEW` compile/runtime flag handling
- dependency resolution through Coursier when `//DEPS` is used

Not yet implemented: catalogs, app install, templates, trust, export, edit integration, native image, full remote script handling, and the rest of JBang's lovely edge-case museum.

## Example

```java
//MAIN Hello
class Hello {
  public static void main(String[] args) {
    System.out.println("hello " + args[0]);
  }
}
```

```bash
doj run Hello.java world
# hello world
```

## Development

```bash
cargo test --locked
cargo fmt --check
```
