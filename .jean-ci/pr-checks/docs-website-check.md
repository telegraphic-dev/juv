# Docs and Website Freshness Check

## Purpose
Ensure user-facing CLI behavior, agent guidance, and the public website stay aligned with the code in every PR.

## Review Instructions

First determine whether the PR changes any user-facing behavior, CLI commands/options, publishing/install flows, docs generation, website content, skills, release/install scripts, or agent-facing output. If it only changes internal implementation with no visible behavior change, verify the automated check still passes and return PASS.

When the PR affects visible behavior:

### 1. CLI and README alignment
- [ ] New or changed commands/options are documented in `README.md`.
- [ ] Removed commands/options are removed from `README.md`, examples, and tests/docs references.
- [ ] Install, publishing, and release-flow changes are reflected wherever users would copy commands.

### 2. Website alignment
- [ ] Public-facing changes are reflected in `website/content/pages/` when they affect docs, getting started, agent guidance, brand/install copy, or user-visible workflows.
- [ ] `website/public/install.sh` and install snippets agree when installation behavior changes.
- [ ] `website/src/build.mjs` output expectations still match the content structure, including Markdown route siblings, `llms.txt`, and `llms-full.txt`.

### 3. Bundled skill alignment
- [ ] Every affected bundled runtime skill under `skill-data/*/SKILL.md` is updated when command workflows, gates, examples, or safety rules change.
- [ ] Every affected installable discovery skill under `skills/*/SKILL.md` stays accurate and points agents toward version-matched guidance instead of duplicating stale internals.
- [ ] If the PR adds, renames, removes, lists, or retrieves skills, the CLI behavior, embedded skill data, discovery stubs, README, and website examples all agree.
- [ ] The review is generic across all bundled skills; do not hard-code assumptions that only a `jbx` skill can exist.

### 4. Dedicated docs
- [ ] Changes to `jbx docs`, JSON sidecars, or documentation schema update `docs/jbx-docs-schema.md` and website docs as needed.
- [ ] Examples remain copy-pasteable and do not mention stale repository names, flags, or URLs.

## Automated Check

Run from the repository root when a checkout is available:

```bash
scripts/check-docs-website.sh
```

This script validates whitespace, website generation, install-script shell syntax, and generic bundled skill structure. It is not a substitute for the alignment review above; it catches broken docs/website/skill plumbing, not stale prose.

If the reviewer environment cannot run repository commands because no checkout is available, do not fail solely for that harness limitation. Instead, inspect the diff, GitHub Actions results, and the PR/local verification notes; fail only when the script is reported failing in an available checkout/CI or when the diff itself shows broken plumbing.

## Verdict Criteria

- **FAIL** if: User-facing CLI/install/docs/skill behavior changed without corresponding README, website, or skill documentation updates.
- **FAIL** if: `scripts/check-docs-website.sh` fails in an available checkout or CI/local verification reports it failed.
- **WARN** if: Documentation is technically present but examples are incomplete, duplicated, or likely to drift.
- **PASS** if: Documentation, website content, bundled skills, and automated checks are aligned with the PR.
