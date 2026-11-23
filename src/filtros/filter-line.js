/* Línea de filtros propia de cupcraft.
   Todo el catálogo es nuestro: las mezclas de color son filtros CSS con valores
   propios y los de "Textura"/"Firma" usan los shaders GL del motor (gl-post.js).
   No hay ninguna referencia al catálogo de CapCut: ni nombres, ni imágenes, ni
   identificadores, de modo que el panel funciona igual sin red y sin instalar
   carátulas.

   Cada entrada es { label, category, css } y, si el filtro se pinta con el
   motor GL, añade { gl: '<clave del shader>' }. El campo `css` es además la
   vista previa de la tarjeta y el valor de reserva si WebGL no está disponible. */

export const FILTER_LINE_CATEGORIES = [
  'Esenciales',
  'Cálido',
  'Frío',
  'Blanco y negro',
  'Cine',
  'Analógico',
  'Retrato',
  'Paisaje',
  'Neón',
  'Clima',
  'Textura',
  'Firma',
];

/* Mezclas de color (saturación, contraste, brillo, tono, sepia…) escritas a mano. */
const COLOR_LINE = {
  /* --- Esenciales: el punto de partida de cualquier corte --- */
  vivid: { label: 'Vívido', category: 'Esenciales', css: 'saturate(1.45) contrast(1.08)' },
  amber: { label: 'Ámbar', category: 'Esenciales', css: 'sepia(0.3) saturate(1.2) brightness(1.03)' },
  crystal: { label: 'Cristal', category: 'Esenciales', css: 'saturate(0.95) contrast(1.06) brightness(1.06)' },
  sand: { label: 'Arena', category: 'Esenciales', css: 'sepia(0.24) saturate(1.05) brightness(1.04) contrast(0.98)' },
  smoke: { label: 'Humo', category: 'Esenciales', css: 'saturate(0.7) contrast(1.05) brightness(0.98)' },
  dream: { label: 'Ensueño', category: 'Esenciales', css: 'saturate(1.05) brightness(1.12) contrast(0.9) blur(0.3px)' },
  pastel: { label: 'Pastel', category: 'Esenciales', css: 'saturate(0.9) brightness(1.1) contrast(0.88)' },

  /* --- Cálido: luz de tarde, miel y cobre --- */
  warm: { label: 'Cálido', category: 'Cálido', css: 'sepia(0.28) saturate(1.15) hue-rotate(-8deg)' },
  sunset: { label: 'Atardecer', category: 'Cálido', css: 'sepia(0.22) saturate(1.4) hue-rotate(-16deg) brightness(1.04)' },
  golden: { label: 'Dorado', category: 'Cálido', css: 'sepia(0.38) saturate(1.25) brightness(1.08)' },
  copper: { label: 'Cobre', category: 'Cálido', css: 'sepia(0.42) saturate(1.12) contrast(1.06)' },
  honey: { label: 'Miel', category: 'Cálido', css: 'sepia(0.45) saturate(1.2) brightness(1.06)' },
  cinnamon: { label: 'Canela', category: 'Cálido', css: 'sepia(0.36) hue-rotate(-6deg) contrast(1.05)' },
  dusk: { label: 'Poniente', category: 'Cálido', css: 'sepia(0.3) saturate(1.5) hue-rotate(-22deg) brightness(1.02)' },

  /* --- Frío: azules limpios, hielo y agua --- */
  cool: { label: 'Frío', category: 'Frío', css: 'saturate(1.1) hue-rotate(12deg) brightness(1.02)' },
  arctic: { label: 'Ártico', category: 'Frío', css: 'saturate(0.9) hue-rotate(16deg) brightness(1.1) contrast(0.98)' },
  ocean: { label: 'Océano', category: 'Frío', css: 'saturate(1.25) hue-rotate(178deg) brightness(0.98)' },
  teal: { label: 'Turquesa', category: 'Frío', css: 'saturate(1.2) hue-rotate(140deg) brightness(1.02)' },
  mint: { label: 'Menta', category: 'Frío', css: 'saturate(0.95) hue-rotate(96deg) brightness(1.06)' },
  blizzard: { label: 'Ventisca', category: 'Frío', css: 'saturate(0.85) hue-rotate(20deg) brightness(1.1) contrast(1.02)' },
  blueHaze: { label: 'Niebla azul', category: 'Frío', css: 'saturate(1.1) hue-rotate(190deg) brightness(1.05) contrast(0.95)' },
  coldShadow: { label: 'Sombra fría', category: 'Frío', css: 'hue-rotate(16deg) contrast(1.1) brightness(0.96)' },

  /* --- Blanco y negro: del gris plata al negro profundo --- */
  mono: { label: 'B/N', category: 'Blanco y negro', css: 'grayscale(1) contrast(1.05)' },
  noir: { label: 'Noir', category: 'Blanco y negro', css: 'grayscale(1) contrast(1.35) brightness(0.9)' },
  silver: { label: 'Plata', category: 'Blanco y negro', css: 'grayscale(1) contrast(0.94) brightness(1.12)' },
  charcoal: { label: 'Carbón', category: 'Blanco y negro', css: 'grayscale(1) contrast(1.5) brightness(0.8)' },
  graphite: { label: 'Grafito', category: 'Blanco y negro', css: 'grayscale(1) contrast(1.2) brightness(1.02)' },
  deepBlack: { label: 'Negro profundo', category: 'Blanco y negro', css: 'grayscale(1) contrast(1.6) brightness(0.85)' },
  snow: { label: 'Nieve', category: 'Blanco y negro', css: 'grayscale(1) brightness(1.18) contrast(0.95)' },

  /* --- Cine: salas oscuras, tráilers y fotogramas lavados --- */
  film: { label: 'Película', category: 'Cine', css: 'sepia(0.18) saturate(0.9) contrast(1.12) brightness(0.96)' },
  matte: { label: 'Mate', category: 'Cine', css: 'saturate(0.82) contrast(0.88) brightness(1.06)' },
  fade: { label: 'Desvanecido', category: 'Cine', css: 'opacity(0.92) saturate(0.75) brightness(1.08)' },
  darkRoom: { label: 'Sala oscura', category: 'Cine', css: 'contrast(1.2) saturate(0.9) brightness(0.92)' },
  trailer: { label: 'Tráiler', category: 'Cine', css: 'contrast(1.3) saturate(1.15) brightness(0.95)' },
  bleach: { label: 'Bleach', category: 'Cine', css: 'contrast(1.15) saturate(0.6) brightness(1.06)' },
  seventies: { label: 'Setenta', category: 'Cine', css: 'sepia(0.2) contrast(1.1) saturate(1.1) hue-rotate(-8deg)' },
  anamorphic: { label: 'Anamórfico', category: 'Cine', css: 'contrast(1.12) saturate(1.1) hue-rotate(6deg) brightness(1.02)' },

  /* --- Analógico: cámaras de bobina, polaroid y revelado casero --- */
  vintage: { label: 'Vintage', category: 'Analógico', css: 'sepia(0.5) contrast(0.92) brightness(0.95)' },
  retro: { label: 'Retro', category: 'Analógico', css: 'sepia(0.32) saturate(0.85) contrast(0.9) brightness(1.05)' },
  polaroid: { label: 'Polaroid', category: 'Analógico', css: 'sepia(0.3) saturate(1.1) contrast(0.92) brightness(1.06)' },
  eightMm: { label: 'Ocho milímetros', category: 'Analógico', css: 'sepia(0.4) contrast(1.08) saturate(0.85) brightness(1.02)' },
  reel: { label: 'Carrete', category: 'Analógico', css: 'sepia(0.26) contrast(1.05) saturate(0.95)' },
  develop: { label: 'Revelado', category: 'Analógico', css: 'contrast(1.1) saturate(1.05) brightness(0.98) sepia(0.12)' },
  underexposed: { label: 'Subexpuesto', category: 'Analógico', css: 'brightness(0.9) contrast(1.08) saturate(0.9)' },

  /* --- Retrato: piel limpia, rubor y tono melocotón --- */
  rose: { label: 'Rosa', category: 'Retrato', css: 'sepia(0.16) saturate(1.2) hue-rotate(324deg) brightness(1.03)' },
  lavender: { label: 'Lavanda', category: 'Retrato', css: 'saturate(0.95) hue-rotate(250deg) brightness(1.06)' },
  softSkin: { label: 'Piel suave', category: 'Retrato', css: 'saturate(1.08) brightness(1.06) contrast(0.96)' },
  peach: { label: 'Melocotón', category: 'Retrato', css: 'sepia(0.2) saturate(1.25) hue-rotate(-10deg) brightness(1.04)' },
  ruby: { label: 'Rubí', category: 'Retrato', css: 'saturate(1.3) hue-rotate(-14deg) contrast(1.04)' },
  warmPortrait: { label: 'Retrato cálido', category: 'Retrato', css: 'sepia(0.24) saturate(1.12) brightness(1.03)' },

  /* --- Paisaje: valles, cumbres, desierto y bosque --- */
  forest: { label: 'Bosque', category: 'Paisaje', css: 'saturate(1.15) hue-rotate(28deg) brightness(0.98)' },
  valley: { label: 'Valle', category: 'Paisaje', css: 'saturate(1.2) hue-rotate(12deg) contrast(1.05)' },
  summit: { label: 'Cumbre', category: 'Paisaje', css: 'saturate(1.15) brightness(1.06) contrast(1.08)' },
  deepForest: { label: 'Bosque profundo', category: 'Paisaje', css: 'saturate(1.25) hue-rotate(30deg) brightness(0.94) contrast(1.06)' },
  desert: { label: 'Desierto', category: 'Paisaje', css: 'sepia(0.34) saturate(1.2) brightness(1.04)' },

  /* --- Neón: colores de escaparate y sintetizadores --- */
  cyber: { label: 'Cyber', category: 'Neón', css: 'saturate(1.7) hue-rotate(288deg) contrast(1.18)' },
  infrared: { label: 'Infrarrojo', category: 'Neón', css: 'saturate(2) hue-rotate(230deg) contrast(1.1)' },
  magenta: { label: 'Magenta', category: 'Neón', css: 'saturate(1.6) hue-rotate(300deg) contrast(1.12)' },
  acid: { label: 'Ácido', category: 'Neón', css: 'saturate(1.7) hue-rotate(70deg) contrast(1.1)' },
  plasma: { label: 'Plasma', category: 'Neón', css: 'saturate(1.55) hue-rotate(265deg) contrast(1.14) brightness(1.02)' },

  /* --- Clima: bruma, tormenta, lluvia y sol de invierno --- */
  mist: { label: 'Bruma', category: 'Clima', css: 'saturate(0.9) brightness(1.08) contrast(0.94) blur(0.3px)' },
  storm: { label: 'Tormenta', category: 'Clima', css: 'saturate(0.8) hue-rotate(196deg) contrast(1.14) brightness(0.9)' },
  drizzle: { label: 'Lluvia fina', category: 'Clima', css: 'saturate(0.85) brightness(0.97) contrast(1.05) blur(0.2px)' },
  winterSun: { label: 'Sol de invierno', category: 'Clima', css: 'brightness(1.1) saturate(0.92) hue-rotate(10deg) contrast(1.02)' },
  frost: { label: 'Escarcha', category: 'Clima', css: 'saturate(0.8) hue-rotate(14deg) brightness(1.12) contrast(0.98)' },
};

