//! Biblioteca de efectos parametrizables que replica el catálogo real de
//! AbsentMotion (<https://absentmotion.com/effects/>): 240 efectos en 12
//! categorías (Color, Desenfoque, Distorsión, Estilizar, Ruido, Repetir, Croma,
//! Movimiento, Ritmo, 3D, Texto y Otros).
//!
//! A diferencia de `paint::effect_css` (cadenas CSS fijas sin ajustes), aquí
//! cada efecto expone parámetros reales que el panel de propiedades muestra como
//! deslizadores. Cada parámetro se traduce a un token del mismo motor de render
//! por píxel que usan la vista previa (`paint::clip_chain_from`) y la exportación
//! (`services::compose`), de modo que lo que se ve es exactamente lo que se
//! exporta.
//!
//! El motor actual solo soporta color + desenfoque. Los efectos puramente de
//! color/desenfoque se replican de forma fiel (`approx = false`). Los efectos
//! geométricos, de deformación, de movimiento y 3D se aproximan con gradaciones
//! de color/desenfoque equivalentes y se marcan con `approx = true`: sus
//! parámetros alteran de verdad el fotograma (no hay controles muertos), pero la
//! deformación geométrica real llegará cuando se amplíe el motor de render
//! (fase B). El panel de propiedades avisa de ello.

use serde_json::Value;

/// Token CSS al que mapea un parámetro.
#[derive(Clone, Copy)]
enum Func {
    Brightness,
    Contrast,
    Saturate,
    Hue,
    Grayscale,
    Sepia,
    Invert,
    Blur,
    /// Parámetro geométrico: no produce token CSS; lo consume
    /// `paint::warp_rgba` (deformación real de coordenadas).
    Warp,
}

/// Definición de un parámetro configurable de un efecto.
pub struct FxParam {
    pub key: &'static str,
    pub label: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub unit: &'static str,
    func: Func,
}

/// Definición de un efecto de la biblioteca.
pub struct FxDef {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub params: &'static [FxParam],
    /// `true` cuando el efecto es una aproximación de color/desenfoque de un
    /// efecto geométrico/3D/movimiento (la deformación real es fase B).
    pub approx: bool,
}

const fn p(
    key: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    default: f64,
    unit: &'static str,
    func: Func,
) -> FxParam {
    FxParam { key, label, min, max, default, unit, func }
}

const fn fx(
    id: &'static str,
    label: &'static str,
    category: &'static str,
    params: &'static [FxParam],
    approx: bool,
) -> FxDef {
    FxDef { id, label, category, params, approx }
}

// ---- Presets de parámetros reutilizados por efectos afines ----
// Cada preset es un conjunto de deslizadores que mapean a tokens reales del
// motor. Compartirlos mantiene el catálogo compacto sin crear controles falsos.

