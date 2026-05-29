# jbx

`jbx` is a Rust-native Java toolbox: JBang-compatible script running, Maven tool execution, testing, formatting, publishing, cache management, and JDK handling behind one CLI.

```bash
curl -fsSL https://jbx.telegraphic.dev/install.sh | bash
export PATH="$HOME/.jbx/bin:$PATH"
jbx <GAV|script.java> [args...]
```

The installer downloads a native release binary for Linux or macOS; Rust is only needed when building from source.

This repository is intentionally aiming for JBang compatibility rather than inventing a new script format.

## Current slice

Implemented now:

- `jbx run <script.java> [args...]`
- `jbx build <script.java>`
- `jbx check [path...]` checks Java source files/directories with javac `-Xlint:all` and Error Prone by default
- `jbx init <script.java>` built-in Java template generation
- `jbx template list [--json]` list built-in and imported catalog init templates
- `jbx cache clear`
- `jbx cache path [--cache-dir ...]`
- `jbx cache list [--json] [--cache-dir ...]`
- `jbx trust add|remove|list|clear` for remote script trust entries
- `jbx run --trust <http(s)://.../script.java>` / `jbx build --trust <http(s)://.../script.java>`
- `jbx app install <script.java>` install a script as a PATH command
- `jbx app uninstall <name>` remove an installed command
- `jbx app list` list installed script commands
- `jbx alias list [--json]` list aliases from the nearest `jbang-catalog.json`
- `jbx alias add <script.java|url> [args...]` add a local catalog alias
- `jbx alias remove <name>` remove a local catalog alias
- `jbx catalog add <name> <catalog-path-or-url> [--import]` register an external catalog
- `jbx catalog list [--json]` list external catalogs from the nearest `jbang-catalog.json`
- `jbx run <alias>` / `jbx <alias>` run local catalog aliases
- `jbx test [script.java|directory]` run JUnit tests with the standalone console launcher
- `jbx fmt [path...]` format Java files with Palantir Java Format, including Java 25 compact scripts
- `jbx graph dump <script.java>` print JavaParser's native AST JSON serialization
- `jbx graph import <ast.json> [-o script.java]` convert JavaParser's native AST JSON serialization back to Java source
- `jbx export local <script.java|alias> [-o app.jar]` export a runnable JAR with local manifest classpath
- `jbx export portable <script.java|alias> [-o app.jar]` export a runnable JAR plus `lib/` dependencies
- `jbx export native <script.java|alias> [-o app]` export a native executable via GraalVM `native-image`
- `jbx publish [script.java] --file jbx.json --dry-run` prepares a Maven Central bundle ZIP using flat `group`/`id`/`version` metadata; `--version` overrides the descriptor version
- `jbx publish [script.java] --file jbx.json --publish` signs the artifacts, uploads the bundle through the Maven Central Portal API, and waits for publication
- `jbx publish [script.java] --file jbx.json --serve <port>` serves the prepared artifact from a local Maven-compatible HTTP repository
- `jbx install [script.java] --file jbx.json [--destination repo]` installs the artifact into a Maven repository layout; default destination is `~/.m2/repository`
- `jbx skill list` lists version-matched agent skills bundled with this jbx release
- `jbx skill get [name]` prints a bundled agent skill; defaults to the main `jbx` skill
- `jbx docs <GAV|source|dir> [--json]` prints agent-friendly documentation; remote `group:artifact` resolves the latest release before fetching sidecars, remote GAV sidecars are cached, local sources are generated fresh
- `jbx info classpath <script.java>`
- `jbx info tools <script.java>` with `--select`
- `jbx info docs <script.java>`
- `jbx info cache [--cache-dir ...]`
- `jbx info main|java|description|gav|module <script.java>`
- `jbx info deps|repos|sources|files <script.java>`
- `jbx info compile-options|runtime-options|native-options|javaagents|manifest <script.java>`
- `jbx <GAV|script.java> [args...]` run a Java script or Maven executable tool
- `jbx --main <class> <GAV> -- [args...]` run a main class with the resolved classpath
- `jbx search [text|group:artifact[:version]] [--group G] [--id A] [--limit N] [--json]` search Maven Central artifacts
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
- `jbx build` compiles scripts into cache without running them
- `jbx init` creates Java 25+ unnamed-class scripts from built-in or imported catalog templates (`hello`/`java`, `compact`, `cli`, `agent`, plus catalog `templates` entries), supports `--deps`, `--java`, `--template`, and `--force`
- `jbx template list` lists built-in and imported catalog init templates and supports `--json`
- `jbx cache clear` clears the compiled-script cache
- `jbx cache path` prints the effective compiled-script cache directory
- `jbx cache list` lists cached script entries with their classes/cache directories and supports `--json`
- `jbx trust add <url>` stores the current remote script content hash
- `jbx trust list`, `remove`, and `clear` manage trusted remote script entries
- remote `http://` and `https://` scripts are downloaded into cache and require matching trust before build/run unless `--trust` is passed
- remote relative `//SOURCES`, source-like `//DEPS`, and `//FILES` resources are downloaded next to the cached remote script
- remote trust hashes cover those downloaded resources
- `jbx info classpath` prints the script runtime classpath, with `--deps-only`
- `jbx info tools` prints JSON metadata for tooling and supports selecting a single field
- `jbx info docs` prints description and documentation references
- `jbx info main`, `java`, `description`, `gav`, and `module` print single metadata values
- `jbx info deps`, `repos`, `sources`, and `files` print directive collections one-per-line
- `jbx info compile-options`, `runtime-options`, `native-options`, `javaagents`, and `manifest` print advanced directive collections
- `jbx info cache` prints the effective cache directory
- CLI overrides for `--deps`, `--repo`, `--source`, `--files`, `--java`, `--main`, `--compile-option`, `--java-option`, and `--javaagent`
- local and imported catalog aliases from `jbang-catalog.json` / `.jbang/jbang-catalog.json`, including `alias add/remove`, `catalog add/list`, `base-ref`, `script-ref`, `catalog-ref`, `import`, `arguments`, `dependencies`, `repositories`, `sources`, `files`, classpaths, Java/runtime/compile options, agents, Java version, and main-class metadata
- imported catalog templates with `file-refs` and default `properties`, usable via `jbx init --template <name>`
- `jbx export local` creates runnable JARs whose manifest classpath points at local dependency paths
- `jbx export portable` creates runnable JARs and copies file-based dependency classpath entries into sibling `lib/`
- `jbx export native` compiles scripts then invokes GraalVM `native-image`, passing `//NATIVE_OPTIONS` plus `--native-option` values
- `jbx publish --dry-run` reads `jbx.json` by default, accepts flat `group` / `id` / `version` metadata, supports `--version` overrides, compiles/stages all declared Java sources including compact unnamed-class scripts, and writes a Maven repository-layout Central bundle with main, sources, generated javadoc when possible, POM, `-jbx-docs.md` / `-jbx-docs.json` sidecars, and required checksum artifacts
- `jbx publish --publish` uploads the signed Central bundle to the Portal API with `publishingType=AUTOMATIC` by default and polls `/api/v1/publisher/status` until it is `PUBLISHED` or `FAILED`
- `jbx publish --serve <port>` prepares the same Maven repository layout unsigned and serves it from `http://127.0.0.1:<port>/`; port `0` asks the OS to choose a free port; it also serves artifact-level `maven-metadata.xml` plus checksums so version-less Maven lookups work
- `jbx install` installs the current project into `~/.m2/repository` by default or another Maven-layout repository with `--destination` / `--to`; it updates `maven-metadata-local.xml` for the installed artifact
- `jbx skill list` and `jbx skill get [name]` expose version-matched agent guidance bundled with the binary from editable `skill-data/` files
- `jbx docs <source|dir>` generates Markdown docs from local Java sources without writing cache entries
- `jbx docs <group:artifact>` resolves the latest Maven release metadata before fetching `artifact-version-jbx-docs.md`
- `jbx docs <group:artifact:version> [--json]` fetches `artifact-version-jbx-docs.md` or `.json` Maven sidecars and caches remote results under the docs cache namespace; see [`docs/jbx-docs-schema.md`](docs/jbx-docs-schema.md) for the JSON shape
- `jbx graph dump <script.java>` emits JavaParser's native JSON serialization directly; there is no jbx-specific graph text or custom JSON shape
- `jbx graph import <ast.json>` deserializes JavaParser JSON and prints JavaParser-formatted Java source to stdout; pass `-o <script.java>` / `--output <script.java>` to write a file
- Java 25 compact source files and `import module` declarations are parsed directly by JavaParser 3.28.1+
- Java package-aware main-class inference
- `//FILES` resources copied onto the runtime classpath
- non-coordinate `//DEPS` treated as source dependencies; Maven coordinates may be `group:artifact:version`, `group:artifact:classifier:version`, or just `group:artifact` to resolve the latest release from Maven metadata
- `//PREVIEW` compile/runtime flag handling
- `jbx resolve <coordinates...>` resolve Maven coordinates to dependency list
- `jbx resolve --classpath <coordinates...>` resolve to classpath
- `jbx fetch <coordinates...>` fetch artifacts and print classpath
- `jbx fetch --deps-only <coordinates...>` print coordinates instead of paths
- `jbx search <text|group:artifact[:version]>` queries Maven Central Search; `--group`, `--id`/`--artifact-id`, and `--version` add Solr field filters (`g`, `a`, `v`); default text output is an aligned table sorted by version-count popularity, and `--json` returns structured `query`, `numFound`, and `artifacts` for agents
- `jbx jdk list` list discovered/cached JDKs
- `jbx jdk home [version]` print the JDK home for a version (default: 25)
- `jbx jdk install <version>` download a Temurin JDK from Adoptium
- native Maven dependency resolver (no Coursier required)
- reuses existing Maven (`~/.m2/repository`), Gradle (`~/.gradle/caches`), and Coursier (`~/.cache/coursier`) caches via symlinks
- discovers existing JDKs from `JAVA_HOME`, `PATH`, JBang (`~/.jbang/jdks`), SDKMAN (`~/.sdkman/candidates/java`), mise (`~/.local/share/mise/installs`), Gradle (`~/.gradle/jdks`), and system JVM directories
- caches discovered JDKs under `~/.cache/jbx/jdks/<major>` via symlinks so future runs do not rescan everything
- auto-provisions missing JDKs from Adoptium/Eclipse Temurin with SHA-256 archive verification

