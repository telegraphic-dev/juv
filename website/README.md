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

`Dockerfile` builds the static site and serves it with nginx. The repository workflow publishes `ghcr.io/telegraphic-dev/jbx-website:latest` from `main`.