const P_GRADE: &[FxParam] = &[
    p("brightness", "Brillo", 50.0, 200.0, 100.0, "%", Func::Brightness),
    p("contrast", "Contraste", 50.0, 200.0, 108.0, "%", Func::Contrast),
    p("saturate", "Saturación", 0.0, 300.0, 112.0, "%", Func::Saturate),
];
const P_HUESAT: &[FxParam] = &[
    p("hue", "Tono", -180.0, 180.0, 12.0, "°", Func::Hue),
    p("saturate", "Saturación", 0.0, 300.0, 125.0, "%", Func::Saturate),
];
const P_BLUR: &[FxParam] = &[
    p("blur", "Radio", 0.0, 40.0, 6.0, "px", Func::Blur),
];
const P_GLOW: &[FxParam] = &[
    p("brightness", "Brillo", 50.0, 200.0, 118.0, "%", Func::Brightness),
    p("blur", "Resplandor", 0.0, 40.0, 4.0, "px", Func::Blur),
];
const P_MONO: &[FxParam] = &[
    p("grayscale", "Intensidad", 0.0, 100.0, 100.0, "%", Func::Grayscale),
    p("contrast", "Contraste", 50.0, 200.0, 115.0, "%", Func::Contrast),
];
const P_SEPIA: &[FxParam] = &[
    p("sepia", "Intensidad", 0.0, 100.0, 80.0, "%", Func::Sepia),
    p("contrast", "Contraste", 50.0, 200.0, 105.0, "%", Func::Contrast),
];
const P_INVERT: &[FxParam] = &[
    p("invert", "Intensidad", 0.0, 100.0, 100.0, "%", Func::Invert),
];
const P_SAT: &[FxParam] = &[
    p("saturate", "Saturación", 0.0, 300.0, 150.0, "%", Func::Saturate),
    p("contrast", "Contraste", 50.0, 200.0, 112.0, "%", Func::Contrast),
];
const P_BRIGHT: &[FxParam] = &[
    p("brightness", "Brillo", 50.0, 200.0, 115.0, "%", Func::Brightness),
    p("contrast", "Contraste", 50.0, 200.0, 110.0, "%", Func::Contrast),
];
const P_DARK: &[FxParam] = &[
    p("brightness", "Brillo", 50.0, 200.0, 85.0, "%", Func::Brightness),
    p("contrast", "Contraste", 50.0, 200.0, 125.0, "%", Func::Contrast),
];
const P_TEMP: &[FxParam] = &[
    p("hue", "Temperatura", -180.0, 180.0, 15.0, "°", Func::Hue),
    p("sepia", "Tinte", 0.0, 100.0, 35.0, "%", Func::Sepia),
];
const P_DUO: &[FxParam] = &[
    p("grayscale", "Base", 0.0, 100.0, 70.0, "%", Func::Grayscale),
    p("sepia", "Teñido", 0.0, 100.0, 60.0, "%", Func::Sepia),
    p("hue", "Tono", -180.0, 180.0, 40.0, "°", Func::Hue),
];
// Efectos geométricos reales (deformación de coordenadas en `paint::warp_rgba`).
const P_WARP: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 50.0, "%", Func::Warp),
];
const P_WARP_DETAIL: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 50.0, "%", Func::Warp),
    p("detail", "Detalle", 1.0, 20.0, 8.0, "", Func::Warp),
];
const P_WARP_ANGLE: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 50.0, "%", Func::Warp),
    p("detail", "Ángulo", 0.0, 360.0, 0.0, "°", Func::Warp),
];
// Ritmo: cantidad (+ detalle/ángulo) + tempo en BPM.
const P_BEAT: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 60.0, "%", Func::Warp),
    p("bpm", "Tempo", 40.0, 220.0, 120.0, " BPM", Func::Warp),
];
const P_BEAT_DETAIL: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 60.0, "%", Func::Warp),
    p("detail", "Detalle", 1.0, 20.0, 8.0, "", Func::Warp),
    p("bpm", "Tempo", 40.0, 220.0, 120.0, " BPM", Func::Warp),
];
const P_BEAT_ANGLE: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 60.0, "%", Func::Warp),
    p("detail", "Ángulo", 0.0, 360.0, 0.0, "°", Func::Warp),
    p("bpm", "Tempo", 40.0, 220.0, 120.0, " BPM", Func::Warp),
];
const P_FPS: &[FxParam] = &[
    p("fps", "Fotogramas/s", 1.0, 60.0, 8.0, "", Func::Warp),
];
const P_HUE: &[FxParam] = &[
    p("hue", "Desplazamiento", -180.0, 180.0, 60.0, "°", Func::Hue),
];
const P_BOXBLUR: &[FxParam] = &[
    p("blur", "Radio", 0.0, 40.0, 8.0, "px", Func::Blur),
    p("contrast", "Contraste", 50.0, 200.0, 100.0, "%", Func::Contrast),
];
// ---- Texto (los consume `fx_text`) ----
const P_TXT_REVEAL: &[FxParam] = &[
    p("amount", "Duración", 1.0, 100.0, 40.0, "% del clip", Func::Warp),
];
const P_TXT_MOTION: &[FxParam] = &[
    p("amount", "Cantidad", 0.0, 100.0, 50.0, "%", Func::Warp),
    p("detail", "Velocidad", 1.0, 20.0, 8.0, "", Func::Warp),
];
const P_TXT_WEIGHT: &[FxParam] = &[
    p("weight", "Grosor", 100.0, 900.0, 800.0, "", Func::Warp),
];
const P_TXT_COUNT: &[FxParam] = &[
    p("from", "Desde", -10000.0, 10000.0, 0.0, "", Func::Warp),
    p("to", "Hasta", -10000.0, 10000.0, 100.0, "", Func::Warp),
    p("decimals", "Decimales", 0.0, 3.0, 0.0, "", Func::Warp),
];
const P_TXT_TC: &[FxParam] = &[
    p("fps", "Fotogramas/s", 1.0, 60.0, 30.0, "", Func::Warp),
];
const P_TXT_SPACING: &[FxParam] = &[
    p("spacing", "Espaciado", -10.0, 60.0, 12.0, "px", Func::Warp),
];

