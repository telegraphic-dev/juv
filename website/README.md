# jbx website

Static website for `https://jbx.telegraphic.dev`, kept inside the CLI repository so docs, branding, and agent-facing context move with implementation.

## Commands

```bash
npm run build
npm run check
npm run serve
```

The build emits HTML pages, Markdown route siblings, `llms.txt`, `llms-full.txt`, `robots.txt`, and `sitemap.xml` into `dist/`.

## Shared CLI docs and skills

Command reference pages are the curated source for both the public website and bundled `jbx skill get ...` content:

```bash
python3 scripts/generate-agent-docs.py
```

That script reads `website/content/pages/docs/commands/*.md` and writes derived skill copies:

- `skill-data/jbx*.md` for `jbx skill list` / `jbx skill get`
- `skills/jbx/SKILL.md` as the only installable discovery stub; command-specific skills are served by the binary, not exposed statically

`scripts/check-docs-website.sh` reruns the generator and fails if the derived skills are stale. Edit command docs first, then regenerate skills.

## Publishing

GitHub Actions builds `dist/` and deploys it to GitHub Pages from `main`. The custom domain is set by `public/CNAME`.
