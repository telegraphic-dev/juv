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

### 3. Agent skill alignment
- [ ] Agent-facing guidance in `skill-data/jbx/SKILL.md` is updated when command workflows change.
- [ ] The discovery skill at `skills/jbx/SKILL.md` stays thin and points agents to version-matched `jbx skill` output.
- [ ] `jbx skill list` / `jbx skill get` output remains consistent with the bundled skill data when those commands are touched.

### 4. Dedicated docs
- [ ] Changes to `jbx docs`, JSON sidecars, or documentation schema update `docs/jbx-docs-schema.md` and website docs as needed.
- [ ] Examples remain copy-pasteable and do not mention stale repository names, flags, or URLs.

## Automated Check

Run from the repository root:

```bash
scripts/check-docs-website.sh
```

This script validates whitespace, website generation, and install-script shell syntax. It is not a substitute for the alignment review above; it catches broken docs/website plumbing, not stale prose.

## Verdict Criteria

- **FAIL** if: User-facing CLI/install/docs/skill behavior changed without corresponding README, website, or skill documentation updates.
- **FAIL** if: `scripts/check-docs-website.sh` fails.
- **WARN** if: Documentation is technically present but examples are incomplete, duplicated, or likely to drift.
- **PASS** if: Documentation, website content, bundled skills, and automated checks are aligned with the PR.
