#!/usr/bin/env node
// Source-of-truth generator for the per-game PWA/home-screen icon set.
//
// Each game's glyph is authored once below in a 0..100 design space (bbox ~[10,90]),
// in the shared CRT palette, then composed into three framings and rasterised with
// headless Chromium (best SVG-filter/glow fidelity available on this box):
//
//   icon.svg                 scalable master (the `any` framing; doubles as favicon.svg)
//   icon-192.png             any       transparent rounded tile + dim keyline
//   icon-512.png             any
//   icon-192-maskable.png    maskable  opaque, full-bleed, glyph inside the adaptive safe zone
//   icon-512-maskable.png    maskable
//   apple-touch-icon.png     apple     180x180 opaque, full-bleed (iOS rounds it itself)
//
// Files are written straight into games/<slug>/assets/. The manifest that references
// them is generated/served by riparion-cms (it owns the canonical /apps/<slug>/run/
// origin + scope); this repo only ships the art. See NOTES.md "Icons".
//
// Prereqs (same recipe as the screenshot tooling in NOTES.md):
//   npm install playwright-core   # in a dir on NODE_PATH, or beside this script
//   CHROMIUM=/usr/bin/chromium    # override if elsewhere
// Run:  node scripts/gen-icons.mjs [--sheet]   (--sheet also writes /tmp/icon-contact-sheet.png)

import { writeFileSync, readFileSync, mkdirSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
let chromium;
try {
  ({ chromium } = require('playwright-core'));
} catch {
  console.error('playwright-core not found. Run `npm install playwright-core` (see NOTES.md) and/or set NODE_PATH.');
  process.exit(1);
}
const CHROMIUM = process.env.CHROMIUM || '/usr/bin/chromium';
const SHEET = process.argv.includes('--sheet');
const GAMES_DIR = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'games');

const PH = '#33ff66', BG = '#020402', DIM = '#1f9c3f';

