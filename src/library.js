/* Catálogo local de efectos, stickers, transiciones, subtítulos y avatares. */

/* Crea una ficha de efecto. `css` se aplica al clip, el resto lo interpreta
   el motor procedural de renderer.js (partículas, marcos, luz y movimiento). */
const fx = (label, category, icon, css = 'none', spec = {}) => ({
  label,
  category,
  icon,
  css,
  ...spec,
});

export const EFFECTS = {
  none: fx('Ninguno', 'Básicos', '∅'),

  /* ------------------------------ Tendencias ------------------------------ */
  tnd_chalk: fx('Grafiti de tiza', 'Tendencias', '✎', 'contrast(1.12) saturate(0.82)', { overlay: 'chalk', accent: '#f4f1e8' }),
  tnd_heart: fx('Corazón brillante', 'Tendencias', '💖', 'brightness(1.06) saturate(1.14)', { particles: 'heart', glyphs: ['❤', '💖', '💕'], count: 22, speed: 0.55, size: 40 }),
  tnd_sepia: fx('Sepia disco', 'Tendencias', '🪩', 'sepia(0.42) saturate(1.5) hue-rotate(-16deg) contrast(1.06)', { overlay: 'disco' }),
  tnd_action: fx('Barra de acción', 'Tendencias', '⚡', 'contrast(1.25) saturate(1.24)', { overlay: 'action', accent: '#ffcf3f' }),
  tnd_beat: fx('Latido afuera', 'Tendencias', '💓', 'contrast(1.08)', { move: 'heartbeat', overlay: 'pulse', accent: '#ff4d88' }),
  tnd_horror: fx('Falla de terror', 'Tendencias', '👁', 'contrast(1.3) saturate(0.68) brightness(0.92)', { overlay: 'glitch', accent: '#ff2f4f' }),
  tnd_mosaic: fx('Mosaico cinético', 'Tendencias', '▦', 'contrast(1.08) saturate(1.15)', { overlay: 'mosaic' }),
  tnd_doodle: fx('Garabato', 'Tendencias', '✏', 'saturate(1.18)', { overlay: 'doodle', accent: '#ffffff' }),
  tnd_laugh: fx('Amor de risa', 'Tendencias', '😂', 'brightness(1.08) saturate(1.1)', { particles: 'emoji', glyphs: ['😂', '❤', '✨'], count: 16, speed: 0.5, size: 42 }),
  tnd_chroma: fx('Cromático', 'Tendencias', '◑', 'saturate(1.5) contrast(1.06)', { overlay: 'chroma' }),
  tnd_print: fx('Impresión efímera', 'Tendencias', '🖨', 'contrast(1.2) saturate(0.9)', { overlay: 'print', accent: '#f3e9d2' }),
  tnd_retroShake: fx('Temblor retro', 'Tendencias', '📼', 'sepia(0.3) contrast(1.12)', { move: 'shake', amount: 0.35, overlay: 'grain' }),

  /* --------------------------------- Pato --------------------------------- */
  pat_feather: fx('Plumas al viento', 'Pato', '🪶', 'brightness(1.04)', { particles: 'emoji', glyphs: ['🪶', '🍃', '✨'], count: 18, speed: 0.35, size: 40 }),
  pat_beak: fx('Pico dorado', 'Pato', '🐤', 'sepia(0.24) saturate(1.3) brightness(1.06)', { overlay: 'gold' }),
  pat_happy: fx('Patito feliz', 'Pato', '🦆', 'saturate(1.2) brightness(1.05)', { particles: 'emoji', glyphs: ['🦆', '🐤', '💛'], count: 18, speed: 0.45, size: 44 }),
  pat_puddle: fx('Charco mágico', 'Pato', '💧', 'hue-rotate(175deg) saturate(1.25)', { overlay: 'ripple', accent: '#58d7ff' }),
  pat_egg: fx('Huevo sorpresa', 'Pato', '🥚', 'brightness(1.07)', { particles: 'emoji', glyphs: ['🥚', '✨', '🐣'], count: 16, speed: 0.4, size: 40 }),
  pat_rain: fx('Lluvia de patos', 'Pato', '🌧', 'saturate(1.12)', { particles: 'emoji', glyphs: ['🦆', '🐤', '💛'], count: 28, speed: 0.75, size: 38, spawn: 'top' }),
  pat_flap: fx('Aleteo', 'Pato', '🪽', 'brightness(1.03)', { move: 'flap', overlay: 'breeze' }),
  pat_bubbles: fx('Burbujas de pato', 'Pato', '🫧', 'brightness(1.08) saturate(1.08)', { particles: 'bubble', count: 26, speed: 0.35, size: 34, colors: ['#bff4ff', '#ffffff', '#8ee7ff'] }),

  /* ------------------------------- Halloween ------------------------------ */
  hal_pumpkin: fx('Calabaza', 'Halloween', '🎃', 'saturate(1.24) contrast(1.05)', { particles: 'emoji', glyphs: ['🎃', '🍬', '🕸'], count: 20, speed: 0.35, size: 40 }),
  hal_ghost: fx('Fantasmas', 'Halloween', '👻', 'brightness(1.05) contrast(0.98)', { particles: 'emoji', glyphs: ['👻', '💀', '🌫'], count: 20, speed: 0.3, size: 44 }),
  hal_web: fx('Telaraña', 'Halloween', '🕸', 'contrast(1.1)', { frame: 'web', accent: '#dfe6ef' }),
  hal_bats: fx('Murciélagos', 'Halloween', '🦇', 'brightness(0.96) contrast(1.08)', { particles: 'emoji', glyphs: ['🦇', '🌙', '✨'], count: 20, speed: 0.45, size: 38 }),
  hal_fog: fx('Niebla', 'Halloween', '🌫', 'contrast(1.04) saturate(0.85)', { overlay: 'fog' }),
  hal_lightning: fx('Relámpago', 'Halloween', '⚡', 'brightness(1.1) contrast(1.16)', { overlay: 'lightning' }),
  hal_eyes: fx('Ojos en la oscuridad', 'Halloween', '👀', 'brightness(0.8) contrast(1.2)', { overlay: 'eyes' }),
  hal_blood: fx('Sangre', 'Halloween', '🩸', 'saturate(1.42) contrast(1.08)', { overlay: 'blood', accent: '#a40012' }),
  hal_skull: fx('Calavera', 'Halloween', '💀', 'grayscale(0.35) contrast(1.16)', { particles: 'emoji', glyphs: ['💀', '🦴', '✨'], count: 16, speed: 0.32, size: 40 }),
  hal_witch: fx('Brujería', 'Halloween', '🧙', 'hue-rotate(-12deg) saturate(1.24)', { particles: 'emoji', glyphs: ['🧙', '✨', '🕯'], count: 16, speed: 0.4, size: 42, overlay: 'magic' }),

  /* ------------------------------- Especiales ----------------------------- */
  esp_dreamy: fx('Soñador', 'Especiales', '✧', 'blur(1.1px) brightness(1.12) saturate(1.18)', { overlay: 'dreamy' }),
  esp_confetti: fx('Confeti', 'Especiales', '🎊', 'saturate(1.15)', { particles: 'confetti', count: 40, speed: 0.55, size: 22, colors: ['#ff4d6d', '#ffd166', '#4cc9f0', '#8ac926', '#c77dff'] }),
  esp_stars: fx('Estrellas', 'Especiales', '⭐', 'brightness(1.06)', { particles: 'star', count: 26, speed: 0.4, size: 34, colors: ['#fff3a8', '#ffffff', '#ffd166'] }),
  esp_glitter: fx('Purpurina', 'Especiales', '✨', 'brightness(1.08) saturate(1.1)', { particles: 'spark', count: 48, speed: 0.3, size: 16, colors: ['#ffffff', '#ffe8a3', '#ffc6f2'] }),
  esp_rainbow: fx('Arcoíris', 'Especiales', '🌈', 'saturate(1.35)', { frame: 'rainbow', overlay: 'rainbow' }),
  esp_fireworks: fx('Fuegos artificiales', 'Especiales', '🎆', 'brightness(1.05)', { particles: 'firework', count: 30, speed: 0.6, size: 26, colors: ['#ffd166', '#ff5d8f', '#7bdff2', '#c8ffbe'] }),
  esp_hearts: fx('Corazones', 'Especiales', '❤', 'saturate(1.18)', { particles: 'heart', glyphs: ['❤', '💖', '💘'], count: 24, speed: 0.45, size: 38 }),
  esp_notes: fx('Notas musicales', 'Especiales', '🎵', 'brightness(1.04)', { particles: 'emoji', glyphs: ['🎵', '🎶', '♪'], count: 22, speed: 0.4, size: 36 }),
  esp_petals: fx('Pétalos', 'Especiales', '🌸', 'saturate(1.2)', { particles: 'emoji', glyphs: ['🌸', '🌺', '🌼'], count: 24, speed: 0.35, size: 36 }),
  esp_candles: fx('Velas', 'Especiales', '🕯', 'brightness(1.04)', { particles: 'emoji', glyphs: ['🕯', '✨'], count: 14, speed: 0.25, size: 40 }),
  esp_magic: fx('Magia', 'Especiales', '🪄', 'saturate(1.2) brightness(1.05)', { particles: 'emoji', glyphs: ['✨', '🪄', '🔮'], count: 22, speed: 0.5, size: 38, overlay: 'magic' }),

  /* ------------------------------ Black Friday ---------------------------- */
  blk_sale: fx('Oferta flash', 'Black Friday', '🏷', 'brightness(1.1) contrast(1.06)', { overlay: 'sale', accent: '#ffd400' }),
  blk_tags: fx('Etiquetas', 'Black Friday', '🔖', 'saturate(1.14)', { particles: 'emoji', glyphs: ['🏷', '💸', '🛍'], count: 20, speed: 0.45, size: 40 }),
  blk_money: fx('Lluvia de billetes', 'Black Friday', '💵', 'saturate(1.16)', { particles: 'emoji', glyphs: ['💵', '💰', '💸'], count: 26, speed: 0.65, size: 40, spawn: 'top' }),
  blk_cart: fx('Carrito', 'Black Friday', '🛒', 'brightness(1.05)', { particles: 'emoji', glyphs: ['🛒', '📦', '🎁'], count: 18, speed: 0.4, size: 40 }),
  blk_neon: fx('Neón oferta', 'Black Friday', '💥', 'saturate(1.85) contrast(1.12)', { overlay: 'neon', accent: '#ffd400' }),
  blk_flash: fx('Flash', 'Black Friday', '📸', 'brightness(1.2) contrast(1.15)', { overlay: 'strobe' }),
  blk_arrows: fx('Flechas', 'Black Friday', '➡', 'contrast(1.1)', { particles: 'emoji', glyphs: ['➡', '⬇', '💥'], count: 18, speed: 0.5, size: 34 }),
  blk_coupon: fx('Cupón', 'Black Friday', '🎟', 'saturate(1.1)', { frame: 'coupon', overlay: 'sale', accent: '#ffd400' }),

  /* -------------------------------- Clásica ------------------------------- */
  blur: fx('Desenfoque', 'Clásica', '◌', 'blur(2.4px)'),
  cls_vintage: fx('Vintage', 'Clásica', '📷', 'sepia(0.35) contrast(1.05) saturate(0.9)', { overlay: 'vignette' }),
  cls_polaroid: fx('Polaroid', 'Clásica', '🖼', 'brightness(1.04) contrast(0.98)', { frame: 'polaroid' }),
  cls_film: fx('Película', 'Clásica', '🎞', 'contrast(1.04) saturate(0.94)', { frame: 'film', overlay: 'grain' }),
  cls_bw: fx('Blanco y negro', 'Clásica', '◐', 'grayscale(1) contrast(1.2)'),
  cls_sepia: fx('Sepia', 'Clásica', '🟤', 'sepia(0.8) contrast(1.05)'),
  cls_flash: fx('Destello', 'Clásica', '☀', 'brightness(1.45) contrast(1.2)', { overlay: 'strobe' }),

  /* ---------------------------- Recorte mágico ---------------------------- */
  cut_neon: fx('Recorte neón', 'Recorte mágico', '🟣', 'contrast(1.1) saturate(1.16)', { overlay: 'neonOutline', accent: '#ff4de1' }),
  cut_silhouette: fx('Silueta', 'Recorte mágico', '👤', 'brightness(0.72) contrast(1.28)', { overlay: 'silhouette' }),
  cut_outline: fx('Contorno', 'Recorte mágico', '◯', 'contrast(1.1)', { overlay: 'outline', accent: '#ffffff' }),
  cut_chalk: fx('Tiza', 'Recorte mágico', '🖍', 'contrast(1.16) saturate(0.86)', { overlay: 'chalk', accent: '#f2ede1' }),
  cut_double: fx('Doble borde', 'Recorte mágico', '⧉', 'contrast(1.08)', { overlay: 'doubleBorder', accent: '#ffd166' }),
  cut_shadow: fx('Sombra', 'Recorte mágico', '🌑', 'brightness(0.98) contrast(1.08)', { overlay: 'shadow' }),
  cut_comic: fx('Cómic', 'Recorte mágico', '💬', 'contrast(1.3) saturate(1.5)', { overlay: 'halftone', frame: 'comic' }),
  cut_watercolor: fx('Acuarela', 'Recorte mágico', '🎨', 'saturate(1.18) blur(0.4px) brightness(1.04)', { overlay: 'watercolor' }),

  /* --------------------------------- NUEVO -------------------------------- */
  new_3d: fx('Giro 3D', 'NUEVO', '🧊', 'saturate(1.1)', { move: 'flip' }),
  new_aura: fx('Aura', 'NUEVO', '🌟', 'brightness(1.06) saturate(1.12)', { overlay: 'aura', accent: '#8ffcff' }),
  new_glass: fx('Cristal', 'NUEVO', '💎', 'brightness(1.08) saturate(1.08)', { overlay: 'glass' }),
  new_holo: fx('Holograma', 'NUEVO', '🪩', 'hue-rotate(20deg) saturate(1.4)', { overlay: 'hologram' }),
  new_prism: fx('Prisma', 'NUEVO', '🔷', 'saturate(1.24)', { overlay: 'prism' }),
  new_pulse: fx('Pulso', 'NUEVO', '💫', 'contrast(1.08)', { move: 'heartbeat', overlay: 'pulse', accent: '#7defff' }),
  new_particles: fx('Partículas', 'NUEVO', '✨', 'brightness(1.06)', { particles: 'spark', count: 36, speed: 0.35, size: 18, colors: ['#ffffff', '#7defff', '#ffd166'] }),
  new_badge: fx('Nuevo', 'NUEVO', '🆕', 'brightness(1.05)', { frame: 'badge', overlay: 'sale', accent: '#00e5d0' }),

  /* ------------------------------- Bombazos -------------------------------- */
  bmb_explosion: fx('Explosión', 'Bombazos', '💥', 'contrast(1.18) saturate(1.3)', { particles: 'explosion', count: 34, speed: 0.9, size: 34, colors: ['#ffd166', '#ff7b00', '#ff3d00'], move: 'impact' }),
  bmb_confetti: fx('Confeti explosivo', 'Bombazos', '🎉', 'saturate(1.3)', { particles: 'confetti', count: 52, speed: 0.9, size: 24, colors: ['#ff4d6d', '#ffd166', '#4cc9f0', '#8ac926', '#c77dff'] }),
  bmb_flash: fx('Flash bomba', 'Bombazos', '🔆', 'brightness(1.5) contrast(1.2)', { overlay: 'strobe' }),
  bmb_boom: fx('Cómic boom', 'Bombazos', '💬', 'contrast(1.35) saturate(1.4)', { frame: 'comic', overlay: 'boom', accent: '#ffd166' }),
  bmb_fire: fx('Fuego', 'Bombazos', '🔥', 'saturate(1.35) contrast(1.08)', { particles: 'emoji', glyphs: ['🔥', '✨'], count: 30, speed: 0.8, size: 38, overlay: 'fire' }),
  bmb_sparks: fx('Chispas', 'Bombazos', '✨', 'brightness(1.12)', { particles: 'spark', count: 56, speed: 0.9, size: 18, colors: ['#fff3a8', '#ffd166', '#ffffff'] }),
  bmb_impact: fx('Impacto', 'Bombazos', '💢', 'contrast(1.25)', { move: 'impact', overlay: 'impact', accent: '#ffd166' }),
  bmb_quake: fx('Terremoto', 'Bombazos', '🌋', 'contrast(1.12) saturate(1.18)', { move: 'shake', amount: 1.6 }),

  /* ---------------------------- Intro y cierre ---------------------------- */
  int_countdown: fx('Cuenta atrás', 'Intro y cierre', '⏱', 'contrast(1.1)', { overlay: 'countdown' }),
  int_curtain: fx('Cortinilla', 'Intro y cierre', '🎭', 'brightness(0.98)', { overlay: 'curtain' }),
  int_wipe: fx('Barrido', 'Intro y cierre', '➡', 'contrast(1.06)', { overlay: 'wipe' }),
  int_flash: fx('Flash de apertura', 'Intro y cierre', '📸', 'brightness(1.4)', { overlay: 'strobe' }),
  int_zoom: fx('Zoom de entrada', 'Intro y cierre', '🔍', 'contrast(1.06)', { move: 'zoomIn' }),
  int_glitch: fx('Glitch de entrada', 'Intro y cierre', '📺', 'contrast(1.14)', { overlay: 'glitch' }),
  int_close: fx('Cierre', 'Intro y cierre', '⬛', 'brightness(0.92)', { overlay: 'close' }),
  int_open: fx('Apertura', 'Intro y cierre', '🌅', 'brightness(1.08)', { frame: 'open', overlay: 'flare' }),

  /* --------------------------- Marcos aleatorios -------------------------- */
  frm_polaroid: fx('Polaroid', 'Marcos aleatorios', '🖼', 'contrast(0.99) brightness(1.04)', { frame: 'polaroid' }),
  frm_film: fx('Película', 'Marcos aleatorios', '🎞', 'saturate(0.94)', { frame: 'film' }),
  frm_tape: fx('Cinta', 'Marcos aleatorios', '🩹', 'brightness(1.03)', { frame: 'tape' }),
  frm_neon: fx('Neón', 'Marcos aleatorios', '🟪', 'saturate(1.2)', { frame: 'neon', accent: '#ff4de1' }),
  frm_floral: fx('Flores', 'Marcos aleatorios', '🌸', 'saturate(1.12)', { frame: 'floral' }),
  frm_torn: fx('Papel rasgado', 'Marcos aleatorios', '📄', 'contrast(1.02)', { frame: 'torn' }),
  frm_wood: fx('Madera', 'Marcos aleatorios', '🪵', 'sepia(0.2) contrast(1.04)', { frame: 'wood' }),
  frm_dots: fx('Puntos', 'Marcos aleatorios', '⚪', 'brightness(1.04)', { frame: 'dots', accent: '#ffffff' }),
  frm_rainbow: fx('Arcoíris', 'Marcos aleatorios', '🌈', 'saturate(1.25)', { frame: 'rainbow' }),

  /* --------------------------------- Fiesta ------------------------------- */
  fst_confetti: fx('Confeti', 'Fiesta', '🎊', 'saturate(1.24)', { particles: 'confetti', count: 44, speed: 0.7, size: 22, colors: ['#ff4d6d', '#ffd166', '#4cc9f0', '#8ac926', '#c77dff'] }),
  fst_disco: fx('Bolas disco', 'Fiesta', '🪩', 'saturate(1.3) brightness(1.06)', { particles: 'emoji', glyphs: ['🪩', '✨'], count: 18, speed: 0.35, size: 40, overlay: 'disco' }),
  fst_lights: fx('Luces de fiesta', 'Fiesta', '🔆', 'saturate(1.25)', { overlay: 'party' }),
  fst_balloons: fx('Globos', 'Fiesta', '🎈', 'saturate(1.22)', { particles: 'emoji', glyphs: ['🎈', '🎉', '✨'], count: 24, speed: 0.38, size: 42 }),
  fst_streamers: fx('Serpentina', 'Fiesta', '🎉', 'saturate(1.2)', { particles: 'confetti', count: 36, speed: 0.5, size: 18, colors: ['#ff4d6d', '#ffd166', '#4cc9f0', '#8ac926'] }),
  fst_dj: fx('DJ', 'Fiesta', '🎧', 'saturate(1.28) contrast(1.06)', { particles: 'emoji', glyphs: ['🎧', '🎵', '🔊'], count: 20, speed: 0.5, size: 38 }),
  fst_fireworks: fx('Fuegos', 'Fiesta', '🎆', 'brightness(1.06)', { particles: 'firework', count: 34, speed: 0.7, size: 26, colors: ['#ffd166', '#ff5d8f', '#7bdff2', '#c8ffbe'] }),
  fst_glitter: fx('Brillos', 'Fiesta', '✨', 'brightness(1.08)', { particles: 'spark', count: 48, speed: 0.4, size: 16, colors: ['#ffffff', '#ffe8a3', '#ffc6f2'] }),

  /* ------------------------------- Movimiento ----------------------------- */
  zoomPulse: fx('Latido', 'Movimiento', '◎', 'none', { move: 'zoom' }),
  shake: fx('Vibración', 'Movimiento', '↔', 'none', { move: 'shake' }),
  mov_wave: fx('Onda', 'Movimiento', '〰', 'none', { move: 'wave' }),
  mov_spin: fx('Giro', 'Movimiento', '🔄', 'none', { move: 'spin' }),
  mov_blur: fx('Desenfoque de movimiento', 'Movimiento', '💨', 'blur(1.1px)', { move: 'blur', ghost: 3 }),
  mov_drift: fx('Deriva', 'Movimiento', '🌊', 'none', { move: 'drift' }),
  mov_swing: fx('Balanceo', 'Movimiento', '🎐', 'none', { move: 'swing' }),
  mov_float: fx('Flotar', 'Movimiento', '🫧', 'none', { move: 'float' }),

  /* ------------------------------ Movimiento 2 ---------------------------- */
  mv2_pan: fx('Paneo', 'Movimiento 2', '↔', 'none', { move: 'pan' }),
  mv2_tilt: fx('Inclinación', 'Movimiento 2', '↕', 'none', { move: 'tilt' }),
  mv2_roll: fx('Rodillo', 'Movimiento 2', '⤾', 'none', { move: 'roll' }),
  mv2_orbit: fx('Órbita', 'Movimiento 2', '🪐', 'none', { move: 'orbit' }),
  mv2_jelly: fx('Gelatina', 'Movimiento 2', '🍮', 'none', { move: 'jelly' }),
  mv2_rebound: fx('Rebote', 'Movimiento 2', '🏀', 'none', { move: 'rebound' }),
  mv2_parallax: fx('Paralaje', 'Movimiento 2', '🗻', 'none', { move: 'parallax' }),
  mirror: fx('Espejo', 'Movimiento 2', '◐', 'none', { mirror: true }),

  /* -------------------------------- Ediciones ----------------------------- */
  edt_vhs: fx('VHS', 'Ediciones', '📼', 'contrast(1.1) saturate(0.9)', { overlay: 'vhs' }),
  edt_glitch: fx('Glitch', 'Ediciones', '🛠', 'contrast(1.16) saturate(1.08)', { overlay: 'glitch' }),
  edt_rgb: fx('Separación RGB', 'Ediciones', '🌈', 'drop-shadow(2px 0 rgba(255,0,0,0.5)) drop-shadow(-2px 0 rgba(0,255,255,0.5))'),
  scanlines: fx('Líneas TV', 'Ediciones', '☰', 'contrast(1.12) saturate(0.85)', { overlay: 'scanlines' }),
  edt_pixel: fx('Píxel', 'Ediciones', '▦', 'contrast(1.3) saturate(1.1)', { overlay: 'pixel' }),
  edt_vcr: fx('VCR', 'Ediciones', '📹', 'contrast(1.08)', { overlay: 'vcr' }),
  edt_tracking: fx('Tracking', 'Ediciones', '📺', 'contrast(1.12)', { overlay: 'tracking' }),
  edt_datamosh: fx('Datamosh', 'Ediciones', '🧩', 'contrast(1.14) saturate(1.16)', { overlay: 'datamosh', move: 'glitch' }),

  /* ----------------------------------- 3D --------------------------------- */
  d3_anaglyph: fx('Anaglifo', '3D', '👓', 'drop-shadow(2px 0 rgba(255,0,0,0.55)) drop-shadow(-2px 0 rgba(0,255,255,0.55))'),
  d3_cube: fx('Cubo', '3D', '🧊', 'saturate(1.08)', { move: 'cube' }),
  d3_flip: fx('Giro 3D', '3D', '🔄', 'saturate(1.08)', { move: 'flip' }),
  d3_holo: fx('Holograma', '3D', '🪩', 'hue-rotate(24deg) saturate(1.4)', { overlay: 'hologram' }),
  d3_depth: fx('Profundidad', '3D', '🔭', 'brightness(1.04) contrast(1.1)', { overlay: 'depth' }),
  d3_tilt: fx('Inclinación 3D', '3D', '📐', 'none', { move: 'tilt3d' }),
  d3_parallax: fx('Paralaje 3D', '3D', '🗻', 'none', { move: 'parallax' }),
  d3_extrude: fx('Extrusión', '3D', '⬛', 'contrast(1.08)', { overlay: 'extrude' }),

  /* ---------------------------------- Luz --------------------------------- */
  glow: fx('Resplandor', 'Luz', '✦', 'brightness(1.16) saturate(1.2)', { overlay: 'glow', accent: '#7defff' }),
  neon: fx('Neón', 'Luz', '⚡', 'saturate(1.9) contrast(1.08) hue-rotate(-12deg)', { overlay: 'neon', accent: '#ff4de1' }),
  luz_leak: fx('Fuga de luz', 'Luz', '🌅', 'brightness(1.06) saturate(1.1)', { overlay: 'lightLeak' }),
  luz_flare: fx('Destello de lente', 'Luz', '🔆', 'brightness(1.08)', { overlay: 'flare' }),
  luz_spot: fx('Foco', 'Luz', '🔦', 'contrast(1.08)', { overlay: 'spotlight' }),
  strobe: fx('Destello', 'Luz', '☀', 'brightness(1.45) contrast(1.2)', { overlay: 'strobe' }),
  luz_bokeh: fx('Bokeh', 'Luz', '🫧', 'brightness(1.05) saturate(1.08)', { overlay: 'bokeh' }),
  luz_rays: fx('Rayos de sol', 'Luz', '🌞', 'brightness(1.08) saturate(1.1)', { overlay: 'rays' }),
  luz_aurora: fx('Aurora', 'Luz', '🌌', 'saturate(1.3) brightness(1.04)', { overlay: 'aurora' }),
  luz_flash: fx('Flash', 'Luz', '📸', 'brightness(1.5) contrast(1.16)', { overlay: 'flash' }),

  /* --------------------------------- Retro -------------------------------- */
  oldFilm: fx('Cine viejo', 'Retro', '🎞', 'sepia(0.45) contrast(1.02) brightness(0.94)', { overlay: 'oldFilm' }),
  vignette: fx('Viñeta', 'Retro', '◍', 'contrast(1.06) brightness(0.98)', { overlay: 'vignette' }),
  grain: fx('Grano', 'Retro', '▦', 'contrast(1.08) saturate(0.9)', { overlay: 'grain' }),
  ret_scan: fx('Líneas TV', 'Retro', '☰', 'contrast(1.1) saturate(0.86)', { overlay: 'scanlines' }),
  ret_sepia: fx('Sepia', 'Retro', '🟤', 'sepia(0.85) contrast(1.05)'),
  ret_vhs: fx('VHS retro', 'Retro', '📼', 'contrast(1.08) saturate(0.88)', { overlay: 'vhs' }),
  ret_polaroid: fx('Polaroid', 'Retro', '🖼', 'brightness(1.04) contrast(0.98)', { frame: 'polaroid' }),
  ret_projector: fx('Proyector', 'Retro', '🎥', 'sepia(0.3) contrast(1.08)', { overlay: 'projector' }),
  ret_dust: fx('Polvo', 'Retro', '🌫', 'contrast(1.06) saturate(0.9)', { particles: 'dust', count: 36, speed: 0.2, size: 4, colors: ['#ffffff', '#d8c9a8'] }),
  chromatic: fx('Cromático', 'Retro', '◑', 'saturate(1.4)', { overlay: 'chroma' }),

  /* --------------------------- Efectos corporales -------------------------- */
  body_aura: fx('Aura corporal', 'Efectos corporales', '🌟', 'brightness(1.06) saturate(1.12)', { overlay: 'aura', accent: '#8ffcff' }),
  body_slim: fx('Silueta fina', 'Efectos corporales', '🪞', 'contrast(1.1)', { overlay: 'slim' }),
  body_glow: fx('Brillo corporal', 'Efectos corporales', '💡', 'brightness(1.1) saturate(1.1)', { overlay: 'bodyGlow' }),
  body_comic: fx('Cómic corporal', 'Efectos corporales', '💥', 'contrast(1.3) saturate(1.4)', { overlay: 'halftone' }),
  body_outline: fx('Contorno corporal', 'Efectos corporales', '◯', 'contrast(1.1)', { overlay: 'outline' }),
  body_thermal: fx('Térmico', 'Efectos corporales', '🌡', 'invert(1) hue-rotate(180deg) saturate(2.4) contrast(1.4)', { overlay: 'thermal' }),
  body_xray: fx('Rayos X', 'Efectos corporales', '🦴', 'invert(1) grayscale(1) contrast(1.6)', { overlay: 'xray' }),
  body_clone: fx('Clon', 'Efectos corporales', '👥', 'none', { move: 'cloneGhost' }),
  body_sticker: fx('Sticker corporal', 'Efectos corporales', '💪', 'saturate(1.2)', { particles: 'emoji', glyphs: ['✨', '💪', '🔥'], count: 16, speed: 0.4, size: 38 }),
  body_neon: fx('Neón corporal', 'Efectos corporales', '🟪', 'saturate(1.3)', { overlay: 'neonOutline', accent: '#00e5ff' }),
};

