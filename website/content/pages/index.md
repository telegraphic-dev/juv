---
title: jbx — All-in-One Java CLI
description: Highly opinionated native command line utility for daily Java tasks: scripts, Maven artifacts, templates, JDKs, docs, formatting, tests, rewriting, ASTs, and publishing.
---

<section class="hero-panel">

# All-in-One Java CLI

The Java ecosystem is rich. So rich that one can get easily lost. `jbx` (pronounced "jay-box") is a highly opinionated native command line utility that provides everything required to use Java for daily tasks.

<div class="install-switch" aria-label="Install jbx">
  <div class="install-tabs" role="tablist" aria-label="Choose installation view">
    <button type="button" role="tab" aria-selected="true" aria-controls="install-humans" id="tab-humans" data-install-tab="humans">For humans</button>
    <span aria-hidden="true">|</span>
    <button type="button" role="tab" aria-selected="false" aria-controls="install-agents" id="tab-agents" data-install-tab="agents">For agents</button>
  </div>
  <div class="install-command-box" id="install-humans" role="tabpanel" aria-labelledby="tab-humans" data-install-panel="humans">
    <pre class="install-command"><code>$ curl -fsSL https://jbx.telegraphic.dev/install.sh | bash</code></pre>
    <button class="copy-command" type="button" data-copy-command="curl -fsSL https://jbx.telegraphic.dev/install.sh | bash">Copy</button>
  </div>
  <div class="install-command-box" id="install-agents" role="tabpanel" aria-labelledby="tab-agents" data-install-panel="agents" hidden>
    <pre class="install-command"><code>$ npx skills add telegraphic-dev/jbx</code></pre>
    <button class="copy-command" type="button" data-copy-command="npx skills add telegraphic-dev/jbx">Copy</button>
  </div>
</div>

</section>

<section class="split-cards">
  <article class="card feature-card">
    <h2>For humans</h2>
    <p><code>jbx</code> executes JBang-style scripts and launches artifacts from Maven Central without turning a small task into a build-system archaeology dig.</p>

```bash
jbx hello.java
jbx org.openapitools:openapi-generator-cli
```
  </article>

  <article class="card feature-card">
    <h2>For agents</h2>
    <p><code>jbx</code> provides the pieces agents need to write Java effectively and verify their work without guessing.</p>

```bash
jbx skill list
jbx skill get jbx-init
jbx init -t hello hello.java
jbx fmt hello.java
jbx init -t test hello_test.java
jbx test hello_test.java --coverage
jbx hello.java
```
  </article>
</section>

## What `jbx` gives you

- templates
- automatic JDK provisioning
- dependency search and resolution
- documentation in structured format
- formatting and linting
- deterministic rewriting and AST manipulation
- artifact publication

<section class="start-here">

## Start here

<p>Continue to the <a href="/docs/">Documentation</a> for common use cases, or use <span class="nowrap"><a href="/docs/commands/">Commands</a> for the CLI reference</span>.</p>

</section>

## Acknowledgement

`jbx` builds on the shoulders of giants:

- [JBang](https://www.jbang.dev/) — Inspiration for Java scripting, directives, and the practical “run this Java file now” workflow.
- [JUnit](https://junit.org/junit5/) — the standard testing platform for modern Java.
- [JaCoCo](https://www.jacoco.org/jacoco/) — Java code coverage reports for test runs.
- [Palantir Java Format](https://github.com/palantir/palantir-java-format) — deterministic Java formatting.
- [Error Prone](https://errorprone.info/) — compile-time bug detection for Java.
- [OpenRewrite](https://docs.openrewrite.org/) — automated source rewriting and recipe-driven code modernization.
- [JavaParser](https://javaparser.org/) — Java AST parsing and serialization for structured source manipulation.
