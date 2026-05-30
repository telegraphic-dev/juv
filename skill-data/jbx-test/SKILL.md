---
name: jbx-test
description: Run JUnit tests with optional JaCoCo coverage.
---

# `test`

Run JUnit tests with the standalone console launcher; optionally collect JaCoCo coverage.

## When to use it

- Run a small Java kata or library test suite without creating a full build file.
- Give an agent failing test names and stack traces in a parseable shape.
- Collect coverage during a refactor to prove the edited code path is exercised.

## Common workflows

```bash
jbx test src/test/java --json
jbx test --json tests/CalculatorTest.java -- --select-method CalculatorTest#adds
jbx test --coverage --json
```

## Real-life examples

### Run one focused JUnit method during repair

```bash
jbx test --json tests/CalculatorTest.java -- --select-method CalculatorTest#adds
jbx test --coverage --json
```

Use `test` after `check` passes and you need behavioral proof, stack traces, or coverage for a small Java codebase.

### Agent loop

1. Run the narrowest failing test with `--json`.
2. Fix the source or test that explains the failure.
3. Re-run the focused test, then widen to the containing test directory.
4. Use coverage only when the task asks whether code paths are exercised.

## Agent notes

Arguments after `--` go to the JUnit console launcher, not jbx. Keep that boundary explicit in automation.

## JSON and schema

`jbx test --json` returns test execution status, failures, and coverage metadata when requested. Example output and field notes: [`test JSON`](/docs/schemas/#test-json).

Example JSON:

```json
{
  "tests": 1,
  "failures": 0,
  "errors": 0,
  "skipped": 0,
  "testCases": [
    {
      "className": "CalculatorTest",
      "name": "adds()",
      "time": "0.025",
      "status": "passed"
    }
  ]
}
```

## Verification checklist

- JSON reports zero failing tests for the requested scope.
- Focused selection is widened before final confidence claims.
- Coverage output exists and is inspected when `--coverage` is used.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx test`

```text
Run JUnit tests with the standalone console launcher

Usage: jbx test [OPTIONS] [SCRIPT] [ARGS]...

Arguments:
  [SCRIPT]
          Java test source file or directory. Defaults to the current directory

          [default: .]

  [ARGS]...
          Extra arguments passed to the JUnit ConsoleLauncher after defaults

Options:
      --json
          Print converted JUnit XML report as JSON

      --xml
          Print the generated JUnit XML report

      --coverage
          Collect JaCoCo coverage data in target/jacoco.exec

      --jacoco-version <JACOCO_VERSION>
          JaCoCo agent version to use when --coverage is enabled.

          Defaults to the built-in version (0.8.13).

      --junit-version <JUNIT_VERSION>
          JUnit Platform Console Standalone version to use.

          Defaults to the cached latest Maven Central release, refreshed periodically.

      --deps <DEPS>
          Additional dependency coordinates, same shape as //DEPS

      --repo <REPOS>
          Additional repository, same shape as //REPOS

      --source <SOURCES>
          Additional source file, same shape as //SOURCES

      --files <FILES>
          Additional file/resource, same shape as //FILES

      --class-path <CLASSPATH>
          Additional classpath entries

      --javac-option <JAVAC_OPTIONS>
          Additional javac option

      --runtime-option <RUNTIME_OPTIONS>
          Additional java runtime option for the JUnit launcher JVM

      --java <JAVA_VERSION>
          Override //JAVA requested version

      --javaagent <JAVA_AGENTS>
          Additional java agent, same shape as //JAVAAGENT

      --cache-dir <CACHE_DIR>
          Override cache directory

      --trust
          Trust this remote script content hash before testing

  -h, --help
          Print help (see a summary with '-h')
```
