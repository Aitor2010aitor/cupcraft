/* Catálogo de estilos de texto de cupcraft.
   Los valores son originales: imitan la organización del panel de texto de
   CapCut (estilo, burbuja, efecto, animación y fuente) sin reutilizar sus
   recursos ni sus archivos. */

export const TEXT_STYLES = [
  {
    id: 'default',
    label: 'Predeterminado',
    sample: 'Texto',
    patch: { size: 72, weight: 800, color: '#ffffff', shadow: true },
  },
  {
    id: 'title',
    label: 'Título',
    sample: 'Título',
    patch: { size: 96, weight: 900, color: '#ffffff', letterSpacing: 0.02, shadow: true },
  },
  {
    id: 'impact',
    label: 'Impacto',
    sample: 'IMPACTO',
    patch: {
      size: 112, weight: 900, color: '#ffffff', textCase: 'upper',
      strokeColor: '#000000', strokeWidth: 0.08, shadow: true,
    },
  },
  {
    id: 'neon',
    label: 'Neón',
    sample: 'NEÓN',
    patch: {
      size: 72, weight: 800, color: '#b6fdff', glow: true, glowColor: '#00e5d0',
      glowBlur: 0.52, strokeColor: '#00a8a8', strokeWidth: 0.025, shadow: false,
    },
  },
  {
    id: 'gold',
    label: 'Oro',
    sample: 'ORO',
    patch: {
      size: 78, weight: 800, color: '#ffd76a', strokeColor: '#7a4b00',
      strokeWidth: 0.045, shadow: true, shadowColor: '#000000', shadowBlur: 0.28,
    },
  },
  {
    id: 'cinema',
    label: 'Cine',
    sample: 'CINE',
    patch: {
      size: 68, weight: 700, color: '#f5f0e6', letterSpacing: 0.12,
      font: 'Georgia', textCase: 'upper', shadow: true,
    },
  },
  {
    id: 'minimal',
    label: 'Minimal',
    sample: 'Minimal',
    patch: { size: 54, weight: 400, color: '#f2f2f2', shadow: false, letterSpacing: 0.01 },
  },
  {
    id: 'quote',
    label: 'Cita',
    sample: '«Cita»',
    patch: { size: 52, weight: 500, color: '#ffffff', font: 'Georgia', italic: true },
  },
  {
    id: 'retro',
    label: 'Retro',
    sample: 'RETRO',
    patch: {
      size: 76, weight: 800, color: '#ffe7a3', strokeColor: '#c14a2a',
      strokeWidth: 0.09, shadow: true, shadowColor: '#6f2a12', shadowDistance: 0.08,
    },
  },
  {
    id: 'comic',
    label: 'Cómic',
    sample: '¡CÓMIC!',
    patch: {
      size: 80, weight: 900, color: '#ffffff', strokeColor: '#111111',
      strokeWidth: 0.12, textCase: 'upper', font: 'Comic Sans MS', shadow: false,
    },
  },
  {
    id: 'tech',
    label: 'Tecno',
    sample: 'TECNO',
    patch: {
      size: 62, weight: 700, color: '#9ffcf2', font: 'Consolas',
      textCase: 'upper', letterSpacing: 0.14, glow: true, glowColor: '#00b3a4', glowBlur: 0.3,
    },
  },
  {
    id: 'soft',
    label: 'Suave',
    sample: 'Suave',
    patch: {
      size: 60, weight: 500, color: '#ffffff', background: '#1f2933',
      backgroundAlpha: 0.55, backgroundRadius: 0.32, shadow: false,
    },
  },
  {
    id: 'pastel',
    label: 'Pastel',
    sample: 'Pastel',
    patch: {
      size: 64, weight: 700, color: '#5b2a4e', background: '#ffd7ec',
      backgroundAlpha: 1, backgroundRadius: 0.48, shadow: false,
    },
  },
  {
    id: 'label',
    label: 'Etiqueta',
    sample: 'ETIQUETA',
    patch: {
      size: 56, weight: 800, color: '#ffffff', background: '#00666b',
      backgroundAlpha: 1, backgroundRadius: 0.16, textCase: 'upper', letterSpacing: 0.04,
    },
  },
  {
    id: 'outline',
    label: 'Contorno',
    sample: 'CONTORNO',
    patch: {
      size: 86, weight: 900, color: '#ffffff', strokeColor: '#000000',
      strokeWidth: 0.16, shadow: false, textCase: 'upper',
    },
  },
  {
    id: 'threed',
    label: '3D',
    sample: '3D',
    patch: {
      size: 84, weight: 900, color: '#ffffff', strokeColor: '#222222',
      strokeWidth: 0.05, shadow: true, shadowColor: '#0e6b70',
      shadowDistance: 0.1, shadowAngle: 45, shadowBlur: 0, textCase: 'upper',
    },
  },
  {
    id: 'wave',
    label: 'Onda',
    sample: 'Onda',
    patch: { size: 64, weight: 700, color: '#ffe9a8', curve: 38, shadow: false },
  },
  {
    id: 'elegant',
    label: 'Elegante',
    sample: 'ELEGANTE',
    patch: {
      size: 58, weight: 400, color: '#f4ead7', font: 'Palatino Linotype',
      letterSpacing: 0.18, textCase: 'upper', shadow: false,
    },
  },
  {
    id: 'fire',
    label: 'Fuego',
    sample: 'FUEGO',
    patch: {
      size: 82, weight: 900, color: '#ffdf6b', glow: true, glowColor: '#ff5a00',
      glowBlur: 0.75, strokeColor: '#6c1600', strokeWidth: 0.06, textCase: 'upper',
    },
  },
  {
    id: 'ice',
    label: 'Hielo',
    sample: 'HIELO',
    patch: {
      size: 78, weight: 800, color: '#eafcff', glow: true, glowColor: '#36b4ff',
      glowBlur: 0.6, strokeColor: '#0b4b78', strokeWidth: 0.035,
    },
  },
];

