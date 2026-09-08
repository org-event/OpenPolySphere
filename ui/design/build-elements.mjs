#!/usr/bin/env bun
/**
 * Phase 3 — rebuild Foundations + Tokens + Elements from tokens.json (single eval).
 * Usage: bun ui/design/build-elements.mjs
 */
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const dir = dirname(fileURLToPath(import.meta.url));
const root = join(dir, '../..');
const tokens = JSON.parse(readFileSync(join(dir, 'tokens.json'), 'utf8'));
const fig = join(root, 'ui/design/openpolysphere.fig');
const openpencil = process.env.OPENPENCIL_CLI ?? `${process.env.HOME}/.bun/bin/openpencil`;

const code = `
const TOKENS = ${JSON.stringify(tokens)};

for (const p of [...figma.root.children]) p.remove();

await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
await figma.loadFontAsync({ family: 'Inter', style: 'Medium' });
await figma.loadFontAsync({ family: 'Inter', style: 'Semi Bold' });
await figma.loadFontAsync({ family: 'Inter', style: 'Bold' });
await figma.loadFontAsync({ family: 'Inter', style: 'Italic' });

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

const L = TOKENS.color.light;
const D = TOKENS.color.dark;
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
    t.resize(width, size + 8);
    t.textAutoResize = 'HEIGHT';
    t.textAlignHorizontal = align;
  } else {
    t.textAutoResize = 'WIDTH_AND_HEIGHT';
  }
  art.appendChild(t);
  return t;
}

function section(title, y) {
  txt('section / ' + title, title, 48, y, 13, L['text/muted'], 'Semi Bold');
  box('section / ' + title + ' / rule', 48, y + 22, 1184, 1, L['border/subtle']);
  return y + 48;
}

// ── Foundations ─────────────────────────────────────────────────────────
mountPage('Foundations', 1280, 1180, L['bg/base']);
let y = 48;
txt('title', 'Foundations', 48, y, TOKENS.typography.display.size, L['text/primary'], TOKENS.typography.display.weight);
txt('subtitle', 'tokens.json → bun ui/design/build-elements.mjs', 48, y + 42, TOKENS.typography.ui.size, L['text/secondary']);
y = 120;
txt('section / spacing', 'Spacing', 48, y, 13, L['text/muted'], 'Semi Bold');
TOKENS.spacing.scale.forEach((s, i) => {
  const x = 48 + i * 160;
  box('spacing / ' + s + 'px', x, y + 28, s, 40, L['accent/calm'], TOKENS.radius.sm);
  txt('spacing / ' + s + 'px / label', s + 'px', x, y + 76, 12, L['text/secondary'], 'Medium');
});
y += 120;
[['sm', TOKENS.radius.sm], ['md', TOKENS.radius.md], ['lg', TOKENS.radius.lg], ['pill', TOKENS.radius.pill]].forEach(([name, r], i) => {
  const x = 48 + i * 200;
  const w = name === 'pill' ? 120 : 80;
  const rf = box('radius / ' + name, x, y, w, 56, L['bg/elevated'], r);
  stroke(rf, L['border/subtle']);
  txt('radius / ' + name + ' / label', name, x, y + 68, 12, L['text/secondary']);
});
y += 110;
txt('type / display', 'Call in progress', 48, y, TOKENS.typography.display.size, L['text/primary'], TOKENS.typography.display.weight);
txt('type / body', 'Translation reads large and calm.', 48, y + 72, TOKENS.typography.body.size, L['text/primary']);
txt('type / ui', 'Mic Out · Connected', 48, y + 132, TOKENS.typography.ui.size, L['text/secondary'], TOKENS.typography.ui.weight);
txt('type / meta', '627ms · STT 142', 48, y + 186, TOKENS.typography.meta.size, L['text/muted']);

// ── Tokens ─────────────────────────────────────────────────────────────
mountPage('Tokens', 1280, 1100, L['bg/base']);
txt('tokens / title', 'Color Tokens', 48, 48, 32, L['text/primary'], 'Bold');
txt('tokens / subtitle', 'Bound to tokens.json', 48, 90, 14, L['text/secondary']);
function swatches(theme, colX, labelHex) {
  txt('tokens / ' + theme + ' / heading', theme === 'light' ? 'Light' : 'Dark', colX, 140, 20, labelHex, 'Semi Bold');
  Object.entries(TOKENS.color[theme]).forEach(([token, hex], i) => {
    const row = 180 + i * 72;
    const sw = box('token / ' + theme + ' / ' + token, colX, row, 120, 48, hex, TOKENS.radius.md);
    if (token.includes('border')) stroke(sw, hex);
    txt('token / ' + theme + ' / ' + token + ' / name', token, colX + 136, row + 8, 13, labelHex, 'Medium');
    txt('token / ' + theme + ' / ' + token + ' / hex', hex, colX + 136, row + 26, 11, L['text/muted']);
  });
}
swatches('light', 48, L['text/primary']);
const dp = box('tokens / dark / panel', 640, 120, 592, 920, D['bg/base'], TOKENS.radius.lg);
stroke(dp, D['border/subtle']);
swatches('dark', 688, D['text/primary']);

// ── Elements ───────────────────────────────────────────────────────────
mountPage('Elements', 1280, 1320, L['bg/base']);
y = 48;
txt('title', 'Elements', 48, y, 32, L['text/primary'], 'Bold');
txt('subtitle', 'Atoms — dividers, dots, type, icons, focus', 48, y + 42, 14, L['text/secondary']);
y = 120;

y = section('Divider / hairline', y);
txt('element / divider / label-light', 'Light — border/subtle · 1px', 48, y, 12, L['text/secondary']);
box('element / divider / hairline-light', 48, y + 24, 400, 1, L['border/subtle']);
txt('element / divider / label-dark', 'Dark preview', 520, y, 12, L['text/secondary']);
const darkStrip = box('element / divider / dark-bg', 520, y + 16, 400, 40, D['bg/elevated'], TOKENS.radius.sm);
stroke(darkStrip, D['border/subtle']);
box('element / divider / hairline-dark', 536, y + 35, 368, 1, D['border/subtle']);
y += 80;

y = section('Dot / status LED', y);
[['live', L['semantic/live'], 'semantic/live'], ['danger', L['semantic/danger'], 'semantic/danger'], ['muted', L['text/muted'], 'text/muted'], ['brand-open', L['brand/open'], 'brand/open']].forEach(([name, hex, token], i) => {
  const x = 48 + i * 140;
  box('element / dot / ' + name, x, y, 8, 8, hex, 4);
  txt('element / dot / ' + name + ' / label', token, x, y + 16, 11, L['text/muted']);
});
y += 56;

y = section('Text styles', y);
const ty = TOKENS.typography;
txt('element / type / display', 'Display — Call in progress', 48, y, ty.display.size, L['text/primary'], ty.display.weight);
txt('element / type / display / meta', 'type/display · ' + ty.display.size + 'px', 48, y + 40, 11, L['text/muted']);
y += 80;
txt('element / type / body', 'Body — Меня хорошо слышно? Translation is calm.', 48, y, ty.body.size, L['text/primary'], ty.body.weight, 680);
txt('element / type / body / meta', 'type/body · ' + ty.body.size + 'px · lh ' + ty.body.lineHeight, 48, y + 52, 11, L['text/muted']);
y += 88;
txt('element / type / ui', 'UI — Mic Out · Connected', 48, y, ty.ui.size, L['text/secondary'], ty.ui.weight);
txt('element / type / ui / meta', 'type/ui · ' + ty.ui.size + 'px', 48, y + 24, 11, L['text/muted']);
y += 56;
txt('element / type / meta', 'Meta — 627ms · STT 142', 48, y, ty.meta.size, L['text/muted'], ty.meta.weight);
txt('element / type / meta / css', 'ui-monospace in CSS', 48, y + 20, 11, L['text/muted']);
y += 56;
txt('element / type / original', 'Original — Can you hear me clearly?', 48, y, ty.body.size - 2, L['text/secondary'], 'Italic', 680);
y += 64;

y = section('Icon slot (SVG in code)', y);
[16, 20, 24].forEach((size, i) => {
  const x = 48 + i * 160;
  const slot = box('element / icon-slot / ' + size, x, y, size, size, L['bg/elevated'], TOKENS.radius.sm);
  stroke(slot, L['border/subtle']);
  txt('element / icon-slot / ' + size + ' / label', size + '×' + size, x, y + size + 8, 11, L['text/muted']);
});
y += 88;

y = section('Focus ring (CSS spec)', y);
const fw = TOKENS.focus.width;
const fo = TOKENS.focus.offset;
const focusHex = L[TOKENS.focus.colorToken];
const input = box('element / focus / input', 48, y, 240, 44, L['bg/elevated'], TOKENS.radius.sm);
stroke(input, L['border/subtle']);
txt('element / focus / input / text', 'Focused field', 64, y + 13, 14, L['text/primary']);
const ring = box('element / focus / ring', 48 - fo, y - fo, 240 + fo * 2, 44 + fo * 2, L['bg/base'], TOKENS.radius.sm + fo);
stroke(ring, focusHex, fw);
txt('element / focus / spec', 'focus: ' + fw + 'px ' + TOKENS.focus.colorToken + ', offset ' + fo + 'px', 48, y + 60, 12, L['text/muted']);
y += 100;

y = section('Surface + border (theme pair)', y);
const lightCard = box('element / surface / light', 48, y, 280, 100, L['bg/elevated'], TOKENS.radius.lg);
stroke(lightCard, L['border/subtle']);
txt('element / surface / light / label', 'bg/elevated + border/subtle', 64, y + 42, 12, L['text/secondary']);
const darkCard = box('element / surface / dark', 360, y, 280, 100, D['bg/elevated'], TOKENS.radius.lg);
stroke(darkCard, D['border/subtle']);
txt('element / surface / dark / label', 'dark pair', 376, y + 42, 12, D['text/secondary']);

figma.currentPage = figma.root.children.find((p) => p.name === 'Elements');
figma.viewport.scrollAndZoomIntoView([figma.currentPage.children[0]]);
`;

const r = spawnSync('bun', [openpencil, 'eval', fig, '-w', '--code', code], {
  cwd: root,
  encoding: 'utf8',
});

if (r.status !== 0) {
  console.error(r.stderr || r.stdout);
  process.exit(r.status ?? 1);
}
console.log(r.stdout.trim() || 'OK');
console.log('Foundations + Tokens + Elements rebuilt from tokens.json');