export const EFFECT_CATEGORIES = [
  'Tendencias',
  'Pato',
  'Halloween',
  'Especiales',
  'Black Friday',
  'Clásica',
  'Recorte mágico',
  'NUEVO',
  'Bombazos',
  'Intro y cierre',
  'Marcos aleatorios',
  'Fiesta',
  'Movimiento',
  'Movimiento 2',
  'Ediciones',
  '3D',
  'Luz',
  'Retro',
  'Efectos corporales',
];

export const STICKER_CATEGORIES = [
  {
    name: 'Tendencias',
    items: ['✨', '🔥', '❤️', '😂', '🎉', '⭐', '💫', '🌈', '💥', '👑', '💯', '🫶', '😎', '🥳', '💅'],
  },
  {
    name: 'Generado por IA',
    items: ['🤖', '✨', '🪄', '🧠', '💫', '🔮', '🛸', '🦾', '🧬', '🌌', '🪐', '⚡'],
  },
  {
    name: 'Pato',
    items: ['🦆', '🐤', '🐥', '🥚', '🛁', '💛', '🌼', '🎀', '🍋', '🫧'],
  },
  {
    name: 'Halloween',
    items: ['🎃', '👻', '🕷️', '🕸️', '🦇', '💀', '🧙‍♀️', '🧛‍♂️', '🧟‍♂️', '🍬', '🌙', '🕯️'],
  },
  {
    name: 'Especiales',
    items: ['💫', '🪩', '🎇', '🎆', '🌠', '🦄', '🔮', '✨', '🪬', '🧿'],
  },
  {
    name: 'Black Friday',
    items: ['🛍️', '🛒', '💸', '🏷️', '💰', '📦', '🎁', '💳', '🤑', '🔖'],
  },
  {
    name: 'Clásica',
    items: ['❤️', '🔥', '🎉', '⭐', '🌈', '✨', '💯', '👍', '🎵', '🔆'],
  },
  {
    name: 'NUEVO',
    items: ['🆕', '✨', '🪄', '💫', '🌟', '🫧', '🧿', '🩷', '🪅', '🎊'],
  },
  {
    name: 'Meme de animales',
    items: ['🙈', '🙉', '🙊', '🐶', '🐱', '🦄', '🐸', '🐼', '🦦', '🦥', '🐔', '🦆'],
  },
  {
    name: 'Bombazos',
    items: ['💥', '💣', '🧨', '⚡', '🔥', '💫', '🎆', '🚀', '💢', '🌪️'],
  },
  {
    name: 'Battle Royale',
    items: ['🎮', '🔫', '🛡️', '🏆', '⚔️', '💣', '🎯', '🪖', '🥇', '🕹️'],
  },
  {
    name: 'Iconos',
    items: ['✅', '❌', '❗', '❓', '💤', '💬', '🔴', '🟢', '🔵', '⭐', '🔔', '⚙️'],
  },
  {
    name: 'Emoji',
    items: ['😀', '😂', '😍', '😎', '🥺', '😭', '🤯', '😴', '🤔', '😱', '🥳', '😇', '🫠', '🤩'],
  },
  {
    name: 'Estilo',
    items: ['🕶️', '👑', '💎', '💅', '🎩', '👠', '👜', '💄', '🧣', '💍'],
  },
  {
    name: 'Cubiertas',
    items: ['📱', '🎨', '🖼️', '📸', '🎞️', '🪞', '🛋️', '🧵', '🖌️', '🪟'],
  },
  {
    name: 'Equipaje',
    items: ['🧳', '✈️', '🗺️', '🎒', '🏝️', '🚢', '🗽', '🎫', '🚂', '🌍'],
  },
  {
    name: 'Amor',
    items: ['❤️', '🧡', '💛', '💚', '💙', '💜', '🖤', '🤍', '💕', '💞', '💘', '💖', '💗', '💓', '💝', '💟'],
  },
  {
    name: 'LETTERS',
    items: ['🅰️', '🅱️', '🅾️', '🆎', '🆑', '🅿️', '🔠', '🔡', '🔤', '🆒'],
  },
  {
    name: 'Formas',
    items: ['🔺', '🔻', '🔶', '🔷', '🔸', '🔹', '🔴', '🟠', '🟡', '🟢', '🔵', '🟣', '⚫', '⚪', '⬛', '⬜'],
  },
];