Not yet implemented: catalog remove for nested catalogs, multi-file template expansion, template property CLI overrides, export mavenrepo/jlink/project variants, edit integration, and the rest of JBang's lovely edge-case museum.

## Example

```java
//JAVA 25+
void main(String[] args) {
  IO.println("hello " + args[0]);
}
```

```bash
jbx check
# checks all Java files under the current directory with javac -Xlint:all and Error Prone

jbx check src --no-error-prone --json
# structured javac diagnostics only
```

```bash
jbx run Hello.java world
# hello world

jbx dev.telegraphic:hello-tool:1.0.0 -- --help
# runs an executable JAR from Maven coordinates
```

### `jbx.json` publish descriptor

```json
{
  "main": "src/main/java/dev/telegraphic/demo/HelloTool.java",
  "group": "dev.telegraphic.demo",
  "id": "hello-tool",
  "version": "1.0.0",
  "package": "dev.telegraphic.demo",
  "description": "Small demo tool",
  "url": "https://github.com/telegraphic-dev/hello-tool",
  "licenses": [
    {
      "name": "MIT License",
      "url": "https://opensource.org/licenses/MIT"
    }
  ],
  "developers": [
    {
      "name": "Telegraphic",
      "organizationUrl": "https://github.com/telegraphic-dev"
    }
  ],
  "scm": {
    "connection": "scm:git:https://github.com/telegraphic-dev/hello-tool.git",
    "developerConnection": "scm:git:ssh://git@github.com/telegraphic-dev/hello-tool.git",
    "url": "https://github.com/telegraphic-dev/hello-tool"
  },
  "java": "25",
  "dependencies": [
    "info.picocli:picocli:4.7.7"
  ],
  "repositories": [
    "https://repo.maven.apache.org/maven2"
  ]
}
```

