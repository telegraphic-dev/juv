# doj

`doj` means **do Java**: a Rust-native port of JBang, starting with the execution path.

This repository is intentionally aiming for JBang compatibility rather than inventing a new script format.

## Current slice

Implemented now:

- `doj run <script.java> [args...]`
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
- compile/run cache under the OS cache directory
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
