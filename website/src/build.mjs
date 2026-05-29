import fs from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';

const root = path.resolve(new URL('..', import.meta.url).pathname);
const contentDir = path.join(root, 'content', 'pages');
const publicDir = path.join(root, 'public');
const distDir = path.join(root, 'dist');
const checkOnly = process.argv.includes('--check');
const site = {
  origin: 'https://jbx.telegraphic.dev',
  title: 'jbx — Java tools for agents',
  description: 'A Rust-native, JBang-compatible Java toolbox built for autonomous agents and impatient humans.'
};

function escapeHtml(value = '') {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}

function slugify(value) {
  return value.toLowerCase().replace(/`/g, '').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

function parseFrontmatter(markdown) {
  if (!markdown.startsWith('---\n')) return [{}, markdown];
  const end = markdown.indexOf('\n---\n', 4);
  if (end === -1) return [{}, markdown];
  const raw = markdown.slice(4, end).trim().split('\n');
  const data = {};
  for (const line of raw) {
    const idx = line.indexOf(': ');
    if (idx > -1) data[line.slice(0, idx).trim()] = line.slice(idx + 2).trim().replace(/^['"]|['"]$/g, '');
  }
  return [data, markdown.slice(end + 5).trimStart()];
}

function matchingParen(value, start) {
  let depth = 0;
  for (let i = start; i < value.length; i += 1) {
    if (value[i] === '(') depth += 1;
    if (value[i] === ')') {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function inline(md) {
  let out = '';
  for (let i = 0; i < md.length;) {
    if (md[i] === '`') {
      const end = md.indexOf('`', i + 1);
      if (end !== -1) {
        out += `<code>${escapeHtml(md.slice(i + 1, end))}</code>`;
        i = end + 1;
        continue;
      }
    }
    if (md.startsWith('**', i)) {
      const end = md.indexOf('**', i + 2);
      if (end !== -1) {
        out += `<strong>${escapeHtml(md.slice(i + 2, end))}</strong>`;
        i = end + 2;
        continue;
      }
    }
    if (md[i] === '[') {
      const textEnd = md.indexOf('](', i + 1);
      if (textEnd !== -1) {
        const hrefStart = textEnd + 2;
        const hrefEnd = matchingParen(md, hrefStart - 1);
        if (hrefEnd !== -1) {
          out += `<a href="${escapeHtml(md.slice(hrefStart, hrefEnd))}">${escapeHtml(md.slice(i + 1, textEnd))}</a>`;
          i = hrefEnd + 1;
          continue;
        }
      }
    }
    out += escapeHtml(md[i]);
    i += 1;
  }
  return out;
}