export const TEXT_BUBBLES = [
  {
    id: 'none',
    label: 'Sin burbuja',
    sample: 'Aa',
    patch: {
      background: 'transparent', backgroundAlpha: 1, backgroundBorderColor: '#000000',
      backgroundBorderWidth: 0, backgroundRadius: 0.16,
    },
  },
  {
    id: 'solid',
    label: 'Sólida',
    sample: 'Aa',
    patch: {
      background: '#ffffff', backgroundAlpha: 1, backgroundRadius: 0.16,
      backgroundBorderColor: '#000000', backgroundBorderWidth: 0, color: '#111111', shadow: false,
    },
  },
  {
    id: 'dark',
    label: 'Oscura',
    sample: 'Aa',
    patch: {
      background: '#101418', backgroundAlpha: 0.86, backgroundRadius: 0.18,
      backgroundBorderColor: '#3a424b', backgroundBorderWidth: 0.01, color: '#ffffff',
    },
  },
  {
    id: 'pill',
    label: 'Pastilla',
    sample: 'Aa',
    patch: {
      background: '#00e5d0', backgroundAlpha: 1, backgroundRadius: 0.5,
      backgroundBorderColor: '#00e5d0', backgroundBorderWidth: 0, color: '#032a2b', shadow: false,
    },
  },
  {
    id: 'outline',
    label: 'Contorno',
    sample: 'Aa',
    patch: {
      background: '#000000', backgroundAlpha: 0.06, backgroundRadius: 0.18,
      backgroundBorderColor: '#ffffff', backgroundBorderWidth: 0.012, color: '#ffffff', shadow: false,
    },
  },
  {
    id: 'label',
    label: 'Etiqueta',
    sample: 'Aa',
    patch: {
      background: '#ffd76a', backgroundAlpha: 1, backgroundRadius: 0.1,
      backgroundBorderColor: '#b88700', backgroundBorderWidth: 0.008, color: '#3a2a00', shadow: false,
    },
  },
  {
    id: 'ribbon',
    label: 'Cinta',
    sample: 'Aa',
    patch: {
      background: '#ff5c8a', backgroundAlpha: 1, backgroundRadius: 0.08,
      backgroundBorderColor: '#a80033', backgroundBorderWidth: 0.008, color: '#ffffff', shadow: false,
    },
  },
  {
    id: 'note',
    label: 'Nota',
    sample: 'Aa',
    patch: {
      background: '#fff3bf', backgroundAlpha: 1, backgroundRadius: 0.04,
      backgroundBorderColor: '#d9c66b', backgroundBorderWidth: 0.006, color: '#3a2f00', shadow: false,
    },
  },
  {
    id: 'glass',
    label: 'Cristal',
    sample: 'Aa',
    patch: {
      background: '#7ce7ff', backgroundAlpha: 0.28, backgroundRadius: 0.24,
      backgroundBorderColor: '#c8f6ff', backgroundBorderWidth: 0.01, color: '#ffffff',
    },
  },
];

