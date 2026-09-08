#!/usr/bin/env bun
/**
 * Phase 4 — rebuild Foundations + Tokens + Elements + Patterns from tokens.json.
 * Usage: bun ui/design/build-patterns.mjs
 */
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const dir = dirname(fileURLToPath(import.meta.url));
const root = join(dir, '../..');
const tokens = JSON.parse(readFileSync(join(dir, 'tokens.json'), 'utf8'));
const logoPng = readFileSync(join(dir, 'openpolysphere-tree-icon.png'));
const fig = join(root, 'ui/design/openpolysphere.fig');
const openpencil = process.env.OPENPENCIL_CLI ?? `${process.env.HOME}/.bun/bin/openpencil`;

const code = `
const TOKENS = ${JSON.stringify(tokens)};
const LOGO_PNG = new Uint8Array(${JSON.stringify([...logoPng])});

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

/** Label in a box — full container width, fontSize height (OpenPencil default text is 100×100). */
function lbl(name, x, y, w, h, s, size, hex, weight = 'Medium', alignH = 'CENTER') {
  const t = figma.createText();
  t.name = name;
  t.fontSize = size;
  t.fontName = { family: 'Inter', style: weight };
  t.fills = solid(hex);
  t.characters = s;
  t.textAutoResize = 'NONE';
  t.resize(w, size);
  t.textAlignHorizontal = alignH;
  t.x = x;
  t.y = y + Math.round((h - size) / 2);
  art.appendChild(t);
  return t;
}

function placeLogo(name, x, y, size) {
  const f = figma.createFrame();
  f.name = name;
  f.x = x;
  f.y = y;
  f.resize(size, size);
  f.fills = [];
  const img = figma.createImage(LOGO_PNG);
  f.fills = [{ type: 'IMAGE', scaleMode: 'FIT', imageHash: img.hash }];
  art.appendChild(f);
  return f;
}

/** Single-line body text vertically centered (bubbles). */
function vtxt(name, x, y, w, h, s, size, hex, weight = 'Regular') {
  const t = figma.createText();
  t.name = name;
  t.fontSize = size;
  t.fontName = { family: 'Inter', style: weight };
  t.fills = solid(hex);
  t.characters = s;
  t.textAutoResize = 'WIDTH_AND_HEIGHT';
  art.appendChild(t);
  const th = size;
  t.textAutoResize = 'NONE';
  t.resize(w, th);
  t.y = y + Math.round((h - th) / 2);
  t.x = x;
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

// ── Patterns ───────────────────────────────────────────────────────────
mountPage('Patterns', 1280, 1680, L['bg/base']);
y = 48;
txt('title', 'Patterns', 48, y, 32, L['text/primary'], 'Bold');
txt('subtitle', 'Larger UI chunks — Granola calm, content-first', 48, y + 42, 14, L['text/secondary']);
y = 110;

const m = TOKENS.spacing.screenMargin;

function patternCard(name, x, py, w, h) {
  const c = box('pattern / ' + name + ' / card', x, py, w, h, L['bg/elevated'], TOKENS.radius.lg);
  stroke(c, L['border/subtle']);
  return py + 16;
}

// ── Top bar (44px airy) ──────────────────────────────────────────────────
y = section('Top bar', y);
const topY = y;
const topH = 44;
box('pattern / top-bar / bg', m, topY, 1280 - m * 2, topH, L['bg/elevated'], 0);
box('pattern / top-bar / rule', m, topY + topH - 1, 1280 - m * 2, 1, L['border/subtle']);
placeLogo('pattern / top-bar / logo', m + 12, topY + 8, 28);
txt('pattern / top-bar / open', 'Open', m + 48, topY + 6, 15, L['brand/open'], 'Bold');
txt('pattern / top-bar / poly', 'PolySphere', m + 48, topY + 22, 15, L['brand/poly'], 'Bold');
const settings = box('pattern / top-bar / settings', 1280 - m - 44, topY + 6, 32, 32, L['bg/base'], TOKENS.radius.sm);
stroke(settings, L['border/subtle']);
txt('pattern / top-bar / settings / icon', '⚙', 1280 - m - 34, topY + 12, 14, L['text/muted']);
y = topY + topH + 40;

// ── Language chip ──────────────────────────────────────────────────────
y = section('Language chip', y);
const chipY = y;
const chip = box('pattern / lang-chip', m, chipY, 152, 36, L['bg/elevated'], TOKENS.radius.pill);
stroke(chip, L['border/subtle']);
box('pattern / lang-chip / dot-en', m + 16, chipY + 12, 10, 10, L['accent/bubble-out'], 5);
lbl('pattern / lang-chip / en', m + 28, chipY, 36, 36, 'EN', 13, L['text/primary'], 'Semi Bold', 'CENTER');
lbl('pattern / lang-chip / arrow', m + 58, chipY, 28, 36, '↔', 14, L['text/muted'], 'Regular', 'CENTER');
box('pattern / lang-chip / dot-ru', m + 84, chipY + 12, 10, 10, L['accent/bubble-in'], 5);
lbl('pattern / lang-chip / ru', m + 96, chipY, 40, 36, 'RU', 13, L['text/primary'], 'Semi Bold', 'CENTER');
y = chipY + 56;

// ── Session strip ────────────────────────────────────────────────────────
y = section('Session strip', y);
const sessY = y;
box('pattern / session / dot', m, sessY + 6, 8, 8, L['semantic/live'], 4);
const livePill = box('pattern / session / live-pill', m + 16, sessY, 72, 28, L['bg/elevated'], TOKENS.radius.pill);
stroke(livePill, L['border/subtle']);
lbl('pattern / session / live', m + 16, sessY, 72, 28, 'Live', 12, L['semantic/live'], 'Medium');
lbl('pattern / session / timer', m + 96, sessY, 56, 28, '02:34', 13, L['text/secondary'], 'Medium', 'LEFT');
lbl('pattern / session / conn', m + 160, sessY, 96, 28, 'Connected', 13, L['semantic/live'], 'Medium', 'LEFT');
y = sessY + 48;

// ── Message stack ────────────────────────────────────────────────────────
y = section('Message stack', y);
const colX = Math.round((1280 - TOKENS.spacing.maxReadWidth) / 2);
const colW = TOKENS.spacing.maxReadWidth;
let msgY = y;

function messageBubble(name, x, py, w, trans, orig, outgoing) {
  const bh = 52;
  const bg = outgoing ? L['bg/bubble-out'] : L['bg/bubble-in'];
  const accent = outgoing ? L['accent/bubble-out'] : L['accent/bubble-in'];
  const b = box('pattern / msg / ' + name, x, py, w, bh, bg, TOKENS.radius.lg);
  stroke(b, L['border/subtle']);
  box('pattern / msg / ' + name + ' / accent', x, py, 4, bh, accent, 2);
  vtxt('pattern / msg / ' + name + ' / trans', x + 16, py, w - 32, bh, trans, ty.body.size, L['text/primary'], ty.body.weight);
  txt('pattern / msg / ' + name + ' / orig', orig, x, py + bh + 6, ty.body.size - 2, L['text/secondary'], 'Italic', w, outgoing ? 'RIGHT' : 'LEFT');
  return py + bh + 36;
}

msgY = messageBubble('out-1', colX + colW - 420, msgY, 420, 'Меня хорошо слышно?', 'Can you hear me clearly?', true);
msgY = messageBubble('in-1', colX, msgY + 16, 400, 'Да, отлично слышно.', 'Yes, perfectly fine.', false);
msgY = messageBubble('out-2', colX + colW - 440, msgY + 16, 440, 'Отправлю документ после звонка.', "I'll send the document after the call.", true);
const typing = box('pattern / msg / typing', colX, msgY + 24, 88, 32, L['bg/elevated'], TOKENS.radius.pill);
stroke(typing, L['border/subtle']);
box('pattern / msg / typing / d1', colX + 20, msgY + 36, 6, 6, L['semantic/live'], 3);
box('pattern / msg / typing / d2', colX + 32, msgY + 36, 6, 6, L['text/muted'], 3);
box('pattern / msg / typing / d3', colX + 44, msgY + 36, 6, 6, L['text/muted'], 3);
y = msgY + 80;

// ── Pill dock (Granola command bar) ────────────────────────────────────
y = section('Pill dock', y);
const dockW = 720;
const dockX = Math.round((1280 - dockW) / 2);
const dockY = y;
const dockH = 60;
const dock = box('pattern / dock', dockX, dockY, dockW, dockH, L['bg/elevated'], TOKENS.radius.pill);
stroke(dock, L['border/subtle']);
const stop = box('pattern / dock / stop', dockX + 16, dockY + 12, 100, 36, L['semantic/danger'], TOKENS.radius.md);
lbl('pattern / dock / stop / label', dockX + 16, dockY + 12, 100, 36, '■ Stop', 14, L['bg/elevated'], 'Semi Bold');
const micOn = box('pattern / dock / mic-out', dockX + 128, dockY + 12, 44, 36, L['semantic/live'], TOKENS.radius.sm);
stroke(micOn, L['semantic/live']);
const micOff = box('pattern / dock / mic-in', dockX + 180, dockY + 12, 44, 36, L['bg/base'], TOKENS.radius.sm);
stroke(micOff, L['border/subtle']);
const mon = box('pattern / dock / monitor', dockX + 240, dockY + 12, 108, 36, L['bg/base'], TOKENS.radius.sm);
stroke(mon, L['border/subtle']);
lbl('pattern / dock / monitor / label', dockX + 240, dockY + 12, 108, 36, 'Monitor', 13, L['text/primary'], 'Medium');
const more = box('pattern / dock / more', dockX + dockW - 52, dockY + 12, 36, 36, L['bg/base'], TOKENS.radius.sm);
stroke(more, L['border/subtle']);
lbl('pattern / dock / more / label', dockX + dockW - 52, dockY + 12, 36, 36, '···', 16, L['text/muted'], 'Regular');
y = dockY + dockH + 32;

// ── Metrics row (single line) ────────────────────────────────────────────
y = section('Metrics row', y);
txt('pattern / metrics', 'STT 142ms  ·  Translate 198ms  ·  TTS 287ms  ·  Total 627ms  ·  Phrases 12', m, y, ty.meta.size, L['text/muted'], ty.meta.weight, 1280 - m * 2, 'CENTER');
y += 48;

// ── Combined chrome preview ──────────────────────────────────────────────
y = section('Call chrome (composed)', y);
const prev = box('pattern / preview / frame', m, y, 1280 - m * 2, 320, L['bg/base'], TOKENS.radius.lg);
stroke(prev, L['accent/calm']);
txt('pattern / preview / label', 'Light call shell — patterns combined at 1:4 scale feel', m + 16, y + 12, 12, L['text/muted']);
box('pattern / preview / top', m + 16, y + 36, 1184, 32, L['bg/elevated']);
placeLogo('pattern / preview / top / logo', m + 24, y + 38, 24);
const prevChip = box('pattern / preview / chip', 1280 / 2 - 60, y + 80, 120, 28, L['bg/elevated'], TOKENS.radius.pill);
stroke(prevChip, L['border/subtle']);
box('pattern / preview / bubble-r', m + 700, y + 120, 200, 40, L['bg/bubble-out'], TOKENS.radius.md);
box('pattern / preview / bubble-l', m + 80, y + 170, 180, 40, L['bg/bubble-in'], TOKENS.radius.md);
const prevDock = box('pattern / preview / dock', 1280 / 2 - 200, y + 248, 400, 44, L['bg/elevated'], TOKENS.radius.pill);
stroke(prevDock, L['border/subtle']);

figma.currentPage = figma.root.children.find((p) => p.name === 'Patterns');
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
console.log('Foundations + Tokens + Elements + Patterns rebuilt from tokens.json');