export const STICKERS = STICKER_CATEGORIES.flatMap((category) => category.items);

export const TRANSITIONS = [
  { id: 'fade', label: 'Fundido', category: 'Básicas', className: 'tr-fade' },
  { id: 'dissolve', label: 'Disolver', category: 'Básicas', className: 'tr-dissolve' },
  { id: 'black', label: 'Fundido a negro', category: 'Básicas', className: 'tr-black' },
  { id: 'white', label: 'Fundido a blanco', category: 'Básicas', className: 'tr-white' },
  { id: 'slide-left', label: 'Deslizar ←', category: 'Movimiento', className: 'tr-left' },
  { id: 'slide-right', label: 'Deslizar →', category: 'Movimiento', className: 'tr-right' },
  { id: 'slide-up', label: 'Deslizar ↑', category: 'Movimiento', className: 'tr-up' },
  { id: 'slide-down', label: 'Deslizar ↓', category: 'Movimiento', className: 'tr-down' },
  { id: 'zoom-in', label: 'Acercar', category: 'Zoom', className: 'tr-zoomin' },
  { id: 'zoom-out', label: 'Alejar', category: 'Zoom', className: 'tr-zoomout' },
  { id: 'spin', label: 'Girar', category: 'Zoom', className: 'tr-spin' },
  { id: 'wipe-left', label: 'Barrido ←', category: 'Cortinilla', className: 'tr-wipeleft' },
  { id: 'wipe-right', label: 'Barrido →', category: 'Cortinilla', className: 'tr-wiperight' },
  { id: 'blur', label: 'Desenfoque', category: 'Cortinilla', className: 'tr-blur' },
  { id: 'flash', label: 'Flash', category: 'Cortinilla', className: 'tr-flash' },
  { id: 'none', label: 'Sin transición', category: 'Básicas', className: 'tr-none' },
];

