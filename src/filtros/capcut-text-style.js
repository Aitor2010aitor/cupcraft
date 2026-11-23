// Deriva un estilo de texto editable a partir de la miniatura de una plantilla
// de CapCut. No copiamos sus recursos: solo se leen los colores dominantes de
// la imagen (a través del proxy local) y se construye un patch para el modelo
// de texto de cupcraft.

const cache = new Map();
const MAX_CACHE = 80;

function proxied(url) {
  if (!url) return null;
  return url.startsWith('/api/img?') ? url : `/api/img?u=${encodeURIComponent(url)}`;
}

function loadImage(src) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('no se pudo cargar la miniatura'));
    img.src = src;
  });
}

function toHex(r, g, b) {
  return `#${[r, g, b].map((value) => Math.max(0, Math.min(255, Math.round(value))).toString(16).padStart(2, '0')).join('')}`;
}

function relativeLuminance(r, g, b) {
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
}

/**
 * Analiza la miniatura y devuelve colores representativos:
 * fill (el más brillante), stroke (el más oscuro) y glow (el más saturado).
 */
function paletteFromImage(img) {
  const size = 48;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(img, 0, 0, size, size);
  const data = ctx.getImageData(0, 0, size, size).data;

  const buckets = new Map();
  let bestFill = [255, 255, 255];
  let bestFillLum = -1;
  let bestStroke = [0, 0, 0];
  let bestStrokeLum = 2;
  let bestGlow = [255, 255, 255];
  let bestGlowScore = -1;

  for (let index = 0; index < data.length; index += 4) {
    const alpha = data[index + 3] / 255;
    if (alpha < 0.35) continue;
    const r = data[index];
    const g = data[index + 1];
    const b = data[index + 2];
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const saturation = max === 0 ? 0 : (max - min) / max;
    const luminance = relativeLuminance(r, g, b);

    if (luminance > bestFillLum && saturation < 0.92) {
      bestFillLum = luminance;
      bestFill = [r, g, b];
    }
    if (luminance < bestStrokeLum && luminance < 0.35) {
      bestStrokeLum = luminance;
      bestStroke = [r, g, b];
    }
    const glowScore = saturation * (0.4 + luminance);
    if (glowScore > bestGlowScore) {
      bestGlowScore = glowScore;
      bestGlow = [r, g, b];
    }

    const key = `${r >> 5}-${g >> 5}-${b >> 5}`;
    const bucket = buckets.get(key) || { count: 0, r: 0, g: 0, b: 0 };
    bucket.count += 1;
    bucket.r += r;
    bucket.g += g;
    bucket.b += b;
    buckets.set(key, bucket);
  }

  const ranked = [...buckets.values()].sort((a, b) => b.count - a.count).slice(0, 6)
    .map((entry) => toHex(entry.r / entry.count, entry.g / entry.count, entry.b / entry.count));

  const fill = bestFillLum > 0.2 ? bestFill : [255, 255, 255];
  const stroke = bestStrokeLum < 0.35 ? bestStroke : [15, 15, 20];
  const glow = bestGlowScore > 0.18 ? bestGlow : null;

  return { fill: toHex(...fill), stroke: toHex(...stroke), glow: glow ? toHex(...glow) : null, colors: ranked };
}

function shouldUseGlow(name) {
  return /ne[oó]n|glow|brillo|luz|shine|flash|led|light|luminous|荧光|发光|霓虹/i.test(name || '');
}

function shouldUseShadow(name) {
  return /sombra|shadow|3d|relieve|depth|drop|立体|阴影/i.test(name || '');
}

function shouldOutline(name, palette) {
  if (/contorno|outline|stroke|border|描边|边框/i.test(name || '')) return true;
  return relativeLuminance(...paletteFill(palette.fill)) > 0.72;
}

function paletteFill(hex) {
  const value = String(hex).replace('#', '');
  return [0, 2, 4].map((index) => parseInt(value.slice(index, index + 2), 16));
}

/**
 * Devuelve el patch de texto para una plantilla de CapCut. Se carga de forma
 * asíncrona y se guarda en caché. `url` es la imagen original remota; `img` la
 * miniatura local (que normalmente no está incluida en el paquete).
 */
export async function capcutTextStyle(item) {
  if (!item) return null;
  const original = item.asset || item.url || null;
  const key = item.id || original || item.img;
  if (cache.has(key)) return cache.get(key);

  // Sin red: derivamos el estilo de la miniatura local, que siempre existe.
  // Las URLs firmadas de CapCut caducan y sólo generan errores HTTP.
  const sources = [item.img].filter(Boolean);
  let palette = null;
  for (const source of sources) {
    try {
      const img = await loadImage(source);
      palette = paletteFromImage(img);
      break;
    } catch {
      // Se prueba la siguiente fuente.
    }
  }
  if (!palette) {
    palette = { fill: '#ffffff', stroke: '#101014', glow: null, colors: [] };
  }

  const name = item.name || '';
  const patch = {
    color: palette.fill,
    strokeColor: palette.stroke,
    shadow: shouldUseShadow(name),
    shadowColor: palette.stroke,
    glow: shouldUseGlow(name),
    glowColor: palette.glow || palette.fill,
    background: 'transparent',
  };
  if (!shouldUseGlow(name) && shouldOutline(name, palette)) {
    patch.strokeWidth = 0.055;
  }
  if (shouldUseGlow(name)) {
    patch.glowBlur = 0.5;
  }
  if (shouldUseShadow(name)) {
    patch.shadowBlur = 0.1;
    patch.shadowDistance = 0.06;
  }

  const result = { patch, palette, name };
  if (cache.size >= MAX_CACHE) cache.delete(cache.keys().next().value);
  cache.set(key, result);
  return result;
}
