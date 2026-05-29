# jbx website

Static website for `https://jbx.telegraphic.dev`, kept inside the CLI repository so docs, branding, and agent-facing context move with implementation.

## Commands

```bash
npm run build
npm run check
npm run serve
```

The build emits HTML pages, Markdown route siblings, `llms.txt`, `llms-full.txt`, `robots.txt`, and `sitemap.xml` into `dist/`.

## Publishing

GitHub Actions builds `dist/` and deploys it to GitHub Pages from `main`. The custom domain is set by `public/CNAME`.