/// Catálogo completo de efectos. Las categorías (y su orden en el panel) las
/// define `EFFECT_SUBS` en `inspector_clip`.
pub const LIBRARY: &[FxDef] = &[
    // ===================== Color (31) =====================
    fx("col_huesat", "Tono y saturación", "Color", P_HUESAT, false),
    fx("col_hsl", "HSL", "Color", P_HUESAT, false),
    fx("col_bricon", "Brillo y contraste", "Color", P_BRIGHT, false),
    fx("col_adjust", "Ajuste de color", "Color", P_GRADE, false),
    fx("col_solid", "Color sólido", "Color", P_WARP_DETAIL, false),
    fx("col_gradover", "Superposición de degradado", "Color", P_WARP_DETAIL, false),
    fx("col_grad4", "Degradado de cuatro colores", "Color", P_WARP, false),
    fx("col_balance", "Balance de color", "Color", P_HUESAT, false),
    fx("col_lgg", "Lift Gamma Gain", "Color", P_GRADE, false),
    fx("col_splittone", "Tono dividido", "Color", P_HUESAT, false),
    fx("col_temp", "Temperatura de color", "Color", P_TEMP, false),
    fx("col_duotone", "Duotono", "Color", P_DUO, false),
    fx("col_gradmap", "Mapa de degradado", "Color", P_WARP, false),
    fx("col_exposure", "Exposición", "Color", P_BRIGHT, false),
    fx("col_invert", "Invertir", "Color", P_INVERT, false),
    fx("col_levels", "Niveles", "Color", P_GRADE, false),
    fx("col_posterize", "Posterizar", "Color", P_SAT, false),
    fx("col_sepia", "Sepia", "Color", P_SEPIA, false),
    fx("col_threshold", "Umbral", "Color", P_MONO, false),
    fx("col_tint", "Tinte", "Color", P_TEMP, false),
    fx("col_colorize", "Colorear", "Color", P_HUESAT, false),
    fx("col_palette", "Mapa de paleta", "Color", P_WARP, false),
    fx("col_dither", "Tramado retro", "Color", P_WARP_DETAIL, false),
    fx("col_spot", "Color selectivo", "Color", P_WARP_DETAIL, false),
    fx("col_replace", "Reemplazar color", "Color", P_WARP_DETAIL, false),
    fx("col_remaprgb", "Remapeo de canales (RGB)", "Color", P_WARP_DETAIL, false),
    fx("col_remaphsv", "Remapeo de canales (HSV)", "Color", P_WARP_DETAIL, false),
    fx("col_mixer", "Mezclador de canales", "Color", P_WARP, false),
    fx("col_difference", "Diferencia", "Color", P_WARP_DETAIL, false),
    fx("col_hishad", "Luces y sombras", "Color", P_GRADE, false),
    fx("col_darkglow", "Resplandor oscuro", "Color", P_WARP, false),
    fx("col_hueshift", "Cambio de tono", "Color", P_HUE, false),
    fx("col_vibrance", "Saturación/Intensidad", "Color", P_SAT, false),
    fx("col_colortune", "Ajuste fino de color", "Color", P_GRADE, false),
    fx("col_hotcolor", "Color caliente", "Color", P_WARP, false),
    fx("col_spectral", "Mapa espectral", "Color", P_WARP_DETAIL, false),
    // ===================== Desenfoque (17) =====================
    fx("blr_blur", "Desenfoque", "Desenfoque", P_BLUR, false),
    fx("blr_inner", "Desenfoque interior", "Desenfoque", P_WARP, false),
    fx("blr_directional", "Desenfoque direccional", "Desenfoque", P_WARP_ANGLE, false),
    fx("blr_motion", "Desenfoque de movimiento", "Desenfoque", P_WARP_ANGLE, false),
    fx("blr_smear", "Manchar", "Desenfoque", P_WARP_ANGLE, false),
    fx("blr_warp", "Desenfoque de deformación", "Desenfoque", P_WARP, false),
    fx("blr_zoom", "Desenfoque de zoom", "Desenfoque", P_WARP, false),
    fx("blr_chromzoom", "Desenfoque de zoom cromático", "Desenfoque", P_WARP, false),
    fx("blr_zoomstreak", "Estelas de zoom", "Desenfoque", P_WARP, false),
    fx("blr_spinstreak", "Estelas de giro", "Desenfoque", P_WARP, false),
    fx("blr_linstreak", "Estelas lineales", "Desenfoque", P_WARP_ANGLE, false),
    fx("blr_spin", "Desenfoque de giro", "Desenfoque", P_WARP, false),
    fx("blr_vortex", "Desenfoque de vórtice", "Desenfoque", P_WARP, false),
    fx("blr_lens", "Desenfoque de lente", "Desenfoque", P_BLUR, false),
    fx("blr_mask", "Desenfoque de máscara", "Desenfoque", P_WARP, false),
    fx("blr_fillbehind", "Rellenar detrás", "Desenfoque", P_WARP, false),
    fx("blr_denoise", "Reducir ruido", "Desenfoque", P_BLUR, false),
    fx("blr_box", "Desenfoque de caja", "Desenfoque", P_BOXBLUR, false),
    fx("blr_gaussian", "Desenfoque gaussiano", "Desenfoque", P_BLUR, false),
    fx("blr_streakstrips", "Tiras de estela", "Desenfoque", P_WARP_ANGLE, false),
    // ===================== Distorsión (31) =====================
    fx("dst_swirl", "Remolino", "Distorsión", P_WARP, false),
    fx("dst_pinch", "Pellizcar/Abultar", "Distorsión", P_WARP, false),
    fx("dst_puppet", "Alfiler de marioneta", "Distorsión", P_WARP, false),
    fx("dst_innerpinch", "Pellizco interior", "Distorsión", P_WARP, false),
    fx("dst_glitch", "Fallo", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_noisedisp", "Desplazamiento por ruido", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_ripple", "Onda", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_circripple", "Onda circular", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_wavewarp", "Deformación de onda", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_turbdisp", "Desplazamiento turbulento", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_fractal", "Deformación fractal", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_tiles", "Mosaicos", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_hextile", "Mosaico hexagonal", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_tilerot", "Rotar mosaico", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_tileshift", "Desplazar mosaico", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_hextilerot", "Rotar mosaico hexagonal", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_hextileshift", "Desplazar mosaico hexagonal", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_squeeze", "Estrujar", "Distorsión", P_WARP, false),
    fx("dst_polar", "Coordenadas polares", "Distorsión", P_WARP, false),
    fx("dst_360view", "Visor 360°", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_360reorient", "Reorientar esfera 360°", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_stretchseg", "Estirar segmento", "Distorsión", P_WARP, false),
    fx("dst_bend", "Doblar", "Distorsión", P_WARP, false),
    fx("dst_curl", "Rizar", "Distorsión", P_WARP, false),
    fx("dst_pagecurl", "Pasar página", "Distorsión", P_WARP, false),
    fx("dst_magnify", "Ampliar fondo", "Distorsión", P_WARP, false),
    fx("dst_glass", "Cristal", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_dispmap", "Mapa de desplazamiento", "Distorsión", P_WARP, false),
    fx("dst_polardispmap", "Mapa de desplazamiento polar", "Distorsión", P_WARP, false),
    fx("dst_ominoglass", "Cristal Omino", "Distorsión", P_WARP_DETAIL, false),
    fx("dst_liquidglass", "Cristal líquido", "Distorsión", P_WARP_DETAIL, false),
    // ===================== Estilizar (59) =====================
    fx("sty_cartoon", "Dibujos animados", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_ominodiff", "Difusión Omino+", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_hexarray", "Matriz hexagonal", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_glow", "Resplandor", "Estilizar", P_GLOW, false),
    fx("sty_outline", "Contorno", "Estilizar", P_WARP, false),
    fx("sty_lightglow", "Resplandor de luz", "Estilizar", P_GLOW, false),
    fx("sty_softglow", "Resplandor suave", "Estilizar", P_GLOW, false),
    fx("sty_deepglow", "Resplandor profundo", "Estilizar", P_GLOW, false),
    fx("sty_innerglow", "Resplandor interior", "Estilizar", P_WARP, false),
    fx("sty_edgeglow", "Resplandor de bordes", "Estilizar", P_WARP, false),
    fx("sty_contourlines", "Líneas de contorno", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_contourgrad", "Degradado de contorno", "Estilizar", P_WARP, false),
    fx("sty_electricedges", "Bordes eléctricos", "Estilizar", P_WARP, false),
    fx("sty_contourstrips", "Tiras de contorno", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_glowscan", "Barrido de resplandor", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_ribbon", "Cinta", "Estilizar", P_WARP, false),
    fx("sty_edgedetect", "Detección de bordes", "Estilizar", P_WARP, false),
    fx("sty_emboss", "Repujado", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_smoothbevel", "Bisel suave", "Estilizar", P_WARP, false),
    fx("sty_bumpmap", "Mapa de relieve", "Estilizar", P_WARP, false),
    fx("sty_roughen", "Bordes ásperos", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_halftone", "Semitono", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_halftonelines", "Líneas de semitono", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_cmykdots", "Puntos CMYK", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_ascii", "ASCII", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_lightflare", "Destello de luz", "Estilizar", P_WARP, false),
    fx("sty_longshadow", "Sombra larga", "Estilizar", P_WARP, false),
    fx("sty_dropshadow", "Sombra paralela", "Estilizar", P_WARP, false),
    fx("sty_radialshadow", "Sombra radial", "Estilizar", P_WARP, false),
    fx("sty_pixelate", "Pixelar", "Estilizar", P_WARP, false),
    fx("sty_mosaic", "Mosaico", "Estilizar", P_WARP, false),
    fx("sty_pixeltiles", "Mosaicos de píxeles", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_rgbsplit", "División RGB", "Estilizar", P_WARP_ANGLE, false),
    fx("sty_scanlines", "Líneas de escaneo", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_stripes", "Rayas", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_voronoi", "Celdas de Voronoi", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_star", "Estrella", "Estilizar", P_WARP, false),
    fx("sty_checker", "Damero", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_grid", "Cuadrícula", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_dots", "Puntos", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_radialrays", "Rayos radiales", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_lightning", "Rayo", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_energybeam", "Haz de energía", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_iridescence", "Iridiscencia", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_rays", "Rayos", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_starfield", "Campo de estrellas", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_simplestarfield", "Campo de estrellas simple", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_particles", "Partículas", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_sharpen", "Enfocar", "Estilizar", P_WARP, false),
    fx("sty_vignette", "Viñeta", "Estilizar", P_WARP, false),
    fx("sty_grunge", "Grunge", "Estilizar", P_WARP, false),
    fx("sty_blink", "Parpadeo", "Estilizar", P_WARP, false),
    fx("sty_fade", "Fundido entrada/salida", "Estilizar", P_WARP, false),
    fx("sty_strokecolor", "Color de trazo", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_unsharp", "Máscara de enfoque", "Estilizar", P_WARP, false),
    fx("sty_flicker", "Titileo", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_vhs", "VHS", "Estilizar", P_SAT, false),
    fx("sty_datamosh", "Datamosh", "Estilizar", P_WARP_DETAIL, false),
    fx("sty_drawprogress", "Progreso de dibujo", "Estilizar", P_WARP, false),
    fx("sty_stroketaper", "Afilado de trazo", "Estilizar", P_WARP, false),
    // ===================== Ruido (9) =====================
    fx("noi_filmgrain", "Grano de película", "Ruido", P_WARP_DETAIL, false),
    fx("noi_turbulence", "Turbulencia", "Ruido", P_WARP_DETAIL, false),
    fx("noi_clouds", "Nubes", "Ruido", P_WARP_DETAIL, false),
    fx("noi_fractalridges", "Crestas fractales", "Ruido", P_WARP_DETAIL, false),
    fx("noi_dissolve", "Disolver", "Ruido", P_WARP_DETAIL, false),
    fx("noi_blockdissolve", "Disolución por bloques", "Ruido", P_WARP_DETAIL, false),
    fx("noi_blocknoise", "Ruido de bloques", "Ruido", P_WARP_DETAIL, false),
    fx("noi_noise", "Ruido", "Ruido", P_WARP_DETAIL, false),
    fx("noi_static", "Estática", "Ruido", P_WARP_DETAIL, false),
    // ===================== Repetir (11) =====================
    fx("rep_kaleidoscope", "Caleidoscopio", "Repetir", P_WARP_DETAIL, false),
    fx("rep_mirror", "Espejo", "Repetir", P_WARP, false),
    fx("rep_tile", "Mosaico", "Repetir", P_WARP_DETAIL, false),
    fx("rep_repeat", "Repetir", "Repetir", P_WARP_DETAIL, false),
    fx("rep_feedback", "Retroalimentación de vídeo", "Repetir", P_WARP_DETAIL, false),
    fx("rep_linear", "Repetición lineal", "Repetir", P_WARP_DETAIL, false),
    fx("rep_radial", "Repetición radial", "Repetir", P_WARP_DETAIL, false),
    fx("rep_scatter", "Repetición dispersa", "Repetir", P_WARP_DETAIL, false),
    fx("rep_grid", "Repetición en cuadrícula", "Repetir", P_WARP_DETAIL, false),
    fx("rep_alongpath", "Repetir a lo largo de ruta", "Repetir", P_WARP_DETAIL, false),
    fx("rep_scatterfield", "Campo disperso", "Repetir", P_WARP_DETAIL, false),
    // ===================== Croma (13) =====================
    fx("key_chroma", "Clave cromática", "Croma", P_WARP_DETAIL, false),
    fx("key_spill", "Limpiador de derrame", "Croma", P_WARP_DETAIL, false),
    fx("key_luma", "Clave de luminancia", "Croma", P_WARP, false),
    fx("key_diffmatte", "Mate por diferencia", "Croma", P_WARP, false),
    fx("key_lumastamp", "Estampado de luma", "Croma", P_WARP, false),
    fx("key_solidmatte", "Mate sólido", "Croma", P_WARP_DETAIL, false),
    fx("key_crop", "Recortar", "Croma", P_WARP, false),
    fx("key_wipe", "Barrido", "Croma", P_WARP_ANGLE, false),
    fx("key_feather", "Difuminar borde", "Croma", P_WARP, false),
    fx("key_radialwipe", "Barrido radial", "Croma", P_WARP, false),
    fx("key_choker", "Estrangulador de mate", "Croma", P_WARP, false),
    fx("key_fringe", "Fleco de mate", "Croma", P_WARP_DETAIL, false),
    fx("key_smoothedges", "Bordes suaves", "Croma", P_WARP, false),
    fx("key_advchroma", "Clave cromática avanzada", "Croma", P_WARP_DETAIL, false),
    // ===================== Movimiento (17) =====================
    fx("mov_autoshake", "Sacudida automática", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_randdisp", "Desplazamiento aleatorio", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_randjitter", "Vibración aleatoria", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_oscillate", "Oscilar", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_swing", "Balanceo", "Movimiento", P_WARP, false),
    fx("mov_spin", "Girar", "Movimiento", P_WARP, false),
    fx("mov_pulsesize", "Pulso de tamaño", "Movimiento", P_WARP, false),
    fx("mov_pulseopacity", "Pulso de opacidad", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_stretchaxis", "Estirar eje", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_offset", "Desplazamiento", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_rastertransform", "Transformación ráster", "Movimiento", P_WARP, false),
    fx("mov_alongpath", "Mover a lo largo de ruta", "Movimiento", P_WARP, false),
    fx("mov_scaleassist", "Asistente de escala", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_parenting", "Asistente de vinculación", "Movimiento", P_GRADE, true),
    fx("mov_echokeys", "Eco de fotogramas", "Movimiento", P_WARP_DETAIL, false),
    fx("mov_framerate", "Tasa de fotogramas", "Movimiento", P_FPS, false),
    fx("mov_truemotionblur", "Desenfoque de movimiento real", "Movimiento", P_WARP_ANGLE, false),
    // ===================== Ritmo (9) =====================
    fx("bet_zoom", "Zoom al ritmo", "Ritmo", P_BEAT, false),
    fx("bet_shake", "Sacudida al ritmo", "Ritmo", P_BEAT_DETAIL, false),
    fx("bet_flash", "Destello al ritmo", "Ritmo", P_BEAT, false),
    fx("bet_blur", "Desenfoque al ritmo", "Ritmo", P_BEAT, false),
    fx("bet_glow", "Resplandor al ritmo", "Ritmo", P_BEAT, false),
    fx("bet_rgbsplit", "División RGB al ritmo", "Ritmo", P_BEAT_ANGLE, false),
    fx("bet_pulse", "Pulso al ritmo", "Ritmo", P_BEAT, false),
    fx("bet_rotate", "Rotación al ritmo", "Ritmo", P_BEAT, false),
    fx("bet_glitch", "Fallo al ritmo", "Ritmo", P_BEAT_DETAIL, false),
    // ===================== 3D (19) =====================
    fx("d3_scatter", "Dispersión 3D", "3D", P_WARP_DETAIL, false),
    fx("d3_fracture", "Fractura 3D", "3D", P_WARP_DETAIL, false),
    fx("d3_cube", "Cubo", "3D", P_WARP_DETAIL, false),
    fx("d3_box", "Caja", "3D", P_WARP_DETAIL, false),
    fx("d3_hollowbox", "Caja hueca", "3D", P_WARP_DETAIL, false),
    fx("d3_cylinder", "Cilindro", "3D", P_WARP_DETAIL, false),
    fx("d3_ellipsoid", "Elipsoide", "3D", P_WARP_DETAIL, false),
    fx("d3_torus", "Toro", "3D", P_WARP_DETAIL, false),
    fx("d3_pyramid", "Pirámide", "3D", P_WARP_DETAIL, false),
    fx("d3_octahedron", "Octaedro", "3D", P_WARP_DETAIL, false),
    fx("d3_hexprism", "Prisma hexagonal", "3D", P_WARP_DETAIL, false),
    fx("d3_starprism", "Prisma estrella", "3D", P_WARP_DETAIL, false),
    fx("d3_starpoly", "Poliedro estrella", "3D", P_WARP_DETAIL, false),
    fx("d3_heart", "Corazón", "3D", P_WARP_DETAIL, false),
    fx("d3_cross", "Cruz de tres ejes", "3D", P_WARP_DETAIL, false),
    fx("d3_ring", "Anillo", "3D", P_WARP_DETAIL, false),
    fx("d3_rasterextrude", "Extrusión ráster", "3D", P_WARP, false),
    fx("d3_fliplayer", "Voltear capa", "3D", P_WARP, false),
    fx("d3_spherize", "Esferizar", "3D", P_WARP, false),
    fx("d3_tunnel", "Túnel", "3D", P_WARP_DETAIL, false),
    // ===================== Texto (9) =====================
    fx("txt_progress", "Progreso de texto", "Texto", P_TXT_REVEAL, false),
    fx("txt_transform", "Transformar texto", "Texto", P_TXT_MOTION, false),
    fx("txt_randomizer", "Aleatorizador de texto", "Texto", P_TXT_REVEAL, false),
    fx("txt_shake", "Sacudida de texto", "Texto", P_TXT_MOTION, false),
    fx("txt_fontweight", "Grosor de fuente", "Texto", P_TXT_WEIGHT, false),
    fx("txt_blips", "Destellos de texto", "Texto", P_TXT_MOTION, false),
    fx("txt_countupdown", "Cuenta arriba/abajo", "Texto", P_TXT_COUNT, false),
    fx("txt_timecode", "Código de tiempo", "Texto", P_TXT_TC, false),
    fx("txt_alongpath", "Texto a lo largo de ruta", "Texto", P_TXT_MOTION, false),
    fx("txt_spacing", "Espaciado de texto", "Texto", P_TXT_SPACING, false),
    // ===================== Otros (1) =====================
    fx("oth_copybg", "Copiar fondo", "Otros", P_GRADE, true),
];

/// Busca un efecto por su id.
pub fn by_id(id: &str) -> Option<&'static FxDef> {
    LIBRARY.iter().find(|d| d.id == id)
}

/// Itera los efectos de una categoría.
pub fn in_category(cat: &str) -> impl Iterator<Item = &'static FxDef> + '_ {
    LIBRARY.iter().filter(move |d| d.category == cat)
}

fn token(func: Func, v: f64) -> Option<String> {
    let r = |x: f64| (x * 1000.0).round() / 1000.0;
    match func {
        Func::Brightness => ((v - 100.0).abs() > 0.5).then(|| format!("brightness({})", r(v / 100.0))),
        Func::Contrast => ((v - 100.0).abs() > 0.5).then(|| format!("contrast({})", r(v / 100.0))),
        Func::Saturate => ((v - 100.0).abs() > 0.5).then(|| format!("saturate({})", r(v / 100.0))),
        Func::Hue => (v.abs() > 0.5).then(|| format!("hue-rotate({}deg)", r(v))),
        Func::Grayscale => (v > 0.5).then(|| format!("grayscale({})", r((v / 100.0).clamp(0.0, 1.0)))),
        Func::Sepia => (v > 0.5).then(|| format!("sepia({})", r((v / 100.0).clamp(0.0, 1.0)))),
        Func::Invert => (v > 0.5).then(|| format!("invert({})", r((v / 100.0).clamp(0.0, 1.0)))),
        Func::Blur => (v > 0.05).then(|| format!("blur({}px)", r(v))),
        Func::Warp => None,
    }
}

/// Produce la cadena CSS de filtros para un efecto de la biblioteca, usando los
/// valores guardados en `clip.extra["effectParams"]` (o los predeterminados).
/// Devuelve `None` si el id no pertenece a la biblioteca, para que quien llame
/// pueda recurrir al catálogo fijo de `paint::effect_css`.
pub fn css_for(id: &str, stored: Option<&Value>) -> Option<String> {
    let def = by_id(id)?;
    let mut out = String::new();
    for param in def.params {
        let v = stored
            .and_then(|s| s.get(param.key))
            .and_then(Value::as_f64)
            .unwrap_or(param.default);
        if let Some(tok) = token(param.func, v) {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&tok);
        }
    }
    Some(out)
}