export const TEXT_EFFECTS = [
  {
    id: 'none',
    label: 'Ninguno',
    patch: {
      strokeWidth: 0, glow: false, shadow: false, curve: 0,
    },
  },
  {
    id: 'stroke',
    label: 'Contorno',
    patch: { strokeColor: '#000000', strokeWidth: 0.08, glow: false },
  },
  {
    id: 'neon',
    label: 'Neón',
    patch: {
      glow: true, glowColor: '#00e5d0', glowBlur: 0.55, color: '#c8fffb',
      strokeColor: '#008f8f', strokeWidth: 0.02,
    },
  },
  {
    id: 'shadow',
    label: 'Sombra',
    patch: {
      shadow: true, shadowColor: '#000000', shadowAlpha: 0.7,
      shadowBlur: 0.25, shadowDistance: 0.07, shadowAngle: 90,
    },
  },
  {
    id: 'three-d',
    label: '3D',
    patch: {
      shadow: true, shadowColor: '#0a5b61', shadowAlpha: 1,
      shadowDistance: 0.1, shadowAngle: 45, shadowBlur: 0,
    },
  },
  {
    id: 'glow',
    label: 'Resplandor',
    patch: { glow: true, glowColor: '#ff7a00', glowBlur: 0.7 },
  },
  {
    id: 'outline-3d',
    label: 'Contorno 3D',
    patch: {
      strokeColor: '#000000', strokeWidth: 0.1, shadow: true,
      shadowColor: '#222222', shadowDistance: 0.08, shadowAngle: 45, shadowBlur: 0,
    },
  },
  {
    id: 'soft',
    label: 'Difuminado',
    patch: {
      shadow: true, shadowColor: '#000000', shadowAlpha: 0.45,
      shadowBlur: 0.5, shadowDistance: 0.02,
    },
  },
  {
    id: 'arc',
    label: 'Arco',
    patch: { curve: 42, shadow: false },
  },
];

