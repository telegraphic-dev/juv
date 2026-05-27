# juv

`juv` is a Rust-native port of JBang, starting with the execution path.

This repository is intentionally aiming for JBang compatibility rather than inventing a new script format.

## Current slice

Implemented now:

- `juv run <script.java> [args...]`
- `juv build <script.java>`
- `juv init <script.java>` default Java template generation
- `juv cache clear`
- `juv cache path [--cache-dir ...]`
- `juv cache list [--json] [--cache-dir ...]`
- `juv trust add|remove|list|clear` for remote script trust entries
- `juv run --trust <http(s)://.../script.java>` / `juv build --trust <http(s)://.../script.java>`
- `juv app install <script.java>` install a script as a PATH command
- `juv app uninstall <name>` remove an installed command
- `juv app list` list installed script commands
- `juv alias list [--json]` list aliases from the nearest `jbang-catalog.json`
- `juv run <alias>` / `juv <alias>` run local catalog aliases
- `juv export local <script.java|alias> [-o app.jar]` export a runnable JAR with local manifest classpath
- `juv export portable <script.java|alias> [-o app.jar]` export a runnable JAR plus `lib/` dependencies
- `juv info classpath <script.java>`
- `juv info tools <script.java>` with `--select`
- `juv info docs <script.java>`
- `juv info cache [--cache-dir ...]`
- `juv info main|java|description|gav|module <script.java>`
- `juv info deps|repos|sources|files <script.java>`
- `juv info compile-options|runtime-options|native-options|javaagents|manifest <script.java>`
- `juv <script.java> [args...]` JBang-style shorthand
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
- `juv build` compiles scripts into cache without running them
- `juv init` creates default Java scripts, supports `--deps`, `--java`, and `--force`
- `juv cache clear` clears the compiled-script cache
- `juv cache path` prints the effective compiled-script cache directory
- `juv cache list` lists cached script entries with their classes/cache directories and supports `--json`
- `juv trust add <url>` stores the current remote script content hash
- `juv trust list`, `remove`, and `clear` manage trusted remote script entries
- remote `http://` and `https://` scripts are downloaded into cache and require matching trust before build/run unless `--trust` is passed
- remote relative `//SOURCES`, source-like `//DEPS`, and `//FILES` resources are downloaded next to the cached remote script
- remote trust hashes cover those downloaded resources; legacy trust entries still work for scripts that do not declare relative resources
- `juv info classpath` prints the script runtime classpath, with `--deps-only`
- `juv info tools` prints JSON metadata for tooling and supports selecting a single field
- `juv info docs` prints description and documentation references
- `juv info main`, `java`, `description`, `gav`, and `module` print single metadata values
- `juv info deps`, `repos`, `sources`, and `files` print directive collections one-per-line
- `juv info compile-options`, `runtime-options`, `native-options`, `javaagents`, and `manifest` print advanced directive collections
- `juv info cache` prints the effective cache directory
- CLI overrides for `--deps`, `--repo`, `--source`, `--files`, `--java`, `--main`, `--compile-option`, `--java-option`, and `--javaagent`
- local catalog aliases from `jbang-catalog.json` / `.jbang/jbang-catalog.json`, including `base-ref`, `script-ref`, `arguments`, `dependencies`, `repositories`, `sources`, `files`, classpaths, Java/runtime/compile options, agents, Java version, and main-class metadata
- `juv export local` creates runnable JARs whose manifest classpath points at local dependency paths
- `juv export portable` creates runnable JARs and copies file-based dependency classpath entries into sibling `lib/`
- Java package-aware main-class inference
- `//FILES` resources copied onto the runtime classpath
- non-coordinate `//DEPS` treated as source dependencies
- `//PREVIEW` compile/runtime flag handling
- `juv resolve <coordinates...>` resolve Maven coordinates to dependency list
- `juv resolve --classpath <coordinates...>` resolve to classpath
- `juv fetch <coordinates...>` fetch artifacts and print classpath
- `juv fetch --deps-only <coordinates...>` print coordinates instead of paths
- `juv jdk list` list discovered/cached JDKs
- `juv jdk home [version]` print the JDK home for a version (default: 25)
- `juv jdk install <version>` download a Temurin JDK from Adoptium
- native Maven dependency resolver (no Coursier required)
- reuses existing Maven (`~/.m2/repository`), Gradle (`~/.gradle/caches`), and Coursier (`~/.cache/coursier`) caches via symlinks
- discovers existing JDKs from `JAVA_HOME`, `PATH`, JBang (`~/.jbang/jdks`), SDKMAN (`~/.sdkman/candidates/java`), mise (`~/.local/share/mise/installs`), Gradle (`~/.gradle/jdks`), and system JVM directories
- caches discovered JDKs under `~/.cache/juv/jdks/<major>` via symlinks so future runs do not rescan everything
- auto-provisions missing JDKs from Adoptium/Eclipse Temurin with SHA-256 archive verification

Not yet implemented: catalog import/add/remove, templates beyond the default, export mavenrepo/native/jlink/project variants, edit integration, native image, and the rest of JBang's lovely edge-case museum.

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
juv run Hello.java world
# hello world
```

## Development

```bash
cargo test --locked
cargo fmt --check
```