```bash
jbx publish --file jbx.json --dry-run --gpg-key you@example.com
jbx publish --file jbx.json --version 1.0.1 --dry-run --gpg-key you@example.com
jbx publish --file jbx.json --dry-run --skip-signing  # local inspection only
CENTRAL_TOKEN_USERNAME=... CENTRAL_TOKEN_PASSWORD=... jbx publish --file jbx.json --publish --gpg-key you@example.com
# or set CENTRAL_PORTAL_TOKEN to base64(username:password)
```

Use `--version` when release/tag workflows need to publish a different version than the descriptor. The `main` field accepts either a source path (`src/main/java/dev/telegraphic/demo/HelloTool.java`) or a Java FQN (`dev.telegraphic.demo.HelloTool`). If `main` has no extension, `jbx publish` first checks the exact path, then tries `.java`, `.jsh`, and `.jav` next to the descriptor, then scans Java sources under the descriptor directory for a matching package/class declaration; missing main files get an explicit `publish main source not found` error.

For GitHub-hosted repositories, `jbx publish` can prefill Maven Central POM `url`, `licenses`, `developers`, and `scm` metadata from the `origin` remote plus `gh repo view` when those fields are omitted. Put the fields in `jbx.json` when you want explicit release metadata instead of GitHub-derived defaults. Signed Central-ready bundles require a configured GPG key; `--skip-signing` is only for local inspection. Real Portal publishing requires a generated Maven Central user token supplied via environment variables only: preferably `CENTRAL_TOKEN_USERNAME` / `CENTRAL_TOKEN_PASSWORD`, or `CENTRAL_PORTAL_TOKEN` containing the base64-encoded `username:password` value expected by the Portal API.

## Development

```bash
cargo test --locked
cargo fmt --check
```
