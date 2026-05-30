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
  title: 'jbx — All-in-One Java CLI',
  description: 'Highly opinionated native command line utility for daily Java tasks: scripts, Maven artifacts, templates, JDKs, docs, formatting, tests, rewriting, ASTs, and publishing.'
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
          out += `<a href="${escapeHtml(md.slice(hrefStart, hrefEnd))}">${inline(md.slice(i + 1, textEnd))}</a>`;
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

function markdownToHtml(markdown, { headingPrefix = '' } = {}) {
  const lines = markdown.split('\n');
  const html = [];
  let inCode = false;
  let codeLang = '';
  let code = [];
  let list = [];
  let listTag = 'ul';
  let paragraph = [];
  const seenHeadingIds = new Map();

  const flushParagraph = () => {
    if (!paragraph.length) return;
    html.push(`<p>${inline(paragraph.join(' '))}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (!list.length) return;
    html.push(`<${listTag}>${list.map(item => `<li>${inline(item)}</li>`).join('')}</${listTag}>`);
    list = [];
    listTag = 'ul';
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
      const baseId = headingPrefix + slugify(text);
      const seen = seenHeadingIds.get(baseId) || 0;
      seenHeadingIds.set(baseId, seen + 1);
      const id = seen === 0 ? baseId : `${baseId}-${seen + 1}`;
      html.push(`<h${level} id="${escapeHtml(id)}">${inline(text)}</h${level}>`);
      continue;
    }
    const bullet = line.match(/^[-*]\s+(.+)$/);
    if (bullet) {
      flushParagraph();
      if (listTag !== 'ul') flushList();
      listTag = 'ul';
      list.push(bullet[1]);
      continue;
    }
    const numbered = line.match(/^\d+[.)]\s+(.+)$/);
    if (numbered) {
      flushParagraph();
      if (listTag !== 'ol') flushList();
      listTag = 'ol';
      list.push(numbered[1]);
      continue;
    }
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
  ['/docs/commands/', 'Commands']
];

const commandDocs = [
  ['alias', 'alias'],
  ['app', 'app'],
  ['build', 'build'],
  ['cache', 'cache'],
  ['catalog', 'catalog'],
  ['check', 'check'],
  ['docs', 'docs'],
  ['doctor', 'doctor'],
  ['export', 'export'],
  ['fetch', 'fetch'],
  ['fmt', 'fmt'],
  ['graph', 'graph'],
  ['info', 'info'],
  ['init', 'init'],
  ['install', 'install'],
  ['jbx', 'top-level'],
  ['jdk', 'jdk'],
  ['publish', 'publish'],
  ['resolve', 'resolve'],
  ['rewrite', 'rewrite'],
  ['run', 'run'],
  ['search', 'search'],
  ['skill', 'skill'],
  ['template', 'template'],
  ['test', 'test'],
  ['trust', 'trust']
];

function shell({ title, description, body, route, rawPath }) {
  const canonical = `${site.origin}${route === '/' ? '/' : route}`;
  const mdLink = rawPath ? `<a class="footer-resource footer-markdown" href="${escapeHtml(rawPath)}" aria-label="Markdown"><span class="footer-icon" aria-hidden="true">MD</span><span class="footer-label">Markdown</span></a>` : '';
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="${escapeHtml(description || site.description)}">
<link rel="canonical" href="${canonical}">
${rawPath ? `<link rel="alternate" type="text/markdown" href="${escapeHtml(site.origin + rawPath)}">` : ''}
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
  <button class="menu-toggle" type="button" aria-controls="site-nav" aria-expanded="false">Menu</button>
  <nav id="site-nav" class="site-nav">${nav.map(([href, label]) => {
    const active = route === href || (href === '/docs/' && route.startsWith('/docs/') && !route.startsWith('/docs/commands/')) || (href === '/docs/commands/' && route.startsWith('/docs/commands/'));
    return `<a href="${href}"${active ? ' aria-current="page"' : ''}>${label}</a>`;
  }).join('')}<button class="theme-toggle" type="button" aria-label="Toggle light and dark theme">Theme</button></nav>
</header>
<main>${body}</main>
<footer>
  <span>jbx by <a href="https://telegraphic.dev">telegraphic.dev</a></span>
  <span>${mdLink}<a class="footer-resource footer-github" href="https://github.com/telegraphic-dev/jbx" aria-label="GitHub"><svg class="footer-icon" aria-hidden="true" viewBox="0 0 16 16"><path fill="currentColor" d="M8 0C3.58 0 0 3.67 0 8.19c0 3.62 2.29 6.68 5.47 7.76.4.08.55-.18.55-.4 0-.2-.01-.86-.01-1.56-2.01.38-2.53-.5-2.69-.95-.09-.23-.48-.95-.82-1.14-.28-.16-.68-.55-.01-.56.63-.01 1.08.59 1.23.83.72 1.24 1.87.89 2.33.68.07-.53.28-.89.51-1.09-1.78-.21-3.64-.91-3.64-4.04 0-.89.31-1.62.82-2.19-.08-.21-.36-1.04.08-2.16 0 0 .67-.22 2.2.84A7.42 7.42 0 0 1 8 3.94c.68 0 1.36.09 2 .27 1.53-1.06 2.2-.84 2.2-.84.44 1.12.16 1.95.08 2.16.51.57.82 1.3.82 2.19 0 3.14-1.87 3.83-3.65 4.04.29.25.54.75.54 1.52 0 1.09-.01 1.97-.01 2.24 0 .22.15.48.55.4A8.11 8.11 0 0 0 16 8.19C16 3.67 12.42 0 8 0Z"/></svg><span class="footer-label">GitHub</span></a></span>
</footer>
<script>
(() => {
  const key = 'jbx-theme';
  const button = document.querySelector('.theme-toggle');
  const menuButton = document.querySelector('.menu-toggle');
  const siteNav = document.querySelector('.site-nav');
  const mobileQuery = matchMedia('(max-width: 760px)');
  const closeMobileNav = () => {
    siteNav?.removeAttribute('data-open');
    menuButton?.setAttribute('aria-expanded', 'false');
  };
  menuButton?.addEventListener('click', () => {
    const open = siteNav?.getAttribute('data-open') === 'true';
    if (open) siteNav?.removeAttribute('data-open');
    else siteNav?.setAttribute('data-open', 'true');
    menuButton.setAttribute('aria-expanded', String(!open));
  });
  mobileQuery.addEventListener?.('change', closeMobileNav);
  if (!mobileQuery.matches) closeMobileNav();
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
  const installTabs = [...document.querySelectorAll('[data-install-tab]')];
  const installPanels = [...document.querySelectorAll('[data-install-panel]')];
  const showInstallPanel = name => {
    for (const tab of installTabs) {
      tab.setAttribute('aria-selected', tab.dataset.installTab === name ? 'true' : 'false');
    }
    for (const panel of installPanels) {
      panel.hidden = panel.dataset.installPanel !== name;
    }
  };
  for (const tab of installTabs) {
    tab.addEventListener('click', () => showInstallPanel(tab.dataset.installTab));
  }
  const copyButtons = [...document.querySelectorAll('[data-copy-command]')];
  const copyText = async text => {
    if (navigator.clipboard?.writeText && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return;
    }
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.setAttribute('readonly', '');
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    const ok = document.execCommand('copy');
    textarea.remove();
    if (!ok) throw new Error('execCommand copy failed');
  };
  for (const copyButton of copyButtons) {
    const originalLabel = copyButton.textContent;
    copyButton.addEventListener('click', async () => {
      try {
        await copyText(copyButton.dataset.copyCommand || '');
        copyButton.textContent = 'Copied';
        copyButton.dataset.copied = 'true';
        window.setTimeout(() => {
          copyButton.textContent = originalLabel;
          delete copyButton.dataset.copied;
        }, 1600);
      } catch {
        copyButton.textContent = 'Failed';
        window.setTimeout(() => { copyButton.textContent = originalLabel; }, 1600);
      }
    });
  }
  const commandSearch = document.querySelector('[data-command-search]');
  const commandLinks = [...document.querySelectorAll('[data-command-link]')];
  const tocDetails = document.querySelector('.toc-details');
  const syncTocDetails = () => {
    if (!tocDetails) return;
    tocDetails.open = !mobileQuery.matches;
  };
  syncTocDetails();
  mobileQuery.addEventListener?.('change', syncTocDetails);
  const filterCommands = () => {
    const query = commandSearch?.value.trim().toLowerCase() || '';
    for (const link of commandLinks) {
      const haystack = (link.textContent + ' ' + link.getAttribute('href')).toLowerCase();
      link.style.display = query && !haystack.includes(query) ? 'none' : '';
    }
  };
  commandSearch?.addEventListener('input', filterCommands);
  filterCommands();
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

function commandPageBody(markdown, route) {
  const currentFile = route === '/docs/commands/' ? null : route.replace(/^\/docs\/commands\//, '').replace(/\/$/, '');
  const tocLinks = commandDocs.map(([label, fileName]) => {
    const href = `/docs/commands/${fileName}/`;
    const current = fileName === currentFile;
    return `<a href="${href}" data-command-link${current ? ' aria-current="page"' : ''}>${label}</a>`;
  }).join('');
  const toc = `<aside class="toc" aria-label="Command table of contents"><details class="toc-details" open><summary>Commands</summary><label class="toc-search"><span>Search commands</span><input type="search" placeholder="search…" autocomplete="off" data-command-search></label><div class="toc-links">${tocLinks}</div></details></aside>`;
  return `<div class="docs-with-toc">${toc}<article class="page commands-reference">${markdownToHtml(markdown)}</article></div>`;
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
    const pageMarkdown = md;
    const html = markdownToHtml(pageMarkdown);
    const heroLogo = route === '/' ? '<img class="hero-logo" src="/assets/jbx-toolbox-logo.png" alt="jbx blue toolbox logo">' : '';
    const body = route.startsWith('/docs/commands/')
      ? commandPageBody(pageMarkdown, route)
      : `<article class="page ${meta.layout || ''}">${heroLogo}${html}</article>`;
    const document = shell({ title: meta.title || site.title, description: meta.description, body, route, rawPath });
    pages.push({ route, rawPath, title: meta.title || site.title, description: meta.description || site.description, md: pageMarkdown });
    fullTexts.push(`# ${meta.title || route}\n\nSource: ${site.origin}${rawPath}\n\n${pageMarkdown.trim()}\n`);
    if (!checkOnly) {
      const dir = path.join(distDir, route === '/' ? '' : route);
      await fs.mkdir(dir, { recursive: true });
      await fs.writeFile(path.join(dir, 'index.html'), document);
      await fs.mkdir(path.dirname(path.join(distDir, rawPath)), { recursive: true });
      await fs.writeFile(path.join(distDir, rawPath), pageMarkdown.trim() + '\n');
    }
  }
  const llms = `# jbx\n\n> ${site.description}\n\n## Canonical URLs\n\n- Website: ${site.origin}/\n- GitHub: https://github.com/telegraphic-dev/jbx\n- Docs: ${site.origin}/docs/\n\n## Pages\n\n${pages.map(p => `- [${p.title}](${site.origin}${p.rawPath}): ${p.description}`).join('\n')}\n`;
  const llmsFull = `${llms}\n---\n\n${fullTexts.join('\n---\n\n')}`;
  const robots = `User-agent: *\nAllow: /\n\nSitemap: ${site.origin}/sitemap.xml\n`;
  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages.map(p => `  <url><loc>${site.origin}${p.route === '/' ? '/' : p.route}</loc></url>`).join('\n')}\n</urlset>\n`;
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