/* Filtros con el motor GL: el shader hace el trabajo real y el `css` es la
   vista previa de la tarjeta (aproximada) y la reserva sin WebGL. */
const GL_LINE = {
  vignette: { label: 'Viñeta', category: 'Textura', css: 'contrast(1.06) brightness(0.96)', gl: 'vignette' },
  grain: { label: 'Grano', category: 'Textura', css: 'contrast(1.08) saturate(0.96)', gl: 'grain' },
  chroma: { label: 'Croma', category: 'Textura', css: 'contrast(1.08) saturate(1.12)', gl: 'chromatic' },
  vhs: { label: 'VHS', category: 'Textura', css: 'saturate(1.18) contrast(1.06) hue-rotate(-4deg)', gl: 'vhs' },
  bloom: { label: 'Resplandor', category: 'Textura', css: 'brightness(1.1) contrast(0.94) saturate(1.06)', gl: 'bloom' },
  rgbSplit: { label: 'División RGB', category: 'Textura', css: 'contrast(1.1) saturate(1.15)', gl: 'rgb-split' },
  radialBlur: { label: 'Desenfoque radial', category: 'Textura', css: 'blur(0.5px) contrast(1.04)', gl: 'radial-blur' },
  directionalBlur: { label: 'Desenfoque direccional', category: 'Textura', css: 'blur(0.4px)', gl: 'directional-blur' },
  posterize: { label: 'Posterizar', category: 'Textura', css: 'contrast(1.3) saturate(1.35)', gl: 'posterize' },

  /* Los ocho shaders portados a mano, ya con aspecto propio: son la firma de
     la casa y funcionan sin red, sin carátulas y sin LUT externos. */
  burgundyHaze: { label: 'Bruma borgoña', category: 'Firma', css: 'sepia(0.35) hue-rotate(-18deg) saturate(1.3) contrast(1.05)', gl: 'bruma-borgona' },
  dreamSunset: { label: 'Atardecer de ensueño', category: 'Firma', css: 'sepia(0.3) saturate(1.45) hue-rotate(-14deg) brightness(1.05)', gl: 'atardecer-ensueno' },
  mistyGlow: { label: 'Brillo brumoso', category: 'Firma', css: 'brightness(1.14) contrast(0.92) saturate(1.1)', gl: 'brillo-brumoso' },
  pixelNoir: { label: 'Píxel noir', category: 'Firma', css: 'grayscale(1) contrast(1.35) brightness(0.95)', gl: 'pixel-noir' },
  retroLove: { label: 'Cariño retro', category: 'Firma', css: 'sepia(0.34) saturate(1.1) contrast(1.06) brightness(1.02)', gl: 'carino-retro' },
  khakiDream: { label: 'Sueño caqui', category: 'Firma', css: 'sepia(0.42) saturate(0.95) brightness(1.06) contrast(0.96)', gl: 'sueno-caqui' },
  gingerSoft: { label: 'Jengibre suave', category: 'Firma', css: 'sepia(0.3) saturate(1.2) hue-rotate(-8deg) brightness(1.04)', gl: 'jengibre-suave' },
  hauntedCanvas: { label: 'Lienzo embrujado', category: 'Firma', css: 'saturate(1.25) hue-rotate(255deg) contrast(1.12) brightness(0.96)', gl: 'lienzo-embrujado' },
};

export const FILTER_LINE = { ...COLOR_LINE, ...GL_LINE };

/* Comprobación de coherencia: ninguna categoría fuera de la lista y ningún
   filtro sin nombre. Se ejecuta al importar para que falle rápido en pruebas. */
for (const [key, entry] of Object.entries(FILTER_LINE)) {
  if (!FILTER_LINE_CATEGORIES.includes(entry.category)) {
    throw new Error(`filter-line: “${key}” tiene la categoría desconocida “${entry.category}”`);
  }
  if (!entry.label || !entry.css) {
    throw new Error(`filter-line: “${key}” necesita label y css`);
  }
  if (entry.gl && typeof entry.gl !== 'string') {
    throw new Error(`filter-line: “${key}” tiene un shader inválido`);
  }
}
