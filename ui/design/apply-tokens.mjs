#!/usr/bin/env bun
/**
 * Rebuild Foundations + Tokens pages from ui/design/tokens.json.
 * Usage: bun ui/design/apply-tokens.mjs
 */
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const tokens = JSON.parse(readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'tokens.json'), 'utf8'));
const fig = join(root, 'ui/design/openpolysphere.fig');
const openpencil = process.env.OPENPENCIL_CLI ?? `${process.env.HOME}/.bun/bin/openpencil`;

const code = `
const TOKENS = ${JSON.stringify(tokens)};

for (const p of [...figma.root.children]) p.remove();

await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
await figma.loadFontAsync({ family: 'Inter', style: 'Medium' });
await figma.loadFontAsync({ family: 'Inter', style: 'Semi Bold' });
await figma.loadFontAsync({ family: 'Inter', style: 'Bold' });

function parseColor(hex) {
  if (hex.length === 9) {
    const n = parseInt(hex.slice(1, 7), 16);
    const a = parseInt(hex.slice(7, 9), 16) / 255;
    return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255, a];
  }
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255, 1];
}

function solid(hex) {
  const [r, g, b, a] = parseColor(hex);
  return [{ type: 'SOLID', color: { r, g, b, a }, opacity: 1, visible: true, blendMode: 'NORMAL' }];
}

const C = {
  light: Object.fromEntries(Object.entries(TOKENS.color.light).map(([k, v]) => [k, parseColor(v)])),
  dark: Object.fromEntries(Object.entries(TOKENS.color.dark).map(([k, v]) => [k, parseColor(v)])),
};

let art = null;

function mountPage(name, w, h, bgHex) {
  const page = figma.createPage();
  page.name = name;
  figma.currentPage = page;
  art = figma.createFrame();
  art.name = 'Artboard / ' + name;
  art.x = 0;
  art.y = 0;
  art.resize(w, h);
  art.fills = solid(bgHex);
  art.layoutMode = 'NONE';
  art.clipsContent = false;
  page.appendChild(art);
  return art;
}

function box(name, x, y, w, h, hex, r = 0) {
  const f = figma.createFrame();
  f.name = name;
  f.x = x;
  f.y = y;
  f.resize(w, h);
  f.fills = solid(hex);
  f.cornerRadius = r;
  f.layoutMode = 'NONE';
  art.appendChild(f);
  return f;
}

function stroke(f, hex, weight = 1) {
  f.strokes = solid(hex);
  f.strokeWeight = weight;
  f.strokeAlign = 'INSIDE';
}

function txt(name, s, x, y, size, hex, weight = 'Regular', width = null, align = 'LEFT') {
  const t = figma.createText();
  t.name = name;
  t.x = x;
  t.y = y;
  t.characters = s;
  t.fontSize = size;
  t.fontName = { family: 'Inter', style: weight };
  t.fills = solid(hex);
  if (width) {
    t.resize(width, size + 6);
    t.textAutoResize = 'HEIGHT';
    t.textAlignHorizontal = align;
  } else {
    t.textAutoResize = 'WIDTH_AND_HEIGHT';
  }
  art.appendChild(t);
  return t;
}

// Foundations
mountPage('Foundations', 1280, 1180, TOKENS.color.light['bg/base']);
let y = 48;
txt('title', 'Foundations', 48, y, TOKENS.typography.display.size, TOKENS.color.light['text/primary'], TOKENS.typography.display.weight);
txt('subtitle', 'From tokens.json — edit brand book → bun ui/design/apply-tokens.mjs', 48, y + 42, TOKENS.typography.ui.size, TOKENS.color.light['text/secondary']);
y = 120;
txt('section / spacing', 'Spacing', 48, y, 13, TOKENS.color.light['text/muted'], 'Semi Bold');
TOKENS.spacing.scale.forEach((s, i) => {
  const x = 48 + i * 160;
  box('spacing / ' + s + 'px', x, y + 28, s, 40, TOKENS.color.light['accent/calm'], TOKENS.radius.sm);
  txt('spacing / ' + s + 'px / label', s + 'px', x, y + 76, 12, TOKENS.color.light['text/secondary'], 'Medium');
});
y += 120;
[
  ['sm', TOKENS.radius.sm],
  ['md', TOKENS.radius.md],
  ['lg', TOKENS.radius.lg],
  ['pill', TOKENS.radius.pill],
].forEach(([name, r], i) => {
  const x = 48 + i * 200;
  const w = name === 'pill' ? 120 : 80;
  const rf = box('radius / ' + name, x, y, w, 56, TOKENS.color.light['bg/elevated'], r);
  stroke(rf, TOKENS.color.light['border/subtle']);
  txt('radius / ' + name + ' / label', name, x, y + 68, 12, TOKENS.color.light['text/secondary']);
});
y += 110;
txt('type / display', 'Call in progress', 48, y, TOKENS.typography.display.size, TOKENS.color.light['text/primary'], TOKENS.typography.display.weight);
txt('type / body', 'Translation reads large and calm.', 48, y + 72, TOKENS.typography.body.size, TOKENS.color.light['text/primary']);
txt('type / ui', 'Mic Out · Connected', 48, y + 132, TOKENS.typography.ui.size, TOKENS.color.light['text/secondary'], TOKENS.typography.ui.weight);
txt('type / meta', '627ms · STT 142', 48, y + 186, TOKENS.typography.meta.size, TOKENS.color.light['text/muted']);

// Tokens page
mountPage('Tokens', 1280, 1100, TOKENS.color.light['bg/base']);
txt('tokens / title', 'Color Tokens', 48, 48, 32, TOKENS.color.light['text/primary'], 'Bold');
txt('tokens / subtitle', 'Bound to tokens.json — change file, rerun apply-tokens.mjs', 48, 90, 14, TOKENS.color.light['text/secondary']);

function swatches(theme, colX, labelHex) {
  txt('tokens / ' + theme + ' / heading', theme === 'light' ? 'Light' : 'Dark', colX, 140, 20, labelHex, 'Semi Bold');
  Object.entries(TOKENS.color[theme]).forEach(([token, hex], i) => {
    const row = 180 + i * 72;
    const sw = box('token / ' + theme + ' / ' + token, colX, row, 120, 48, hex, TOKENS.radius.md);
    if (token.includes('border')) stroke(sw, hex);
    txt('token / ' + theme + ' / ' + token + ' / name', token, colX + 136, row + 8, 13, labelHex, 'Medium');
    txt('token / ' + theme + ' / ' + token + ' / hex', hex, colX + 136, row + 26, 11, TOKENS.color.light['text/muted']);
  });
}

swatches('light', 48, TOKENS.color.light['text/primary']);
const dp = box('tokens / dark / panel', 640, 120, 592, 920, TOKENS.color.dark['bg/base'], TOKENS.radius.lg);
stroke(dp, TOKENS.color.dark['border/subtle']);
swatches('dark', 688, TOKENS.color.dark['text/primary']);

figma.currentPage = figma.root.children.find((p) => p.name === 'Foundations');
figma.viewport.scrollAndZoomIntoView([figma.currentPage.children[0]]);
`;

const r = spawnSync('bun', [openpencil, 'eval', fig, '-w', '--code', code], {
  cwd: root,
  encoding: 'utf8',
  stdio: ['pipe', 'pipe', 'pipe'],
});

if (r.status !== 0) {
  console.error(r.stderr || r.stdout);
  process.exit(r.status ?? 1);
}
console.log(r.stdout.trim() || 'OK');
console.log('Updated', fig, 'from tokens.json');