export const TRANSITION_IDS = TRANSITIONS.map((item) => item.id);

/* Presets de ajuste originales de cupcraft. Cada uno fija valores de
   brillo/contraste/saturación/gris/sepia/tono sobre el clip seleccionado. */
export const ADJUST_PRESETS = [
  { id: 'custom', label: 'Ajuste personalizado', category: 'Tuyos', adjust: null },
  { id: 'none', label: 'Ninguno', category: 'Básicos', adjust: {} },
  { id: 'clear', label: 'Limpiar', category: 'Básicos', adjust: { brightness: 105, contrast: 108, saturate: 106 } },
  { id: 'soft', label: 'Suave', category: 'Básicos', adjust: { brightness: 107, contrast: 93, saturate: 95 } },
  { id: 'punch', label: 'Definido', category: 'Básicos', adjust: { brightness: 100, contrast: 122, saturate: 112 } },
  { id: 'vivid', label: 'Vívido', category: 'Color', adjust: { brightness: 103, contrast: 114, saturate: 140 } },
  { id: 'warm', label: 'Cálido', category: 'Color', adjust: { brightness: 106, contrast: 104, saturate: 122, sepia: 22 } },
  { id: 'cool', label: 'Frío', category: 'Color', adjust: { brightness: 101, contrast: 107, saturate: 96, hue: -12 } },
  { id: 'bw', label: 'Blanco y negro', category: 'Color', adjust: { brightness: 103, contrast: 118, grayscale: 100 } },
  { id: 'sepia', label: 'Sepia', category: 'Color', adjust: { brightness: 105, contrast: 102, sepia: 78, saturate: 88 } },
  { id: 'highkey', label: 'Clave alta', category: 'Luz', adjust: { brightness: 118, contrast: 94, saturate: 102 } },
  { id: 'lowkey', label: 'Clave baja', category: 'Luz', adjust: { brightness: 84, contrast: 116, saturate: 96 } },
  { id: 'backlight', label: 'Contraluz', category: 'Luz', adjust: { brightness: 92, contrast: 128, saturate: 108 } },
  { id: 'fade', label: 'Desvanecido', category: 'Cine', adjust: { brightness: 110, contrast: 84, saturate: 86 } },
  { id: 'teal', label: 'Teal & Orange', category: 'Cine', adjust: { brightness: 102, contrast: 116, saturate: 126, hue: -8 } },
  { id: 'noir', label: 'Noir', category: 'Cine', adjust: { brightness: 92, contrast: 134, grayscale: 100 } },
  { id: 'retro', label: 'Retrô', category: 'Cine', adjust: { brightness: 108, contrast: 96, sepia: 42, saturate: 112 } },
  { id: 'night', label: 'Noche', category: 'Cine', adjust: { brightness: 78, contrast: 112, saturate: 88, hue: 10 } },
  { id: 'portrait', label: 'Retrato', category: 'Detalle', adjust: { brightness: 108, contrast: 96, saturate: 104 } },
  { id: 'food', label: 'Comida', category: 'Detalle', adjust: { brightness: 106, contrast: 110, saturate: 132, sepia: 8 } },
  { id: 'landscape', label: 'Paisaje', category: 'Detalle', adjust: { brightness: 104, contrast: 112, saturate: 124, hue: -4 } },
  { id: 'product', label: 'Producto', category: 'Detalle', adjust: { brightness: 114, contrast: 106, saturate: 98 } },
];

