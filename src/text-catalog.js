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
  { id: 'out-fade', label: 'Fundido', phase: 'out', duration: 0.6 },
  { id: 'out-slide-up', label: 'Deslizar arriba', phase: 'out', duration: 0.6 },
  { id: 'out-slide-down', label: 'Deslizar abajo', phase: 'out', duration: 0.6 },
  { id: 'out-zoom', label: 'Alejar', phase: 'out', duration: 0.7 },
  { id: 'loop-wave', label: 'Onda', phase: 'loop', duration: 2.4 },
  { id: 'loop-pulse', label: 'Latido', phase: 'loop', duration: 1.6 },
  { id: 'loop-shake', label: 'Vibración', phase: 'loop', duration: 1.2 },
  { id: 'loop-swing', label: 'Balanceo', phase: 'loop', duration: 2.2 },
  { id: 'loop-float', label: 'Flotar', phase: 'loop', duration: 2.8 },
];

export const TEXT_ANIMATION_GROUPS = [
  { id: 'in', label: 'Entrada', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'in') },
  { id: 'loop', label: 'Bucle', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'loop') },
  { id: 'out', label: 'Salida', items: TEXT_ANIMATIONS.filter((item) => item.phase === 'out') },
];

export const TEXT_FONTS = [
  { value: 'Inter', label: 'Inter (recomendada)' },
  { value: 'Segoe UI', label: 'Segoe UI' },
  { value: 'Segoe UI Variable Display', label: 'Segoe UI Variable' },
  { value: 'Arial', label: 'Arial' },
  { value: 'Arial Black', label: 'Arial Black' },
  { value: 'Impact', label: 'Impact' },
  { value: 'Georgia', label: 'Georgia' },
  { value: 'Times New Roman', label: 'Times New Roman' },
  { value: 'Palatino Linotype', label: 'Palatino' },
  { value: 'Trebuchet MS', label: 'Trebuchet MS' },
  { value: 'Verdana', label: 'Verdana' },
  { value: 'Tahoma', label: 'Tahoma' },
  { value: 'Cambria', label: 'Cambria' },
  { value: 'Candara', label: 'Candara' },
  { value: 'Consolas', label: 'Consolas' },
  { value: 'Courier New', label: 'Courier New' },
  { value: 'Comic Sans MS', label: 'Comic Sans MS' },
  { value: 'Segoe Print', label: 'Segoe Print' },
  { value: 'Segoe Script', label: 'Segoe Script' },
  { value: 'Lucida Handwriting', label: 'Lucida Handwriting' },
];

// Fuentes del sistema detectadas (Font Access API, solo Chromium).
let systemFonts = [];
let systemFontsLoaded = false;
let systemFontsLoading = null;

export async function loadSystemFonts() {
  if (systemFontsLoaded) return systemFonts;
  if (systemFontsLoading) return systemFontsLoading;
  systemFontsLoading = (async () => {
    try {
      if (typeof window !== 'undefined' && typeof window.queryLocalFonts === 'function') {
        const fontsPromise = window.queryLocalFonts();
        const timeoutPromise = new Promise((_, rej) => setTimeout(() => rej(new Error('timeout')), 3000));
        const fonts = await Promise.race([fontsPromise, timeoutPromise]);
        const families = [...new Set(fonts.map((f) => f.family).filter(Boolean))].sort((a, b) => a.localeCompare(b));
        const known = new Set(TEXT_FONTS.map((f) => f.value.toLowerCase()));
        systemFonts = families
          .filter((f) => !known.has(f.toLowerCase()))
          .map((f) => ({ value: f, label: `${f} (sistema)` }));
      }
    } catch (e) {
      console.warn('loadSystemFonts:', e);
      systemFonts = [];
    }
    systemFontsLoaded = true;
    systemFontsLoading = null;
    return systemFonts;
  })();
  return systemFontsLoading;
}

export function getFontList() {
  return [...TEXT_FONTS, ...systemFonts];
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