// glyphs: SVG markup in 0..100, bbox ~[10,90] (~0.8 coverage). `adj` balances visual weight.
const GLYPHS = {
  hammurabi: { adj: 1.0, svg: `
    <polygon points="16,86 84,86 76,74 24,74" fill="${PH}"/>
    <polygon points="26,72 74,72 67,60 33,60" fill="${PH}"/>
    <polygon points="35,58 65,58 59,48 41,48" fill="${PH}"/>
    <rect x="44" y="38" width="12" height="8" fill="${PH}"/>
    <polygon points="50,29 45.5,38 54.5,38" fill="${PH}"/>
    <rect x="46.5" y="48" width="7" height="38" fill="${BG}"/>
    <g fill="${PH}">
      <rect x="46.5" y="52" width="7" height="1.6"/><rect x="46.5" y="58" width="7" height="1.6"/>
      <rect x="46.5" y="64" width="7" height="1.6"/><rect x="46.5" y="70" width="7" height="1.6"/>
      <rect x="46.5" y="76" width="7" height="1.6"/><rect x="46.5" y="82" width="7" height="1.6"/>
    </g>` },

  taipan: { adj: 1.0, svg: `
    <path d="M14,86 Q23,83 32,86 T50,86 T68,86 T86,86" stroke="${DIM}" stroke-width="1.6" fill="none" stroke-linecap="round"/>
    <path d="M16,68 L84,68 Q76,82 50,82 Q24,82 16,68 Z" fill="${PH}"/>
    <rect x="49" y="22" width="2" height="46" fill="${PH}"/>
    <polygon points="51,22 61,25 51,29" fill="${PH}"/>
    <polygon points="51,30 75,35 75,62 51,65" fill="${PH}"/>
    <g stroke="${BG}" stroke-width="1.4">
      <line x1="51" y1="38" x2="75" y2="42"/><line x1="51" y1="46" x2="75" y2="49"/><line x1="51" y1="54" x2="75" y2="56"/>
    </g>
    <polygon points="49,33 33,48 49,53" fill="${PH}"/>
    <line x1="47" y1="41" x2="36" y2="46" stroke="${BG}" stroke-width="1.3"/>` },

  'oregon-trail': { adj: 1.0, svg: `
    <path d="M26,53 Q26,24 50,24 Q74,24 74,53 Z" fill="${PH}"/>
    <g stroke="${BG}" stroke-width="1.5" fill="none">
      <line x1="38" y1="28.5" x2="38" y2="53"/><line x1="50" y1="25" x2="50" y2="53"/><line x1="62" y1="28.5" x2="62" y2="53"/>
    </g>
    <ellipse cx="27.5" cy="40" rx="3.4" ry="10" fill="${BG}"/>
    <path d="M20,52 L80,52 Q82,60 75,64 L25,64 Q18,60 20,52 Z" fill="${PH}"/>
    <g stroke="${PH}" stroke-width="3" fill="none"><circle cx="64" cy="71" r="12"/><circle cx="33" cy="72" r="10"/></g>
    <g stroke="${PH}" stroke-width="1.5">
      <line x1="64" y1="60" x2="64" y2="82"/><line x1="53" y1="71" x2="75" y2="71"/>
      <line x1="56.5" y1="63.5" x2="71.5" y2="78.5"/><line x1="56.5" y1="78.5" x2="71.5" y2="63.5"/>
      <line x1="33" y1="63" x2="33" y2="81"/><line x1="24" y1="72" x2="42" y2="72"/>
      <line x1="26.5" y1="65.5" x2="39.5" y2="78.5"/><line x1="26.5" y1="78.5" x2="39.5" y2="65.5"/>
    </g>
    <circle cx="64" cy="71" r="2.4" fill="${PH}"/><circle cx="33" cy="72" r="2.2" fill="${PH}"/>` },

  'fur-trader': { adj: 1.0, svg: `
    <path d="M31,60 Q13,62 11,74 Q13,85 29,81 Q38,77 35,66 Z" fill="${PH}"/>
    <g stroke="${BG}" stroke-width="1.1" fill="none">
      <path d="M16,73 Q25,71 33,72"/><line x1="18" y1="67" x2="22" y2="79"/>
      <line x1="24" y1="65.5" x2="28" y2="80"/><line x1="30" y1="66" x2="32" y2="78"/>
    </g>
    <path d="M28,54 Q28,33 48,33 Q67,33 71,50 Q74,68 58,74 Q39,78 30,69 Q23,61 28,54 Z" fill="${PH}"/>
    <circle cx="67" cy="45" r="11" fill="${PH}"/>
    <path d="M76,46 Q83,45 82,50 Q81,55 75,53 Z" fill="${PH}"/>
    <circle cx="62" cy="35" r="3.6" fill="${PH}"/><circle cx="62" cy="35" r="1.5" fill="${BG}"/>
    <circle cx="69" cy="43" r="1.9" fill="${BG}"/>
    <rect x="77.5" y="52.5" width="3.4" height="4.6" rx="0.6" fill="${PH}"/>
    <line x1="79.2" y1="52.5" x2="79.2" y2="57.1" stroke="${BG}" stroke-width="0.7"/>
    <ellipse cx="58" cy="74" rx="4.2" ry="3" fill="${PH}"/>` },

  dukedom: { adj: 1.05, svg: `
    <polygon points="16,60 25,30 39,52 50,22 61,52 75,30 84,60" fill="${PH}"/>
    <rect x="16" y="59" width="68" height="25" rx="2" fill="${PH}"/>
    <circle cx="25" cy="28" r="4.5" fill="${PH}"/>
    <circle cx="50" cy="20" r="5" fill="${PH}"/>
    <circle cx="75" cy="28" r="4.5" fill="${PH}"/>
    <g fill="${BG}">
      <circle cx="30" cy="71" r="4"/><circle cx="50" cy="71" r="4.6"/><circle cx="70" cy="71" r="4"/>
    </g>
    <rect x="16" y="80" width="68" height="4" fill="${BG}"/>
    <rect x="16" y="80" width="68" height="1.6" fill="${PH}"/>` },

  'santa-paravia': { adj: 1.0, svg: `
    <rect x="49.2" y="13" width="1.8" height="20" fill="${PH}"/>
    <polygon points="51,13 62,16.5 51,20" fill="${PH}"/>
    <rect x="40" y="31" width="20" height="53" fill="${PH}"/>
    <rect x="40" y="25" width="5" height="7" fill="${PH}"/>
    <rect x="47.5" y="25" width="5" height="7" fill="${PH}"/>
    <rect x="55" y="25" width="5" height="7" fill="${PH}"/>
    <rect x="46.5" y="40" width="7" height="9" rx="1" fill="${BG}"/>
    <path d="M44.5,84 L44.5,72 Q50,66 55.5,72 L55.5,84 Z" fill="${BG}"/>
    <rect x="12" y="60" width="28" height="24" fill="${PH}"/>
    <rect x="12" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="19" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="26" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="33" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="60" y="60" width="28" height="24" fill="${PH}"/>
    <rect x="62" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="69" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="76" y="55" width="5" height="6" fill="${PH}"/>
    <rect x="83" y="55" width="5" height="6" fill="${PH}"/>
    <g fill="${BG}">
      <rect x="22" y="67" width="4" height="6"/><rect x="30" y="67" width="4" height="6"/>
      <rect x="66" y="67" width="4" height="6"/><rect x="74" y="67" width="4" height="6"/>
    </g>` },

  // Fort Nash: a frontier log stockade — pointed palisade logs flanking a
  // gatehouse with a peaked roof and a flag, the fort raised at journey's end.
  'fort-nash': { adj: 1.0, svg: `
    <rect x="49.2" y="10" width="1.6" height="13" fill="${PH}"/>
    <polygon points="50.8,10 61,13.5 50.8,17.5" fill="${PH}"/>
    <g fill="${PH}">
      <rect x="13" y="50" width="6" height="36"/><polygon points="13,50 16,44 19,50"/>
      <rect x="20" y="50" width="6" height="36"/><polygon points="20,50 23,44 26,50"/>
      <rect x="27" y="50" width="6" height="36"/><polygon points="27,50 30,44 33,50"/>
      <rect x="34" y="50" width="6" height="36"/><polygon points="34,50 37,44 40,50"/>
      <rect x="60" y="50" width="6" height="36"/><polygon points="60,50 63,44 66,50"/>
      <rect x="67" y="50" width="6" height="36"/><polygon points="67,50 70,44 73,50"/>
      <rect x="74" y="50" width="6" height="36"/><polygon points="74,50 77,44 80,50"/>
      <rect x="81" y="50" width="6" height="36"/><polygon points="81,50 84,44 87,50"/>
    </g>
    <polygon points="38,34 50,20 62,34" fill="${PH}"/>
    <rect x="40" y="33" width="20" height="53" fill="${PH}"/>
    <path d="M45.5,86 L45.5,64 Q50,58 54.5,64 L54.5,86 Z" fill="${BG}"/>
    <rect x="46.5" y="42" width="7" height="8" rx="1" fill="${BG}"/>` },

  // Colossal Cave: the brass lamp that lights the dark, a glowing flame inside.
  adventure: { adj: 1.0, svg: `
    <path d="M39,24 Q50,9 61,24" stroke="${PH}" stroke-width="3.2" fill="none" stroke-linecap="round"/>
    <polygon points="38,25 62,25 58,33 42,33" fill="${PH}"/>
    <rect x="33" y="33" width="34" height="44" rx="6" fill="${PH}"/>
    <rect x="40" y="41" width="20" height="29" rx="3" fill="${BG}"/>
    <path d="M50,46 Q57,55 50,67 Q43,55 50,46 Z" fill="${PH}"/>
    <polygon points="35,77 65,77 61,85 39,85" fill="${PH}"/>` },

  // Kaintuck: a flatboat riding the current — a long low hull with a deck cabin
  // and the steering sweep raked off the stern, over a line of water.
  kaintuck: { adj: 1.0, svg: `
    <path d="M12,84 Q23,81 34,84 T56,84 T78,84 T90,84" stroke="${DIM}" stroke-width="1.6" fill="none" stroke-linecap="round"/>
    <polygon points="11,64 21,56 75,56 85,64 80,74 16,74" fill="${PH}"/>
    <line x1="19" y1="62" x2="77" y2="62" stroke="${BG}" stroke-width="1.2"/>
    <rect x="33" y="39" width="31" height="18" rx="1.5" fill="${PH}"/>
    <g stroke="${BG}" stroke-width="1.3">
      <line x1="40" y1="39" x2="40" y2="57"/><line x1="48.5" y1="39" x2="48.5" y2="57"/><line x1="57" y1="39" x2="57" y2="57"/>
    </g>
    <line x1="76" y1="58" x2="90" y2="40" stroke="${PH}" stroke-width="3" stroke-linecap="round"/>
    <polygon points="87,36 93,40 89,45" fill="${PH}"/>` },
};