export const ADJUST_CATEGORIES = ['Tuyos', 'Básicos', 'Color', 'Luz', 'Cine', 'Detalle'];

export const CAPTION_STYLES = [
  { id: 'clean', label: 'Limpio', size: 56, weight: 600, color: '#ffffff', background: 'transparent', shadow: true, highlight: '' },
  { id: 'box', label: 'Caja', size: 52, weight: 700, color: '#ffffff', background: '#000000', shadow: false, highlight: '' },
  { id: 'yellow', label: 'Amarillo', size: 58, weight: 800, color: '#ffe34d', background: 'transparent', shadow: true, highlight: '' },
  { id: 'karaoke', label: 'Karaoke', size: 58, weight: 800, color: '#ffffff', background: 'transparent', shadow: true, highlight: '#00e5d0' },
  { id: 'minimal', label: 'Minimal', size: 46, weight: 500, color: '#f2f2f2', background: 'transparent', shadow: false, highlight: '' },
  { id: 'neon', label: 'Neón', size: 58, weight: 800, color: '#8ffcff', background: 'transparent', shadow: true, highlight: '' },
];

export const AVATAR_PRESETS = [
  { id: 'presenter', emoji: '🧑‍💼', name: 'Presentador', color: '#1d6d78' },
  { id: 'creator', emoji: '🧑‍🎤', name: 'Creador', color: '#6d3f8f' },
  { id: 'teacher', emoji: '🧑‍🏫', name: 'Profesor', color: '#2f5b8f' },
  { id: 'host', emoji: '🧑‍🚀', name: 'Anfitrión', color: '#8f4b2f' },
  { id: 'reporter', emoji: '👩‍💼', name: 'Reportera', color: '#8f2f5b' },
  { id: 'gamer', emoji: '🎮', name: 'Gamer', color: '#2f8f6b' },
  { id: 'robot', emoji: '🤖', name: 'Robot', color: '#365c73' },
  { id: 'mascot', emoji: '🐼', name: 'Mascota', color: '#555f66' },
];

export const TRANSITION_FADES = {
  fade: { fadeIn: 0.6, fadeOut: 0.6 },
  dissolve: { fadeIn: 0.8, fadeOut: 0.4 },
  black: { fadeIn: 0.9, fadeOut: 0.9 },
  white: { fadeIn: 0.7, fadeOut: 0.7 },
  flash: { fadeIn: 0.25, fadeOut: 0.25 },
  none: { fadeIn: 0, fadeOut: 0 },
};
