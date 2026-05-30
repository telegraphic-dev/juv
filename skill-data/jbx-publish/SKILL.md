---
name: jbx-publish
description: Build Maven-ready bundles, local served repositories, or Portal uploads.
---

# `publish`

Publish Java projects to Maven repositories, including Maven Central.

## When to use it

- Prepare a library or script artifact for Maven Central review.
- Create a local Maven repository for integration tests.
- Publish documentation sidecars next to Java artifacts so agents can inspect APIs.

## Common workflows

```bash
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --serve 8080
jbx publish --file jbx.json --publish
```

## Real-life examples

### Dry-run a Maven Central bundle before release

```bash
jbx publish --file jbx.json --dry-run
jbx publish --file jbx.json --serve 0
```

Use `publish` in dry-run or local-serve mode during PR work. Real Portal upload belongs in release automation with approved credentials. The descriptor fields are documented in the [`jbx.json` reference](/docs/jbx-json/).

### Publish a release through GitHub Actions

Keep PR checks boring: build the bundle without credentials, and publish only from a release/manual workflow. A minimal single-artifact release job looks like this:

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: actions/setup-java@v4
    with:
      distribution: temurin
      java-version: '25'
  - uses: dtolnay/rust-toolchain@stable
  - name: Install jbx
    run: cargo install --git https://github.com/telegraphic-dev/jbx.git --locked jbx
  - name: Import GPG signing key
    uses: crazy-max/ghaction-import-gpg@v6
    with:
      gpg_private_key: ${{ secrets.GPG_PRIVATE_KEY }}
      passphrase: ${{ secrets.GPG_PASSPHRASE }}
  - name: Publish
    env:
      CENTRAL_TOKEN_USERNAME: ${{ secrets.CENTRAL_TOKEN_USERNAME }}
      CENTRAL_TOKEN_PASSWORD: ${{ secrets.CENTRAL_TOKEN_PASSWORD }}
      GPG_KEY_ID: ${{ secrets.GPG_KEY_ID }}
    run: |
      VERSION="${{ github.event.inputs.version }}"
      if [ -z "$VERSION" ]; then
        VERSION="${GITHUB_REF_NAME#v}"
      fi
      jbx publish \
        --publish \
        --file jbx.json \
        --version "$VERSION" \
        --gpg-key "$GPG_KEY_ID" \
        --output target/central-bundle.zip \
        --target-dir target/publish \
        --cache-dir .jbx-cache
```

The release workflow needs `CENTRAL_TOKEN_USERNAME`, `CENTRAL_TOKEN_PASSWORD`, `GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`, and `GPG_KEY_ID`. Keep the normal PR workflow on `jbx publish --dry-run --skip-signing`; no Portal credentials or signing key should be needed for review builds.

### Agent loop

1. Validate project metadata and run tests before publishing steps.
2. Run `jbx publish --dry-run` and inspect the generated bundle.
3. Use `--serve 0` for local repository integration tests.
4. Reserve `--publish` for explicit release workflows.

## Agent notes

Publishing is the command with the sharpest edges. Never use `--publish` just to satisfy a docs or PR check.

## Verification checklist

- Dry-run bundle contains valid POM, jars, docs sidecars, checksums, and signatures when signing is enabled.
- `--serve 0` prints a loopback URL and serves Maven metadata.
- Real publishing is gated by explicit release intent and secrets.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx publish`

```text
Prepare Maven Central publishing artifacts

Usage: jbx publish [OPTIONS] [SCRIPT]

Arguments:
  [SCRIPT]  Java source file to publish. Defaults to jbx.json main when --file is used

Options:
      --file <FILE>
          jbx descriptor file. Defaults to ./jbx.json when present
      --version <VERSION>
          Override version from jbx.json or //GAV
  -o, --output <OUTPUT>
          Output Maven Central bundle ZIP path
      --target-dir <TARGET_DIR>
          Working directory for staged publish artifacts
      --package <PACKAGE_NAME>
          Override package used when staging default-package sources
      --cache-dir <CACHE_DIR>
          Override cache directory
      --dry-run
          Prepare and verify artifacts without uploading
      --skip-signing
          Allow unsigned dry-run bundles for local inspection
      --gpg-key <GPG_KEY>
          GPG key ID/email to use for detached ASCII signatures
      --publish
          Upload to Maven Central and publish after validation
      --serve <SERVE>
          Serve a local Maven repository containing the artifact on the given port
      --publishing-type <PUBLISHING_TYPE>
          Maven Central Portal publishing type for the upload [default: automatic] [possible values: automatic, user-managed]
      --no-wait
          Do not poll Central after uploading the deployment bundle
      --max-wait-seconds <MAX_WAIT_SECONDS>
          Maximum seconds to wait for Maven Central publication before exiting [default: 600]
  -h, --help
          Print help
```