function buildSvg(name, mode, size) {
  const g = GLYPHS[name];
  const cover = (mode === 'any' ? 0.74 : mode === 'apple' ? 0.66 : 0.60) * g.adj;
  const scale = cover / 0.80;
  const filter = `<filter id="glow" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur in="SourceGraphic" stdDeviation="1.1" result="b"/>
      <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>`;
  const bg = mode === 'any'
    ? `<rect x="3" y="3" width="94" height="94" rx="20" ry="20" fill="${BG}"/>
       <rect x="4.6" y="4.6" width="90.8" height="90.8" rx="18" ry="18" fill="none" stroke="${DIM}" stroke-width="1.3"/>`
    : `<rect width="100" height="100" fill="${BG}"/>`;
  const dim = size ? `width="${size}" height="${size}"` : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" ${dim} viewBox="0 0 100 100">
  <defs>${filter}</defs>
  ${bg}
  <g filter="url(#glow)" transform="translate(50 50) scale(${scale.toFixed(4)}) translate(-50 -50)">${g.svg}</g>
</svg>`;
}

const OUTPUTS = [
  ['icon-192.png', 'any', 192], ['icon-512.png', 'any', 512],
  ['icon-192-maskable.png', 'maskable', 192], ['icon-512-maskable.png', 'maskable', 512],
  ['apple-touch-icon.png', 'apple', 180],
];

const browser = await chromium.launch({ executablePath: CHROMIUM, args: ['--no-sandbox'] });

async function renderPng(svg, size, opaque) {
  const ctx = await browser.newContext({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  await page.setContent(`<!doctype html><meta charset=utf8><style>html,body{margin:0}svg{display:block}</style>${svg}`, { waitUntil: 'networkidle' });
  const buf = await page.screenshot({ omitBackground: !opaque });
  await ctx.close();
  return buf;
}

// All render work runs inside try/finally so a thrown error (malformed glyph
// SVG, unwritable dir, a screenshot that never settles) still closes the
// headless Chromium — otherwise the leaked browser process outlives the
// failed `node scripts/gen-icons.mjs` run.
try {
for (const name of Object.keys(GLYPHS)) {
  const dir = resolve(GAMES_DIR, name, 'assets');
  mkdirSync(dir, { recursive: true });
  writeFileSync(resolve(dir, 'icon.svg'), buildSvg(name, 'any', null));
  for (const [file, mode, size] of OUTPUTS) {
    writeFileSync(resolve(dir, file), await renderPng(buildSvg(name, mode, size), size, mode !== 'any'));
  }
  console.log(`icons -> games/${name}/assets/`);
}

// Visual QA: a contact sheet of every game's any / maskable (with the Android
// adaptive crop circle overlaid) / apple-touch icon on grey, so a new glyph can
// be eyeballed for legibility-at-size and safe-zone fit before shipping. Reads
// the just-written PNGs and inlines them as data URLs (origin-independent).
if (SHEET) {
  const games = Object.keys(GLYPHS);
  const cell = 128, pad = 14, rowH = cell + pad + 18;
  const dataUrl = (name, file) =>
    `data:image/png;base64,${readFileSync(resolve(GAMES_DIR, name, 'assets', file)).toString('base64')}`;
  const cellHtml = (name, file, cap, crop = false) =>
    `<div class="cell"><div class="img"><img src="${dataUrl(name, file)}">${
      crop ? '<div class="crop"></div>' : ''
    }</div><div class="cap">${cap}</div></div>`;
  const rows = games
    .map(
      (name) =>
        `<div class="row"><div class="name">${name}</div>` +
        cellHtml(name, 'icon-512.png', 'any') +
        cellHtml(name, 'icon-512-maskable.png', 'maskable', true) +
        cellHtml(name, 'apple-touch-icon.png', 'apple') +
        `</div>`
    )
    .join('');
  const html = `<!doctype html><meta charset=utf8><style>
    body{margin:0;background:#3a3a3a;font:12px monospace;color:#ddd;padding:10px}
    .row{display:flex;align-items:center;gap:${pad}px;padding:6px ${pad}px}
    .name{width:96px;color:#9f9;font-weight:bold}
    .cell{width:${cell}px;text-align:center}
    .img{width:${cell}px;height:${cell}px;position:relative}
    .img img{width:100%;height:100%;display:block}
    .crop{position:absolute;inset:0;border-radius:50%;box-shadow:0 0 0 2px #ff3b3b inset}
    .cap{font-size:10px;color:#aaa;margin-top:2px}
  </style>${rows}`;
  const W = 96 + pad * 2 + 3 * (cell + pad) + 40;
  const H = games.length * rowH + 30;
  const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  await page.setContent(html, { waitUntil: 'networkidle' });
  await page.screenshot({ path: '/tmp/icon-contact-sheet.png' });
  await ctx.close();
  console.log('contact sheet -> /tmp/icon-contact-sheet.png');
}
} finally {
  await browser.close();
}