export const TEXT_ANIMATIONS = [
  { id: 'in-fade', label: 'Fundido', phase: 'in', duration: 0.6 },
  { id: 'in-slide-up', label: 'Deslizar arriba', phase: 'in', duration: 0.6 },
  { id: 'in-slide-down', label: 'Deslizar abajo', phase: 'in', duration: 0.6 },
  { id: 'in-slide-left', label: 'Deslizar izquierda', phase: 'in', duration: 0.6 },
  { id: 'in-slide-right', label: 'Deslizar derecha', phase: 'in', duration: 0.6 },
  { id: 'in-zoom', label: 'Zoom', phase: 'in', duration: 0.7 },
  { id: 'in-pop', label: 'Pop', phase: 'in', duration: 0.55 },
  { id: 'in-typewriter', label: 'Máquina de escribir', phase: 'in', duration: 1.2 },
  { id: 'in-spin', label: 'Giro', phase: 'in', duration: 0.8 },
  { id: 'in-blur', label: 'Desenfoque', phase: 'in', duration: 0.7 },
  { id: 'in-bounce', label: 'Rebote', phase: 'in', duration: 0.75 },
  { id: 'in-diagonal', label: 'Diagonal', phase: 'in', duration: 0.65 },
  { id: 'in-flip', label: 'Giro 3D', phase: 'in', duration: 0.7 },
  { id: 'out-fade', label: 'Fundido', phase: 'out', duration: 0.6 },
  { id: 'out-slide-up', label: 'Deslizar arriba', phase: 'out', duration: 0.6 },
  { id: 'out-slide-down', label: 'Deslizar abajo', phase: 'out', duration: 0.6 },
  { id: 'out-slide-left', label: 'Salir izquierda', phase: 'out', duration: 0.6 },
  { id: 'out-slide-right', label: 'Salir derecha', phase: 'out', duration: 0.6 },
  { id: 'out-zoom', label: 'Alejar', phase: 'out', duration: 0.7 },
  { id: 'out-blur', label: 'Desenfoque', phase: 'out', duration: 0.65 },
  { id: 'out-spin', label: 'Giro', phase: 'out', duration: 0.7 },
  { id: 'loop-wave', label: 'Onda', phase: 'loop', duration: 2.4 },
  { id: 'loop-pulse', label: 'Latido', phase: 'loop', duration: 1.6 },
  { id: 'loop-shake', label: 'Vibración', phase: 'loop', duration: 1.2 },
  { id: 'loop-swing', label: 'Balanceo', phase: 'loop', duration: 2.2 },
  { id: 'loop-float', label: 'Flotar', phase: 'loop', duration: 2.8 },
  { id: 'loop-blink', label: 'Parpadeo', phase: 'loop', duration: 1.5 },
  { id: 'loop-orbit', label: 'Órbita', phase: 'loop', duration: 2.2 },
  { id: 'loop-wiggle', label: 'Coletazo', phase: 'loop', duration: 1.2 },
  { id: 'caption-karaoke', label: 'Karaoke', phase: 'caption', duration: 0.3 },
  { id: 'caption-pop', label: 'Salto', phase: 'caption', duration: 0.25 },
];

export const TEXT_ANIMATION_GROUPS = [
  { id: 'in', label: 'Entrada', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'in') },
  { id: 'loop', label: 'Bucle', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'loop') },
  { id: 'out', label: 'Salida', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'out') },
  { id: 'caption', label: 'Subtítulos (Pro)', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'caption') },
];

export const POPULAR_FONTS = [
  { value: 'Noto Serif', label: 'Noto Serif', category: 'popular' },
  { value: 'Open Sans', label: 'OpenSans', category: 'popular' },
  { value: 'Source Sans 3', label: 'Source Sans Pro', category: 'popular' },
  { value: 'Quicksand', label: 'Quicksand', category: 'popular' },
  { value: 'Arvo', label: 'Arvo', category: 'popular' },
  { value: 'Red Hat Display', label: 'Red Hat', category: 'popular' },
  { value: 'Bebas Neue', label: 'BEBASNEUE', category: 'popular' },
  { value: 'Playfair Display', label: 'Playfair Display', category: 'popular' },
  { value: 'Montserrat', label: 'Montserrat', category: 'popular' },
  { value: 'Poppins', label: 'Poppins', category: 'popular' },
  { value: 'Roboto', label: 'Roboto', category: 'popular' },
  { value: 'Oswald', label: 'Oswald', category: 'popular' },
  { value: 'Pacifico', label: 'Pacifico', category: 'popular' },
  { value: 'Dancing Script', label: 'Dancing Script', category: 'popular' },
  { value: 'Anton', label: 'Anton', category: 'popular' },
  { value: 'Caveat', label: 'Caveat', category: 'popular' },
  { value: 'Lobster', label: 'Lobster', category: 'popular' },
];

