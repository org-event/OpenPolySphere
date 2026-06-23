#!/usr/bin/env bun
/**
 * Merge settings i18n patches into web/static/locales/*.json
 *
 * Workflow when adding UI strings:
 *   1. Add keys to scripts/i18n-settings-en.json
 *   2. Translate in scripts/i18n-settings-ru.json (and other *-{locale}.json overlays later)
 *   3. bun run i18n:merge-settings
 *
 * - en.json: full overlay (source of truth for new keys)
 * - other locales: missing keys only (keeps existing translations)
 * - ru.json: also applies i18n-settings-ru.json overlay
 */

import { readdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const LOCALES = join(ROOT, 'web/static/locales');
const EN_OVERLAY = join(ROOT, 'scripts/i18n-settings-en.json');
const RU_OVERLAY = join(ROOT, 'scripts/i18n-settings-ru.json');

function clone(value) {
  return structuredClone(value);
}

function deepMergeMissing(dst, src) {
  for (const [key, value] of Object.entries(src)) {
    if (!(key in dst)) {
      dst[key] = clone(value);
    } else if (value && typeof value === 'object' && !Array.isArray(value) && dst[key] && typeof dst[key] === 'object') {
      deepMergeMissing(dst[key], value);
    }
  }
}

function deepMerge(dst, src) {
  for (const [key, value] of Object.entries(src)) {
    if (value && typeof value === 'object' && !Array.isArray(value) && dst[key] && typeof dst[key] === 'object') {
      deepMerge(dst[key], value);
    } else {
      dst[key] = clone(value);
    }
  }
}

const enOverlay = JSON.parse(await readFile(EN_OVERLAY, 'utf8'));
const ruOverlay = JSON.parse(await readFile(RU_OVERLAY, 'utf8'));

const files = (await readdir(LOCALES)).filter((f) => f.endsWith('.json')).sort();

for (const file of files) {
  const code = file.replace(/\.json$/, '');
  const path = join(LOCALES, file);
  const data = JSON.parse(await readFile(path, 'utf8'));

  if (code === 'en') {
    deepMerge(data, enOverlay);
  } else {
    deepMergeMissing(data, enOverlay);
    if (code === 'ru') deepMerge(data, ruOverlay);
  }

  await writeFile(path, `${JSON.stringify(data, null, 2)}\n`, 'utf8');
  console.log(`updated ${file}`);
}
