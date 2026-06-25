const TREE_ICON_URL = '/static/img/openpolysphere-tree-icon.svg?v=7';
const FULL_LOGO_URL = '/static/img/openpolysphere-full-logo.svg?v=1';
const LAYER_PHASE = [0, (Math.PI * 2) / 3, (Math.PI * 4) / 3];
const BREATHE_SPEED = 1.1;
const OPACITY_MIN = 0.72;
const OPACITY_MAX = 1;

let stopAnimation = null;

async function fetchSvg(url) {
  const res = await fetch(url);
  if (!res.ok) return null;
  const markup = await res.text();
  const doc = new DOMParser().parseFromString(markup, 'image/svg+xml');
  return doc.querySelector('svg');
}

function mountSvg(slot, svg, svgClass) {
  svg.classList.add(svgClass);
  svg.setAttribute('aria-hidden', 'true');
  slot.replaceChildren(svg);
}

function breatheLayers(layers, startedAt) {
  const tick = (now) => {
    const t = (now - startedAt) / 1000;
    layers.forEach((layer, i) => {
      const wave = 0.5 + 0.5 * Math.sin(t * BREATHE_SPEED + LAYER_PHASE[i % LAYER_PHASE.length]);
      layer.style.opacity = String(OPACITY_MIN + (OPACITY_MAX - OPACITY_MIN) * wave);
    });
    stopAnimation = requestAnimationFrame(tick);
  };
  stopAnimation = requestAnimationFrame(tick);
}

function animateLogoLayers(root) {
  const layers = [...root.querySelectorAll('.logo-layer, .tree-layer')];
  if (!layers.length) return;

  layers.forEach((layer) => {
    layer.style.opacity = '1';
  });

  if (stopAnimation) {
    cancelAnimationFrame(stopAnimation);
    stopAnimation = null;
  }

  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  breatheLayers(layers, performance.now());
}

async function injectLogo(slot, url, svgClass) {
  if (!slot || slot.dataset.logoReady === '1') return false;
  try {
    const svg = await fetchSvg(url);
    if (!svg) return false;
    mountSvg(slot, svg, svgClass);
    slot.dataset.logoReady = '1';
    animateLogoLayers(slot);
    return true;
  } catch {
    return false;
  }
}

export async function initBrandLogo() {
  const slot = document.querySelector('.brand-logo');
  await injectLogo(slot, TREE_ICON_URL, 'brand-logo-svg');
}

export async function showOverlayLogo() {
  const slot = document.getElementById('overlay-logo');
  const ok = await injectLogo(slot, FULL_LOGO_URL, 'overlay-logo-svg');
  if (ok) slot?.classList.add('is-visible');
}