export const SYSTEM_BASE_FONTS = [
  { value: 'Inter', label: 'Inter (recomendada)', category: 'system' },
  { value: 'Segoe UI', label: 'Segoe UI', category: 'system' },
  { value: 'Segoe UI Variable Display', label: 'Segoe UI Variable', category: 'system' },
  { value: 'Arial', label: 'Arial', category: 'system' },
  { value: 'Arial Black', label: 'Arial Black', category: 'system' },
  { value: 'Impact', label: 'Impact', category: 'system' },
  { value: 'Georgia', label: 'Georgia', category: 'system' },
  { value: 'Times New Roman', label: 'Times New Roman', category: 'system' },
  { value: 'Palatino Linotype', label: 'Palatino', category: 'system' },
  { value: 'Trebuchet MS', label: 'Trebuchet MS', category: 'system' },
  { value: 'Verdana', label: 'Verdana', category: 'system' },
  { value: 'Tahoma', label: 'Tahoma', category: 'system' },
  { value: 'Cambria', label: 'Cambria', category: 'system' },
  { value: 'Candara', label: 'Candara', category: 'system' },
  { value: 'Consolas', label: 'Consolas', category: 'system' },
  { value: 'Courier New', label: 'Courier New', category: 'system' },
  { value: 'Comic Sans MS', label: 'Comic Sans MS', category: 'system' },
  { value: 'Segoe Print', label: 'Segoe Print', category: 'system' },
  { value: 'Segoe Script', label: 'Segoe Script', category: 'system' },
  { value: 'Lucida Handwriting', label: 'Lucida Handwriting', category: 'system' },
];

export const TEXT_FONTS = [...POPULAR_FONTS, ...SYSTEM_BASE_FONTS];

// Carga dinámica de fuentes web (Google Fonts) para previsualización real idéntica a CapCut
export function ensureWebFontsLoaded() {
  if (typeof document === 'undefined') return;
  if (document.getElementById('cupcraft-web-fonts')) return;
  const link = document.createElement('link');
  link.id = 'cupcraft-web-fonts';
  link.rel = 'stylesheet';
  link.href = 'https://fonts.googleapis.com/css2?family=Arvo:ital,wght@0,400;0,700;1,400&family=Bebas+Neue&family=Caveat:wght@400..700&family=Dancing+Script:wght@400..700&family=Lobster&family=Montserrat:ital,wght@0,300..900;1,300..900&family=Noto+Serif:ital,wght@0,400..700;1,400..700&family=Open+Sans:ital,wght@0,300..800;1,300..800&family=Oswald:wght@200..700&family=Pacifico&family=Playfair+Display:ital,wght@0,400..900;1,400..900&family=Poppins:ital,wght@0,300..900;1,300..900&family=Quicksand:wght@300..700&family=Red+Hat+Display:ital,wght@0,300..900;1,300..900&family=Roboto:ital,wght@0,300..900;1,300..900&family=Source+Sans+3:ital,wght@0,300..900;1,300..900&display=swap';
  document.head.appendChild(link);
}
if (typeof window !== 'undefined') {
  ensureWebFontsLoaded();
}

// Fuentes personalizadas importadas por el usuario (.ttf, .otf)
let customFonts = [];
function initCustomFonts() {
  if (typeof localStorage === 'undefined') return;
  try {
    const raw = localStorage.getItem('cupcraft_custom_fonts');
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        customFonts = parsed;
        // Registrar en document.fonts
        if (typeof document !== 'undefined' && document.fonts) {
          customFonts.forEach((f) => {
            if (f.dataUrl) {
              const face = new FontFace(f.value, `url(${f.dataUrl})`);
              face.load().then((loaded) => document.fonts.add(loaded)).catch(() => {});
            }
          });
        }
      }
    }
  } catch (e) {
    console.warn('initCustomFonts:', e);
  }
}
if (typeof window !== 'undefined') {
  initCustomFonts();
}

export async function importCustomFont(file) {
  if (!file) throw new Error('No se seleccionó archivo');
  const baseName = file.name.replace(/\.[^/.]+$/, '').trim();
  const fontName = baseName || 'Fuente personal';
  const buffer = await file.arrayBuffer();

  // Convertir a DataURL para almacenamiento persistente
  const reader = new FileReader();
  const dataUrlPromise = new Promise((resolve) => {
    reader.onload = () => resolve(reader.result);
    reader.readAsDataURL(file);
  });
  const dataUrl = await dataUrlPromise;

  if (typeof document !== 'undefined' && document.fonts) {
    const face = new FontFace(fontName, buffer);
    await face.load();
    document.fonts.add(face);
  }

  const existingIndex = customFonts.findIndex((f) => f.value.toLowerCase() === fontName.toLowerCase());
  const entry = { value: fontName, label: fontName, category: 'imported', isCustom: true, dataUrl };
  if (existingIndex >= 0) {
    customFonts[existingIndex] = entry;
  } else {
    customFonts.unshift(entry);
  }

  try {
    localStorage.setItem('cupcraft_custom_fonts', JSON.stringify(customFonts));
  } catch (e) {
    console.warn('No se pudo guardar la fuente en localStorage (posible límite de tamaño):', e);
  }

  return entry;
}