function markdownToHtml(markdown) {
  const lines = markdown.split('\n');
  const html = [];
  let inCode = false;
  let codeLang = '';
  let code = [];
  let list = [];
  let paragraph = [];

  const flushParagraph = () => {
    if (!paragraph.length) return;
    html.push(`<p>${inline(paragraph.join(' '))}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (!list.length) return;
    html.push(`<ul>${list.map(item => `<li>${inline(item)}</li>`).join('')}</ul>`);
    list = [];
  };
  const flushCode = () => {
    html.push(`<pre><code class="language-${escapeHtml(codeLang)}">${escapeHtml(code.join('\n'))}</code></pre>`);
    code = []; codeLang = '';
  };

  for (const line of lines) {
    if (line.startsWith('```')) {
      if (inCode) { flushCode(); inCode = false; }
      else { flushParagraph(); flushList(); inCode = true; codeLang = line.slice(3).trim(); }
      continue;
    }
    if (inCode) { code.push(line); continue; }
    if (!line.trim()) { flushParagraph(); flushList(); continue; }
    if (line.trim().startsWith('<')) {
      flushParagraph(); flushList();
      html.push(line);
      continue;
    }
    const heading = line.match(/^(#{1,4})\s+(.+)$/);
    if (heading) {
      flushParagraph(); flushList();
      const level = heading[1].length;
      const text = heading[2].trim();
      html.push(`<h${level} id="${slugify(text)}">${inline(text)}</h${level}>`);
      continue;
    }
    const bullet = line.match(/^[-*]\s+(.+)$/);
    if (bullet) { flushParagraph(); list.push(bullet[1]); continue; }
    flushList();
    paragraph.push(line.trim());
  }
  flushParagraph(); flushList();
  if (inCode) flushCode();
  return html.join('\n');
}

const nav = [
  ['/', 'Home'],
  ['/docs/', 'Docs'],
  ['/brand/', 'Brand'],
  ['/llms.txt', 'llms.txt']
];

function shell({ title, description, body, route, rawPath }) {
  const canonical = `${site.origin}${route === '/' ? '/' : route}`;
  const mdLink = rawPath ? `<a href="${rawPath}">Markdown</a>` : '';
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="${escapeHtml(description || site.description)}">
<link rel="canonical" href="${canonical}">
<meta property="og:title" content="${escapeHtml(title)}">
<meta property="og:description" content="${escapeHtml(description || site.description)}">
<meta property="og:image" content="${site.origin}/assets/social-card.png">
<meta property="og:type" content="website">
<link rel="icon" href="/assets/favicon.png">
<link rel="stylesheet" href="/styles.css">
</head>
<body>
<header class="site-header">
  <a class="mark" href="/"><img src="/assets/jbx-toolbox-logo-256.png" alt="jbx toolbox logo"><span>jbx</span></a>
  <nav>${nav.map(([href, label]) => `<a href="${href}"${route === href ? ' aria-current="page"' : ''}>${label}</a>`).join('')}<button class="theme-toggle" type="button" aria-label="Toggle light and dark theme">Theme</button></nav>
</header>
<main>${body}</main>
<footer>
  <span>jbx by Telegraphic</span>
  <span>${mdLink}<a href="https://github.com/telegraphic-dev/jbx">GitHub</a><a href="/llms-full.txt">llms-full.txt</a></span>
</footer>
<script>
(() => {
  const key = 'jbx-theme';
  const button = document.querySelector('.theme-toggle');
  const preferred = () => matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  const apply = theme => {
    document.documentElement.dataset.theme = theme;
    if (button) button.textContent = theme === 'light' ? 'Dark' : 'Light';
  };
  apply(localStorage.getItem(key) || preferred());
  button?.addEventListener('click', () => {
    const next = document.documentElement.dataset.theme === 'light' ? 'dark' : 'light';
    localStorage.setItem(key, next);
    apply(next);
  });
})();
</script>
</body>
</html>`;
}

async function copyDir(src, dest) {
  await fs.mkdir(dest, { recursive: true });
  for (const entry of await fs.readdir(src, { withFileTypes: true })) {
    const s = path.join(src, entry.name);
    const d = path.join(dest, entry.name);
    if (entry.isDirectory()) await copyDir(s, d);
    else await fs.copyFile(s, d);
  }
}

async function pageEntries() {
  const out = [];
  async function walk(dir) {
    for (const entry of await fs.readdir(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) await walk(full);
      else if (entry.name.endsWith('.md')) out.push(full);
    }
  }
  await walk(contentDir);
  return out.sort();
}

function routeFor(file) {
  const rel = path.relative(contentDir, file).replace(/\\/g, '/').replace(/\.md$/, '');
  if (rel === 'index') return '/';
  if (rel.endsWith('/index')) return `/${rel.slice(0, -'/index'.length)}/`;
  return `/${rel}/`;
}

async function build() {
  if (!checkOnly) await fs.rm(distDir, { recursive: true, force: true });
  if (!checkOnly) await fs.mkdir(distDir, { recursive: true });
  if (!checkOnly) await copyDir(publicDir, distDir);
  const pages = [];
  const fullTexts = [];
  for (const file of await pageEntries()) {
    const raw = await fs.readFile(file, 'utf8');
    const [meta, md] = parseFrontmatter(raw);
    const route = routeFor(file);
    const rawPath = route === '/' ? '/index.md' : `${route.replace(/\/$/, '')}.md`;
    const html = markdownToHtml(md);
    const heroLogo = route === '/' ? '<img class="hero-logo" src="/assets/jbx-toolbox-logo.png" alt="jbx blue toolbox logo">' : '';
    const body = `<article class="page ${meta.layout || ''}">${heroLogo}${html}</article>`;
    const document = shell({ title: meta.title || site.title, description: meta.description, body, route, rawPath });
    pages.push({ route, rawPath, title: meta.title || site.title, description: meta.description || site.description, md });
    fullTexts.push(`# ${meta.title || route}\n\nSource: ${site.origin}${rawPath}\n\n${md.trim()}\n`);
    if (!checkOnly) {
      const dir = path.join(distDir, route === '/' ? '' : route);
      await fs.mkdir(dir, { recursive: true });
      await fs.writeFile(path.join(dir, 'index.html'), document);
      await fs.mkdir(path.dirname(path.join(distDir, rawPath)), { recursive: true });
      await fs.writeFile(path.join(distDir, rawPath), md.trim() + '\n');
    }
  }
  const llms = `# jbx\n\n> ${site.description}\n\n## Canonical URLs\n\n- Website: ${site.origin}/\n- GitHub: https://github.com/telegraphic-dev/jbx\n- Agent guide: ${site.origin}/docs/agent-guide/\n- Markdown corpus: ${site.origin}/llms-full.txt\n\n## Pages\n\n${pages.map(p => `- [${p.title}](${site.origin}${p.rawPath}): ${p.description}`).join('\n')}\n`;
  const llmsFull = `${llms}\n---\n\n${fullTexts.join('\n---\n\n')}`;
  const robots = `User-agent: *\nAllow: /\n\nSitemap: ${site.origin}/sitemap.xml\n`;
  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages.map(p => `  <url><loc>${site.origin}${p.route === '/' ? '/' : p.route}</loc></url>`).join('\n')}\n  <url><loc>${site.origin}/llms.txt</loc></url>\n  <url><loc>${site.origin}/llms-full.txt</loc></url>\n</urlset>\n`;
  if (!checkOnly) {
    await fs.writeFile(path.join(distDir, 'llms.txt'), llms);
    await fs.writeFile(path.join(distDir, 'llms-full.txt'), llmsFull);
    await fs.writeFile(path.join(distDir, 'robots.txt'), robots);
    await fs.writeFile(path.join(distDir, 'sitemap.xml'), sitemap);
    const manifest = { name: 'jbx', short_name: 'jbx', start_url: '/', display: 'standalone', background_color: '#090f16', theme_color: '#0f7fe8', icons: [{ src: '/assets/icon-192.png', sizes: '192x192', type: 'image/png' }] };
    await fs.writeFile(path.join(distDir, 'site.webmanifest'), JSON.stringify(manifest, null, 2));
  }
  const digest = crypto.createHash('sha256').update(llmsFull).digest('hex').slice(0, 12);
  console.log(`Built ${pages.length} pages for ${site.origin} (${digest})`);
}

build().catch(error => { console.error(error); process.exit(1); });