// Favoritos
export function getFavoriteFonts() {
  if (typeof localStorage === 'undefined') return new Set();
  try {
    return new Set(JSON.parse(localStorage.getItem('cupcraft_favorite_fonts') || '[]'));
  } catch {
    return new Set();
  }
}

export function isFavoriteFont(fontValue) {
  return getFavoriteFonts().has(fontValue);
}

export function toggleFavoriteFont(fontValue) {
  const favs = getFavoriteFonts();
  if (favs.has(fontValue)) {
    favs.delete(fontValue);
  } else {
    favs.add(fontValue);
  }
  try {
    localStorage.setItem('cupcraft_favorite_fonts', JSON.stringify([...favs]));
  } catch {}
  return favs.has(fontValue);
}

// Fuentes del sistema detectadas. La app nativa (Rust) enumera las fuentes
// instaladas en Windows y las sirve en /api/system-fonts; en un navegador
// normal se intenta la Font Access API como alternativa.
const SYSTEM_FONTS_ENDPOINT = '/api/system-fonts';
const SYSTEM_FONT_STYLE_ID = 'cupcraft-system-fonts';
const registeredFontFaces = new Set();
let systemFonts = [];
let systemFontsLoaded = false;
let systemFontsLoading = null;

function cssFontFamily(name) {
  return `"${String(name).replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

function systemFontFileUrl(file) {
  return `${SYSTEM_FONTS_ENDPOINT}/file?name=${encodeURIComponent(file)}`;
}

// Registra @font-face para las fuentes detectadas. Es importante para las
// fuentes instaladas solo para el usuario actual: WebView2 no siempre las ve
// hasta reiniciar, pero sí puede cargar el archivo desde el servidor local.
export function registerSystemFontFaces(fonts = []) {
  if (typeof document === 'undefined') return;
  let style = document.getElementById(SYSTEM_FONT_STYLE_ID);
  if (!style) {
    style = document.createElement('style');
    style.id = SYSTEM_FONT_STYLE_ID;
    document.head.appendChild(style);
  }
  const rules = [];
  for (const font of fonts) {
    const family = String(font?.value || font?.family || '').trim();
    const file = font?.file;
    if (!family || !file) continue;
    const key = family.toLowerCase();
    if (registeredFontFaces.has(key)) continue;
    registeredFontFaces.add(key);
    rules.push(`@font-face{font-family:${cssFontFamily(family)};src:url("${systemFontFileUrl(file)}");font-display:swap;}`);
  }
  if (rules.length) style.appendChild(document.createTextNode(rules.join('\n')));
}

// Fuerza la carga de una fuente concreta antes de dibujar el texto.
export async function ensureFontReady(fontValue) {
  const family = String(fontValue || '').trim();
  if (!family || typeof document === 'undefined' || !document.fonts) return;
  try {
    await document.fonts.load(`16px ${cssFontFamily(family)}`);
  } catch (e) {
    console.warn('ensureFontReady:', family, e);
  }
}

async function fetchServerSystemFonts(refresh = false) {
  if (typeof fetch !== 'function') return [];
  const controller = typeof AbortController === 'function' ? new AbortController() : null;
  const timer = controller ? setTimeout(() => controller.abort(), 6000) : null;
  try {
    const url = refresh ? `${SYSTEM_FONTS_ENDPOINT}?refresh=1` : SYSTEM_FONTS_ENDPOINT;
    const response = await fetch(url, { cache: 'no-store', signal: controller?.signal });
    if (!response.ok) return [];
    const data = await response.json();
    const raw = Array.isArray(data?.fonts) ? data.fonts : [];
    const known = new Set(TEXT_FONTS.map((font) => font.value.toLowerCase()));
    const seen = new Set();
    const list = [];
    for (const font of raw) {
      const family = String(font?.family || font?.label || '').trim();
      if (!family) continue;
      const key = family.toLowerCase();
      if (known.has(key) || seen.has(key)) continue;
      seen.add(key);
      list.push({
        value: family,
        label: family,
        category: 'system',
        isSystem: true,
        file: font?.file || null,
        source: font?.source || 'windows',
      });
    }
    registerSystemFontFaces(list);
    return list.sort((a, b) => a.label.localeCompare(b.label, 'es'));
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function fetchBrowserSystemFonts() {
  if (typeof window === 'undefined' || typeof window.queryLocalFonts !== 'function') return [];
  const fontsPromise = window.queryLocalFonts();
  const timeoutPromise = new Promise((_, reject) => setTimeout(() => reject(new Error('timeout')), 3000));
  const fonts = await Promise.race([fontsPromise, timeoutPromise]);
  const known = new Set(TEXT_FONTS.map((font) => font.value.toLowerCase()));
  const families = [...new Set(fonts.map((font) => font.family).filter(Boolean))]
    .sort((a, b) => a.localeCompare(b, 'es'));
  return families
    .filter((family) => !known.has(family.toLowerCase()))
    .map((family) => ({ value: family, label: family, category: 'system' }));
}

export async function loadSystemFonts({ refresh = false } = {}) {
  if (systemFontsLoaded && !refresh) return systemFonts;
  if (systemFontsLoading && !refresh) return systemFontsLoading;
  systemFontsLoading = (async () => {
    let detected = [];
    try {
      detected = await fetchServerSystemFonts(refresh);
    } catch (e) {
      console.warn('loadSystemFonts (servidor local):', e);
    }
    if (!detected.length) {
      try {
        detected = await fetchBrowserSystemFonts();
      } catch (e) {
        console.warn('loadSystemFonts (navegador):', e);
      }
    }
    systemFonts = detected;
    systemFontsLoaded = true;
    systemFontsLoading = null;
    return systemFonts;
  })();
  return systemFontsLoading;
}

export function getFontList(category = 'all', searchQuery = '') {
  let list = [...customFonts, ...POPULAR_FONTS, ...SYSTEM_BASE_FONTS, ...systemFonts];
  const query = (searchQuery || '').trim().toLowerCase();
  
  if (category === 'popular') {
    list = list.filter((f) => f.category === 'popular');
  } else if (category === 'system') {
    list = list.filter((f) => f.category === 'system');
  } else if (category === 'imported') {
    list = list.filter((f) => f.category === 'imported' || f.isCustom);
  } else if (category === 'favorites') {
    const favs = getFavoriteFonts();
    list = list.filter((f) => favs.has(f.value));
  }

  if (query) {
    list = list.filter((f) => f.label.toLowerCase().includes(query) || f.value.toLowerCase().includes(query));
  }

  return list;
}

export const TEXT_CASES = [
  { value: 'none', label: 'Aa', title: 'Texto original' },
  { value: 'upper', label: 'AA', title: 'Mayúsculas' },
  { value: 'lower', label: 'aa', title: 'Minúsculas' },
  { value: 'capitalize', label: 'Aa.', title: 'Inicial mayúscula' },
];

export const TEXT_ALIGNS = [
  { value: 'left', label: 'Izquierda' },
  { value: 'center', label: 'Centro' },
  { value: 'right', label: 'Derecha' },
];

export const TEXT_WEIGHTS = [
  { value: 300, label: 'Fina' },
  { value: 400, label: 'Normal' },
  { value: 500, label: 'Media' },
  { value: 600, label: 'Seminegrita' },
  { value: 700, label: 'Negrita' },
  { value: 800, label: 'Extra' },
  { value: 900, label: 'Black' },
];

export function textAnimationById(id) {
  return TEXT_ANIMATIONS.find((item) => item.id === id) || null;
}
