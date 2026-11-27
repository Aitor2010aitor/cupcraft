//! Detalles de un clip de vídeo, imagen, audio o sticker (`DETAIL_TABS`,
//! `detailTabBody` y `clipMarkup` de `src/editor/panels.js`):
//!
//! * Video: Básico · Eliminar fondo · Máscara · Retoque
//! * Audio: Básico · Modificador de voz
//! * Velocidad: Estándar · Curva · Efectos de velocidad
//! * Animación: Entrada · Salida · Combinado
//! * Ajustar: Básico · HSL · Curvas · Círculo cromático · Máscara
//!
//! Pie común: transición al inicio, duración, «Inicio · Duración» y
//! «Restablecer ajustes». Los objetos anidados (`mask`, `hsl`, `curves`,
//! `wheels`, `beauty`, `chromaKey`, `aiStyle`…) se envían completos, igual que
//! `setNestedPatch` del original.

use crate::anim_catalog::{ANIM_COMBO, ANIM_IN, ANIM_OUT};
use crate::app::CupApp;
use crate::format::format_time;
use crate::inspector_text::{apply, group, range, range_ex, switch_row, tab_row, to_hex, Edit, KfState};
use crate::panels::{card, grid, hint, secondary_button, section};
use crate::paint;
use crate::theme;
use crate::voice_filters::VOICE_FILTERS;
use cupcraft_core::{Clip, ClipKind};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use serde_json::{json, Map, Value};

const TABS: &[(&str, &str, &[(&str, &str)])] = &[
    ("video", "Video", &[("basic", "Básico"), ("cutout", "Eliminar fondo"), ("mask", "Máscara"), ("retouch", "Retoque")]),
    ("audio", "Audio", &[("basic", "Básico"), ("voice", "Modificador de voz")]),
    ("speed", "Velocidad", &[("standard", "Estándar"), ("curve", "Curva"), ("fx", "Efectos de velocidad")]),
    ("animation", "Animación", &[("in", "Entrada"), ("out", "Salida"), ("combo", "Combinado")]),
    (
        "adjust",
        "Ajustar",
        &[("basic", "Básico"), ("hsl", "HSL"), ("curves", "Curvas"), ("wheel", "Círculo cromático"), ("mask", "Máscara")],
    ),
    ("effects", "Efectos", EFFECT_SUBS),
];

/// Subpestañas de la pestaña «Efectos»: una por categoría de `fx_library`
/// (clave == etiqueta, que coincide con `FxDef::category`).
const EFFECT_SUBS: &[(&str, &str)] = &[
    ("Color", "Color"),
    ("Desenfoque", "Desenfoque"),
    ("Distorsión", "Distorsión"),
    ("Estilizar", "Estilizar"),
    ("Ruido", "Ruido"),
    ("Repetir", "Repetir"),
    ("Croma", "Croma"),
    ("Movimiento", "Movimiento"),
    ("Ritmo", "Ritmo"),
    ("3D", "3D"),
    ("Texto", "Texto"),
    ("Otros", "Otros"),
];

#[derive(Clone, Default)]
struct TabState {
    tab: usize,
    subs: [usize; 7],
}

const MASK_SHAPES: &[(&str, &str)] = &[
    ("none", "Ninguna"),
    ("linear", "Lineal"),
    ("mirror", "Espejo"),
    ("circle", "Círculo"),
    ("rect", "Rectángulo"),
    ("rounded", "Redondeado"),
    ("heart", "Corazón"),
    ("star", "Estrella"),
    ("triangle", "Triángulo"),
];

const SPEED_CURVES: &[(&str, &str)] = &[
    ("none", "Ninguno"),
    ("custom", "Personalizado"),
    ("montage", "Montaje"),
    ("hero", "Héroe"),
    ("bullet", "Bala"),
    ("cut", "Corte en el ..."),
    ("flashIn", "Flash de entra..."),
    ("flashOut", "Flash de salida"),
];

const SPEED_FX: &[(&str, &str)] = &[
    ("none", "Ninguno"),
    ("flash", "Flash"),
    ("motion", "Movimiento"),
    ("fade", "Desvanecer"),
    ("zoom", "Zoom retro"),
    ("alcohol", "Alcohol"),
];

const HSL_COLORS: &[(&str, &str, &str)] = &[
    ("red", "Rojos", "#ff3b3b"),
    ("orange", "Naranjas", "#ff9f1a"),
    ("yellow", "Amarillos", "#ffe14a"),
    ("green", "Verdes", "#4ade80"),
    ("cyan", "Cianes", "#22d3ee"),
    ("blue", "Azules", "#3b82f6"),
    ("purple", "Morados", "#a855f7"),
    ("magenta", "Magentas", "#ec4899"),
];

const CURVE_CHANNELS: &[(&str, &str, Color32)] = &[
    ("luma", "Luma", Color32::from_rgb(0xe8, 0xe8, 0xe8)),
    ("red", "Rojo", Color32::from_rgb(0xff, 0x55, 0x55)),
    ("green", "Verde", Color32::from_rgb(0x4a, 0xde, 0x80)),
    ("blue", "Azul", Color32::from_rgb(0x3b, 0x82, 0xf6)),
];

const WHEELS: &[(&str, &str)] = &[("shadow", "Sombras"), ("midtone", "Gris intermedio"), ("tint", "Tinte"), ("offset", "Desplazamiento")];

const CUTOUT_STROKES: &[(&str, &str)] = &[
    ("none", "Ninguno"),
    ("paper", "Canto papel"),
    ("glow", "Luminiscencia"),
    ("hand", "Dibujo mano"),
    ("cutout", "Contorno cortado"),
    ("solid", "Contorno sólido"),
    ("wind", "Contorno viento"),
    ("dotted", "Contorneado"),
];

/// Interruptores locales de Video → Básico: (clave, etiqueta, nota).
const VIDEO_SWITCHES: &[(&str, &str, &str)] = &[
    ("stabilize", "Estabilización", "Reduce el temblor de la cámara con análisis de movimiento."),
    ("enhanceQuality", "Mejorar la calidad", "Escala y mejora la nitidez localmente."),
    ("denoise", "Reducir el ruido de la imagen", ""),
    ("opticalFlow", "Flujo óptico", ""),
    ("aiExpand", "Ampliación de IA", ""),
    ("aiRemix", "Remix de IA", ""),
    ("eyeContact", "Contacto visual", "Aproximación local: retoque suave de luz (sin IA en la nube)."),
    ("lipSync", "Sincronización labial", "Aproximación local: retoque suave (sin IA en la nube)."),
    ("lighting", "Adición de iluminación", "Luz suave radial y realce de sombras."),
    ("autoFrame", "Automarcado", "Reencuadre automático con zoom centrado."),
    ("flickerRemove", "Eliminar los parásitos", "Iguala el parpadeo de luminosidad entre fotogramas."),
    ("aiMotion", "Movimiento de IA", "Zoom suave progresivo a lo largo del clip."),
    ("cameraTracking", "Seguimiento de cámara", "Encuadre que sigue la zona con movimiento."),
    ("blurMotion", "Movimiento difuminado", "Desenfoque direccional según el movimiento."),
];

/// `CAPCUT_LOCAL_MAP` del original (nombre CapCut → efecto local).
const CAPCUT_LOCAL_MAP: &[(&[&str], &str)] = &[
    (&["desenfoque", "blur", "bokeh", "difumin", "suaviz"], "esp_dreamy"),
    (&["glitch", "falla", "error", "distorsi", "corrupt"], "tnd_horror"),
    (&["mosaico", "mosaic", "pixelad", "pixel"], "tnd_mosaic"),
    (&["sepia", "vintage", "retro", "vhs", "film"], "ret_vhs"),
    (&["zoom", "pulso", "latido"], "zoomPulse"),
    (&["temblor", "shake", "vibra", "tremor"], "tnd_retroShake"),
    (&["brillo", "glow", "flash", "neon", "luz"], "luz_flash"),
    (&["coraz", "heart", "amor"], "tnd_heart"),
    (&["confeti", "confetti", "glitter", "chispa", "particul"], "fst_confetti"),
    (&["garabato", "doodle", "dibuj"], "tnd_doodle"),
    (&["niebla", "fog", "humo", "smoke", "vapor"], "hal_fog"),
    (&["sangre", "blood"], "hal_blood"),
    (&["calabaza", "pumpkin", "halloween"], "hal_pumpkin"),
    (&["fantasma", "ghost"], "hal_ghost"),
    (&["cromatic", "chroma", "rgb", "aberraci"], "tnd_chroma"),
    (&["grano", "grain", "ruido", "noise"], "grain"),
    (&["viñeta", "vineta", "vignette"], "vignette"),
];

fn capcut_to_local(name: &str) -> &'static str {
    let l = name.to_lowercase();
    CAPCUT_LOCAL_MAP
        .iter()
        .find(|(keys, _)| keys.iter().any(|k| l.contains(k)))
        .map(|(_, id)| *id)
        .unwrap_or("none")
}

// ---------------- lectura del clip ----------------

fn ex<'a>(clip: &'a Clip, key: &str) -> &'a Value {
    clip.extra.get(key).unwrap_or(&Value::Null)
}
fn obj(clip: &Clip, key: &str) -> Map<String, Value> {
    ex(clip, key).as_object().cloned().unwrap_or_default()
}
fn flag(clip: &Clip, key: &str) -> bool {
    ex(clip, key).as_bool().unwrap_or(false)
}
fn exs<'a>(clip: &'a Clip, key: &str, d: &'a str) -> &'a str {
    ex(clip, key).as_str().unwrap_or(d)
}
fn exn(clip: &Clip, key: &str, d: f64) -> f64 {
    ex(clip, key).as_f64().unwrap_or(d)
}
fn mn(m: &Map<String, Value>, key: &str, d: f64) -> f64 {
    m.get(key).and_then(Value::as_f64).unwrap_or(d)
}
fn adj(clip: &Clip, key: &str, d: f64) -> f64 {
    match key {
        "brightness" => clip.adjust.brightness,
        "contrast" => clip.adjust.contrast,
        "saturate" => clip.adjust.saturate,
        _ => clip.adjust.extra.get(key).and_then(Value::as_f64).unwrap_or(d),
    }
}

fn cset(e: &mut Edit, key: &str, v: Value) {
    e.clip.insert(key.to_string(), v);
}

// ---------------- controles comunes ----------------

fn note(ui: &mut egui::Ui, text: &str) {
    if !text.is_empty() {
        ui.label(egui::RichText::new(crate::i18n::tr(text)).size(10.5).color(theme::MUTED2));
    }
}

fn badge(ui: &mut egui::Ui, text: &str) {
    if text.is_empty() {
        return;
    }
    egui::Frame::NONE
        .fill(theme::PANEL3)
        .corner_radius(CornerRadius::from(3))
        .inner_margin(egui::Margin::symmetric(4, 0))
        .show(ui, |ui| ui.label(egui::RichText::new(crate::i18n::tr(&text)).size(9.5).color(theme::ACCENT)));
}

/// `switchRow` sobre una clave de primer nivel del clip.
fn sw(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip, label: &str, key: &str, nt: &str) {
    let on = flag(clip, key);
    if switch_row(ui, label, on) {
        cset(e, key, json!(!on));
    }
    note(ui, nt);
}

/// `switchRow` sobre `objeto.active` (envía el objeto entero).
fn sw_nested(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip, label: &str, key: &str, field: &str) {
    let mut m = obj(clip, key);
    let on = m.get(field).and_then(Value::as_bool).unwrap_or(false);
    if switch_row(ui, label, on) {
        m.insert(field.into(), json!(!on));
        cset(e, key, Value::Object(m));
    }
}

/// `lockedRow`: función que requiere la nube.
fn locked(ui: &mut egui::Ui, e: &mut Edit, label: &str, bdg: &str, nt: &str) {
    let r = ui
        .horizontal(|ui| {
            ui.label(egui::RichText::new(crate::i18n::tr(&label)).size(12.0).color(theme::TEXT));
            badge(ui, bdg);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("›").size(13.0).color(theme::MUTED));
            });
        })
        .response
        .interact(Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    note(ui, nt);
    if r.clicked() {
        e.toast = Some(format!(
            "“{label}” requiere procesamiento en la nube y no está disponible sin conexión. El resto de ajustes del clip siguen funcionando en local."
        ));
    }
}

/// Deslizador sobre un campo de un objeto anidado (`detailRange('…', 'obj.key')`).
#[allow(clippy::too_many_arguments)]
fn nested_range(
    ui: &mut egui::Ui,
    e: &mut Edit,
    m: &mut Map<String, Value>,
    dirty: &mut bool,
    label: &str,
    key: &str,
    def: f64,
    min: f64,
    max: f64,
    factor: f64,
    unit: &str,
) {
    if let Some(v) = range(ui, e, label, mn(m, key, def), min, max, 1.0, factor, unit) {
        m.insert(key.into(), json!(v));
        *dirty = true;
    }
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr(&label)).size(12.0).color(theme::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr(&value)).size(12.0).color(theme::TEXT).strong());
        });
    });
}

fn color_pick(ui: &mut egui::Ui, label: &str, cur: &str) -> Option<String> {
    let mut out = None;
    ui.horizontal(|ui| {
        let mut c = paint::hex(cur);
        if ui.color_edit_button_srgba(&mut c).changed() {
            out = Some(to_hex(c));
        }
        ui.label(egui::RichText::new(crate::i18n::tr(&label)).size(11.5).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr(&cur)).size(11.0).color(theme::TEXT).monospace());
        });
    });
    out
}

/// Rejilla de baldosas con nombre (`fx-tile` / `curve-tile` / `mask-tile`).
fn tiles(ui: &mut egui::Ui, items: &[(&str, &str)], active: &str, glyph: impl Fn(&egui::Painter, Rect, &str)) -> Option<String> {
    let mut pick = None;
    grid(ui, items, 72.0, |ui, (id, label), w| {
        let (img, r) = card(ui, w, w * 0.62, label, *id == active);
        glyph(&ui.painter_at(img), img, id);
        if r.on_hover_text(*label).clicked() {
            pick = Some(id.to_string());
        }
    });
    pick
}

// ---------------- entrada ----------------

pub fn show(ui: &mut egui::Ui, app: &mut CupApp, clip: &Clip, track_name: &str) {
    let id = clip.id.clone();
    let kind_label = match clip.kind {
        ClipKind::Video => "Video",
        ClipKind::Image => "Imagen",
        ClipKind::Audio => "Audio",
        ClipKind::Sticker => "Sticker",
        _ => "Clip",
    };

    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 3.5, theme::ACCENT);
        let name = if clip.name.is_empty() { kind_label } else { clip.name.as_str() };
        ui.label(egui::RichText::new(crate::i18n::tr(&name)).color(theme::TEXT).strong())
            .on_hover_text(format!("Pista: {track_name}"));
        egui::Frame::NONE
            .fill(theme::PANEL3)
            .corner_radius(CornerRadius::from(4))
            .inner_margin(egui::Margin::symmetric(6, 1))
            .show(ui, |ui| ui.label(egui::RichText::new(crate::i18n::tr(&kind_label)).size(10.5).color(theme::MUTED)));
    });
    ui.add_space(4.0);

    let state_id = egui::Id::new("clip-detail-tabs");
    let mut st: TabState = ui.data_mut(|d| d.get_temp(state_id)).unwrap_or_default();
    st.tab = st.tab.min(TABS.len() - 1);
    let labels: Vec<&str> = TABS.iter().map(|t| t.1).collect();
    if let Some(i) = tab_row(ui, &labels, st.tab, 13.0) {
        st.tab = i;
    }
    let (tab, _, subs) = TABS[st.tab];
    let sl: Vec<&str> = subs.iter().map(|x| x.1).collect();
    if let Some(i) = tab_row(ui, &sl, st.subs[st.tab], 12.0) {
        st.subs[st.tab] = i;
    }
    let sub = subs[st.subs[st.tab].min(subs.len() - 1)].0;
    ui.separator();
    ui.add_space(4.0);

    let visual = matches!(clip.kind, ClipKind::Video | ClipKind::Image);
    let mut e = Edit::default();
    let mut jump_mask = false;
    let mut apply_all = false;
    match (tab, sub) {
        ("video", "mask") | ("adjust", "mask") => mask_tab(ui, &mut e, clip),
        ("video", "retouch") => retouch_tab(ui, &mut e, clip),
        ("video", "cutout") => jump_mask = cutout_tab(ui, &mut e, app, clip),
        ("video", _) => apply_all = video_basic(ui, &mut e, app, clip),
        ("audio", "voice") => voice_tab(ui, &mut e, clip),
        ("audio", _) => {
            if matches!(clip.kind, ClipKind::Video | ClipKind::Audio) {
                audio_basic(ui, &mut e, clip);
            } else {
                hint(ui, "Selecciona un clip con audio para ajustar su volumen.");
            }
        }
        ("speed", s) => speed_tab(ui, &mut e, clip, s),
        ("animation", s) => animation_tab(ui, &mut e, clip, s),
        ("adjust", "hsl") => hsl_tab(ui, &mut e, clip),
        ("adjust", "curves") => curves_tab(ui, &mut e, clip),
        ("adjust", "wheel") => wheel_tab(ui, &mut e, clip),
        ("adjust", _) => {
            if visual {
                adjust_basic(ui, &mut e, clip);
            } else {
                hint(ui, "Los ajustes de color se aplican a clips de video o imagen.");
            }
        }
        ("effects", cat) => {
            if visual || matches!(clip.kind, ClipKind::Text | ClipKind::Sticker) {
                effects_tab(ui, &mut e, clip, cat);
            } else {
                hint(ui, "Los efectos se aplican a clips de video, imagen, texto o sticker.");
            }
        }
        _ => {}
    }
    if jump_mask {
        st.subs[0] = 2;
    }
    ui.data_mut(|d| d.insert_temp(state_id, st));

    let reset = footer(ui, &mut e, clip);
    apply(app, &id, e);
    if apply_all {
        apply_to_all(app, clip);
    }
    if reset {
        reset_clip(app, &id);
    }
}

// ---------------- Video ----------------

/// Tamaño (% del lienzo) que ocupa la capa del clip, para alinearla en el lienzo.
fn layer_size_pct(app: &CupApp, clip: &Clip) -> (f64, f64) {
    let project = app.editor.project();
    let ratio = project.map(|p| p.ratio.clone()).unwrap_or_else(|| "16:9".into());
    let aspect = ratio
        .split_once(':')
        .and_then(|(a, b)| Some(a.trim().parse::<f64>().ok()? / b.trim().parse::<f64>().ok()?))
        .filter(|a| *a > 0.0)
        .unwrap_or(16.0 / 9.0);
    let media_aspect = project
        .and_then(|p| p.media.iter().find(|m| Some(&m.id) == clip.media_id.as_ref()))
        .and_then(crate::services::media::media_dims)
        .filter(|(w, h)| *w > 0 && *h > 0)
        .map(|(w, h)| w as f64 / h as f64)
        .unwrap_or(aspect);
    let s = clip.scale * 100.0;
    if media_aspect >= aspect {
        (s, s * aspect / media_aspect)
    } else {
        (s * media_aspect / aspect, s)
    }
}

/// Barra de alineación en el lienzo (izquierda, centro, derecha, arriba, medio, abajo).
fn align_bar(ui: &mut egui::Ui, e: &mut Edit, app: &CupApp, clip: &Clip) {
    let (w, h) = layer_size_pct(app, clip);
    let modes: [(&str, &str, &str); 6] = [
        ("alignLeft", "Alinear a la izquierda", "left"),
        ("alignCenterH", "Alinear al medio", "center"),
        ("alignRight", "Alinear a la derecha", "right"),
        ("alignTop", "Alinear arriba", "top"),
        ("alignCenterV", "Alinear al centro", "middle"),
        ("alignBottom", "Alinear abajo", "bottom"),
    ];
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (icon, tip, mode) in modes {
            if crate::icons::icon_button(ui, icon, 14.0, theme::MUTED, &crate::i18n::tr(tip)).clicked() {
                let r = |v: f64| (v * 100.0).round() / 100.0;
                match mode {
                    "left" => cset(e, "x", json!(r(-50.0 + w / 2.0))),
                    "center" => cset(e, "x", json!(0.0)),
                    "right" => cset(e, "x", json!(r(50.0 - w / 2.0))),
                    "top" => cset(e, "y", json!(r(-50.0 + h / 2.0))),
                    "middle" => cset(e, "y", json!(0.0)),
                    _ => cset(e, "y", json!(r(50.0 - h / 2.0))),
                }
            }
        }
    });
}

/// Copia del clip con los cambios de fotogramas clave que ya se hicieron en este
/// fotograma (para que dos controles no se pisen).
fn working_clip(e: &Edit, clip: &Clip) -> Clip {
    let mut w = clip.clone();
    if let Some(k) = e.clip.get("keyframes") {
        w.extra.insert("keyframes".into(), k.clone());
    }
    w
}

/// Guarda en el parche el mapa de fotogramas clave de `w` (vacío si no queda ninguno).
fn commit_keyframes(e: &mut Edit, w: &Clip) {
    let v = w.extra.get("keyframes").cloned().unwrap_or_else(|| json!({}));
    cset(e, "keyframes", v);
}

/// Deslizador animable: con rombo para crear/quitar un fotograma clave en el
/// instante del cursor. Si la propiedad está animada, editar el valor crea o
/// actualiza el fotograma clave de ese instante; si no, cambia el valor fijo.
#[allow(clippy::too_many_arguments)]
fn kf_range(ui: &mut egui::Ui, e: &mut Edit, app: &CupApp, clip: &Clip, label: &str, prop: &str, min: f64, max: f64, factor: f64, unit: &str) {
    use cupcraft_core::keyframes as kf;
    let t = app.playhead.clamp(clip.start, clip.start + clip.duration);
    let mut w = working_clip(e, clip);
    let cur = kf::value_at(&w, prop, t);
    let st = KfState { animated: kf::has_keyframes(&w, prop), on_key: kf::has_keyframe_at(&w, prop, t) };
    let (val, diamond) = range_ex(ui, e, label, cur, min, max, 1.0, factor, unit, Some(st));
    if diamond {
        kf::toggle_keyframe(&mut w, prop, t);
        commit_keyframes(e, &w);
        cset(e, prop, json!(kf::base_value(&w, prop)));
    } else if let Some(v) = val {
        kf::write_value(&mut w, prop, t, v);
        if kf::has_keyframes(&w, prop) {
            commit_keyframes(e, &w);
        } else {
            cset(e, prop, json!(v));
        }
    }
}

fn video_basic(ui: &mut egui::Ui, e: &mut Edit, app: &CupApp, clip: &Clip) -> bool {
    group(ui, "Transformación", true, |ui| {
        if secondary_button(ui, "↺ Restablecer transformación").clicked() {
            let mut w = working_clip(e, clip);
            for (k, v) in [("x", 0.0), ("y", 0.0), ("scale", 1.0), ("rotate", 0.0)] {
                cupcraft_core::keyframes::clear_prop(&mut w, k);
                cset(e, k, json!(v));
            }
            commit_keyframes(e, &w);
        }
        kf_range(ui, e, app, clip, "Escala", "scale", 10.0, 400.0, 0.01, "%");
        kf_range(ui, e, app, clip, "Posición X", "x", -100.0, 100.0, 1.0, "%");
        kf_range(ui, e, app, clip, "Posición Y", "y", -100.0, 100.0, 1.0, "%");
        kf_range(ui, e, app, clip, "Rotación", "rotate", -180.0, 180.0, 1.0, "°");
        align_bar(ui, e, app, clip);
    });
    group(ui, "Mezcla", true, |ui| {
        if secondary_button(ui, "↺ Restablecer mezcla").clicked() {
            let mut w = working_clip(e, clip);
            cupcraft_core::keyframes::clear_prop(&mut w, "opacity");
            commit_keyframes(e, &w);
            cset(e, "opacity", json!(1.0));
        }
        kf_range(ui, e, app, clip, "Opacidad", "opacity", 0.0, 100.0, 0.01, "%");
    });

    sw(ui, e, clip, "Desenfoque de zona", "aiRemove", "Difumina/borra una zona concreta del clip (funciona en local).");
    if flag(clip, "aiRemove") {
        section(ui, "Zona a eliminar");
        let mut area = obj(clip, "aiRemoveArea");
        let mut dirty = false;
        nested_range(ui, e, &mut area, &mut dirty, "Centro X", "x", 0.0, -50.0, 50.0, 1.0, "%");
        nested_range(ui, e, &mut area, &mut dirty, "Centro Y", "y", 0.0, -50.0, 50.0, 1.0, "%");
        nested_range(ui, e, &mut area, &mut dirty, "Ancho", "w", 30.0, 4.0, 100.0, 1.0, "%");
        nested_range(ui, e, &mut area, &mut dirty, "Alto", "h", 30.0, 4.0, 100.0, 1.0, "%");
        if dirty {
            cset(e, "aiRemoveArea", Value::Object(area));
        }
        if let Some(v) = range(ui, e, "Difuminado", exn(clip, "aiRemoveStrength", 28.0), 4.0, 60.0, 1.0, 1.0, "px") {
            cset(e, "aiRemoveStrength", json!(v));
        }
        hint(ui, "Coloca la zona sobre el objeto que quieras quitar: se rellena con una versión difuminada del propio fotograma, sin subir nada a la nube.");
    }
    for (key, label, nt) in VIDEO_SWITCHES {
        sw(ui, e, clip, label, key, nt);
    }

    let mut all = false;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr("Lienzo")).size(12.5).color(theme::TEXT).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            all = secondary_button(ui, "Aplicar a todo").clicked();
        });
    });
    let (ratio, res) = app
        .editor
        .project()
        .map(|p| (p.ratio.clone(), p.resolution.clone()))
        .unwrap_or_else(|| ("16:9".into(), "1080p".into()));
    info_row(ui, "Relación de aspecto", &ratio);
    info_row(ui, "Resolución", &res);
    all
}

/// `data-apply-all`: copia el grado de color y la máscara a los demás clips
/// visuales del proyecto.
fn apply_to_all(app: &mut CupApp, clip: &Clip) {
    let mut patch = Map::new();
    if let Ok(a) = serde_json::to_value(&clip.adjust) {
        patch.insert("adjust".into(), a);
    }
    for k in ["mask", "beauty", "aiStyle", "hsl", "curves", "effect", "glEffect"] {
        if let Some(v) = clip.extra.get(k).filter(|v| !v.is_null()) {
            patch.insert(k.into(), v.clone());
        }
    }
    patch.insert("filter".into(), json!(clip.filter));
    if let Some(g) = &clip.gl_filter {
        patch.insert("glFilter".into(), json!(g));
    }
    let ids: Vec<String> = app
        .editor
        .project()
        .map(|p| {
            p.tracks
                .iter()
                .flat_map(|t| t.clips.iter())
                .filter(|c| matches!(c.kind, ClipKind::Video | ClipKind::Image) && c.id != clip.id)
                .map(|c| c.id.clone())
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        app.toast("No hay otros clips visuales en el proyecto".to_string());
        return;
    }
    let patch = Value::Object(patch);
    app.mutate_project(|p| {
        for id in &ids {
            p.update_clip(id, patch.clone());
        }
    });
    app.save_current();
    app.toast("Ajustes aplicados a todos los clips visuales".to_string());
}

/// Eliminar fondo. Devuelve `true` si se pulsó «Recortar con una máscara».
fn cutout_tab(ui: &mut egui::Ui, e: &mut Edit, app: &CupApp, clip: &Clip) -> bool {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr("Eliminación automática")).size(12.0).color(theme::TEXT));
        badge(ui, "IA");
    });
    sw(ui, e, clip, "Activar", "autoCutout", "");
    for (label, key, def) in [
        ("Intensidad", "autoCutoutIntensity", 1.0),
        ("Estabilidad temporal", "autoCutoutTemporal", 0.3),
        ("Limpieza de bordes", "autoCutoutEdge", 0.35),
    ] {
        if let Some(v) = range(ui, e, label, exn(clip, key, def), 0.0, 100.0, 1.0, 0.01, "%") {
            cset(e, key, json!(v));
        }
    }
    hint(
        ui,
        "BackgroundMattingV2 (local, MIT): usa una foto del fondo o un color sólido para recortar. La estabilidad temporal y la limpieza de bordes son postproceso propio.",
    );
    if let Some(c) = color_pick(ui, "Color del fondo", exs(clip, "autoCutoutBgColor", "#00b140")) {
        cset(e, "autoCutoutBgColor", json!(c));
    }
    // Foto del fondo: una imagen ya importada en el proyecto.
    let images: Vec<(String, String)> = app
        .editor
        .project()
        .map(|p| {
            p.media
                .iter()
                .filter(|m| m.kind.as_deref() == Some("image"))
                .map(|m| (m.id.clone(), m.name.clone().unwrap_or_else(|| m.id.clone())))
                .collect()
        })
        .unwrap_or_default();
    let cur_bg = exs(clip, "autoCutoutBg", "");
    let cur_name = images.iter().find(|(i, _)| i == cur_bg).map(|(_, n)| n.as_str()).unwrap_or("Sin foto de fondo");
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr("Foto del fondo")).size(11.5).color(theme::MUTED));
        egui::ComboBox::from_id_salt("cutout-bg-plate").selected_text(cur_name).show_ui(ui, |ui| {
            if ui.selectable_label(cur_bg.is_empty(), crate::i18n::tr("Sin foto de fondo")).clicked() {
                cset(e, "autoCutoutBg", Value::Null);
            }
            for (mid, name) in &images {
                if ui.selectable_label(mid == cur_bg, name).clicked() {
                    cset(e, "autoCutoutBg", json!(mid));
                }
            }
        });
    });
    if images.is_empty() {
        note(ui, "Importa una imagen del fondo en Medios para usarla aquí.");
    }

    ui.separator();
    group(ui, "Trazo", false, |ui| {
        sw_nested(ui, e, clip, "Trazo", "cutoutStroke", "active");
        let mut m = obj(clip, "cutoutStroke");
        let mut dirty = false;
        let active = m.get("type").and_then(Value::as_str).unwrap_or("none").to_string();
        if let Some(t) = tiles(ui, CUTOUT_STROKES, &active, |p, r, id| {
            p.text(r.center(), egui::Align2::CENTER_CENTER, if id == "none" { "⊘" } else { "✎" }, egui::FontId::proportional(18.0), theme::TEXT);
        }) {
            m.insert("type".into(), json!(t));
            dirty = true;
        }
        let col = m.get("color").and_then(Value::as_str).unwrap_or("#ffffff").to_string();
        if let Some(c) = color_pick(ui, "Color", &col) {
            m.insert("color".into(), json!(c));
            dirty = true;
        }
        nested_range(ui, e, &mut m, &mut dirty, "Tamaño", "size", 10.0, 0.0, 100.0, 1.0, "");
        nested_range(ui, e, &mut m, &mut dirty, "Espaciado", "spacing", 10.0, 0.0, 100.0, 1.0, "");
        if dirty {
            cset(e, "cutoutStroke", Value::Object(m));
        }
    });
    group(ui, "Eliminación personalizada", false, |ui| {
        sw_nested(ui, e, clip, "Eliminación personalizada", "customCutout", "active");
        hint(ui, "Pintar sobre el reproductor para marcar lo que quieres borrar forma parte de las herramientas de IA; el valor se guarda en el clip.");
    });

    ui.separator();
    group(ui, "Chroma key", true, |ui| {
        sw_nested(ui, e, clip, "Chroma key", "chromaKey", "active");
        let mut m = obj(clip, "chromaKey");
        let mut dirty = false;
        let col = m.get("color").and_then(Value::as_str).unwrap_or("#00ff00").to_string();
        if let Some(c) = color_pick(ui, "Selector de colores", &col) {
            m.insert("color".into(), json!(c));
            m.insert("active".into(), json!(true));
            dirty = true;
        }
        note(ui, "Elige el color y, luego, ajusta la intensidad.");
        nested_range(ui, e, &mut m, &mut dirty, "Intensidad", "strength", 20.0, 0.0, 100.0, 1.0, "");
        nested_range(ui, e, &mut m, &mut dirty, "Sombra", "shadow", 0.0, 0.0, 100.0, 1.0, "");
        nested_range(ui, e, &mut m, &mut dirty, "Borde de plumas", "feather", 0.0, 0.0, 100.0, 1.0, "");
        nested_range(ui, e, &mut m, &mut dirty, "Limpiar borde", "cleanup", 0.0, 0.0, 100.0, 1.0, "");
        if dirty {
            cset(e, "chromaKey", Value::Object(m));
        }
    });
    ui.add_space(6.0);
    secondary_button(ui, "Recortar con una máscara").clicked()
}

fn mask_glyph(p: &egui::Painter, r: Rect, id: &str) {
    let c = r.center();
    let s = r.height() * 0.32;
    let col = theme::TEXT;
    let st = Stroke::new(1.5, col);
    match id {
        "none" => {
            p.text(c, egui::Align2::CENTER_CENTER, "⊘", egui::FontId::proportional(18.0), theme::MUTED);
        }
        "linear" => {
            p.hline(r.x_range().shrink(8.0), c.y, st);
        }
        "mirror" => {
            p.hline(r.x_range().shrink(8.0), c.y - s * 0.5, st);
            p.hline(r.x_range().shrink(8.0), c.y + s * 0.5, st);
        }
        "circle" => {
            p.circle_stroke(c, s, st);
        }
        "rect" => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(s * 2.6, s * 1.8)), 0.0, st, StrokeKind::Middle);
        }
        "rounded" => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(s * 2.6, s * 1.8)), s * 0.5, st, StrokeKind::Middle);
        }
        "heart" => {
            p.text(c, egui::Align2::CENTER_CENTER, "♡", egui::FontId::proportional(20.0), col);
        }
        "star" => {
            p.text(c, egui::Align2::CENTER_CENTER, "☆", egui::FontId::proportional(20.0), col);
        }
        _ => {
            let pts = vec![Pos2::new(c.x, c.y - s), Pos2::new(c.x + s, c.y + s * 0.8), Pos2::new(c.x - s, c.y + s * 0.8)];
            p.add(egui::Shape::closed_line(pts, st));
        }
    }
}

fn mask_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let mut m = obj(clip, "mask");
    let mut dirty = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr("Máscara")).size(12.5).color(theme::TEXT).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if secondary_button(ui, "🗑 Quitar").clicked() {
                m.insert("shape".into(), json!("none"));
                dirty = true;
            }
        });
    });
    let shape = m.get("shape").and_then(Value::as_str).unwrap_or("none").to_string();
    if let Some(s) = tiles(ui, MASK_SHAPES, &shape, mask_glyph) {
        m.insert("shape".into(), json!(s));
        dirty = true;
    }
    if shape == "none" {
        hint(ui, "Elige una forma para recortar el clip. Después ajusta su posición, escala y rotación.");
    } else {
        nested_range(ui, e, &mut m, &mut dirty, "Pluma", "feather", 0.0, 0.0, 100.0, 1.0, "%");
        nested_range(ui, e, &mut m, &mut dirty, "Escala", "scale", 1.0, 10.0, 200.0, 0.01, "%");
        nested_range(ui, e, &mut m, &mut dirty, "Rotación", "rotate", 0.0, -180.0, 180.0, 1.0, "°");
        nested_range(ui, e, &mut m, &mut dirty, "Posición X", "x", 0.0, -100.0, 100.0, 1.0, "%");
        nested_range(ui, e, &mut m, &mut dirty, "Posición Y", "y", 0.0, -100.0, 100.0, 1.0, "%");
        let inv = m.get("invert").and_then(Value::as_bool).unwrap_or(false);
        if switch_row(ui, "Invertir", inv) {
            m.insert("invert".into(), json!(!inv));
            dirty = true;
        }
    }
    if dirty {
        cset(e, "mask", Value::Object(m));
    }
}

fn retouch_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    section(ui, "Retoque");
    let mut m = obj(clip, "beauty");
    let mut dirty = false;
    nested_range(ui, e, &mut m, &mut dirty, "Suavizar piel", "smooth", 0.0, 0.0, 100.0, 1.0, "");
    nested_range(ui, e, &mut m, &mut dirty, "Blanquear", "whiten", 0.0, 0.0, 100.0, 1.0, "");
    nested_range(ui, e, &mut m, &mut dirty, "Sonrojo", "rosy", 0.0, 0.0, 100.0, 1.0, "");
    if dirty {
        cset(e, "beauty", Value::Object(m));
    }
    locked(ui, e, "Adelgazar", "IA", "");
    locked(ui, e, "Ojos", "IA", "");
    locked(ui, e, "Dientes", "IA", "");
    hint(ui, "El suavizado y el blanqueo se aplican al clip en la vista previa y en la exportación.");
}

// ---------------- Audio ----------------

fn audio_basic(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    section(ui, "Básico");
    let db = if clip.volume > 0.0 { (20.0 * clip.volume.log10()).clamp(-30.0, 6.0) } else { -30.0 };
    let shown = (db * 2.0).round() / 2.0;
    if let Some(v) = range(ui, e, "Volumen", shown, -30.0, 6.0, 0.5, 1.0, "dB") {
        let vol = if v <= -30.0 { 0.0 } else { 10f64.powf(v / 20.0).clamp(0.0, 2.0) };
        cset(e, "volume", json!(vol));
    }
    if let Some(v) = range(ui, e, "Aparición progresiva", clip.fade_in, 0.0, 10.0, 0.1, 1.0, "s") {
        cset(e, "fadeIn", json!(v));
    }
    if let Some(v) = range(ui, e, "Desaparición progresiva", clip.fade_out, 0.0, 10.0, 0.1, 1.0, "s") {
        cset(e, "fadeOut", json!(v));
    }
    sw(ui, e, clip, "Normalizar la intensidad", "normalize", "Normaliza el clip al valor objetivo.");
    locked(ui, e, "Mejorar voz", "IA", "");
    locked(ui, e, "Traductor de videos", "IA", "Traduce la voz del clip a otro idioma.");
    locked(ui, e, "Reducir el ruido", "IA", "Elimina el ruido de fondo del audio.");
    locked(ui, e, "Aislar voz", "PRO", "Separa la voz de la música y los efectos.");
    locked(ui, e, "Completar canal", "", "Convierte el audio mono en estéreo.");
}

fn voice_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let active = exs(clip, "voice", "Original").to_string();
    ui.horizontal(|ui| {
        badge(ui, "Efecto de voz");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if secondary_button(ui, "Restablecer").clicked() {
                cset(e, "voice", Value::Null);
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        for name in VOICE_FILTERS {
            if crate::panels::chip(ui, name, active == *name).clicked() {
                cset(e, "voice", if *name == "Original" { Value::Null } else { json!(name) });
            }
        }
    });
    hint(ui, "El filtro de voz se aplica al audio del clip en la vista previa y en la exportación.");
}

// ---------------- Velocidad ----------------

fn curve_glyph(p: &egui::Painter, r: Rect, id: &str) {
    let r = r.shrink2(Vec2::new(10.0, 8.0));
    let ys: &[f32] = match id {
        "none" => {
            p.text(r.center(), egui::Align2::CENTER_CENTER, "⊘", egui::FontId::proportional(18.0), theme::MUTED);
            return;
        }
        "custom" => &[0.5, 0.5, 0.5, 0.5, 0.5],
        "montage" => &[0.3, 0.7, 0.2, 0.8, 0.4],
        "hero" => &[0.7, 0.7, 0.15, 0.7, 0.7],
        "bullet" => &[0.2, 0.8, 0.8, 0.8, 0.2],
        "cut" => &[0.2, 0.2, 0.85, 0.2, 0.2],
        "flashIn" => &[0.85, 0.6, 0.4, 0.3, 0.3],
        _ => &[0.3, 0.3, 0.4, 0.6, 0.85],
    };
    let n = ys.len() as f32 - 1.0;
    let pts: Vec<Pos2> = ys
        .iter()
        .enumerate()
        .map(|(i, y)| Pos2::new(r.left() + r.width() * i as f32 / n, r.top() + r.height() * y))
        .collect();
    p.add(egui::Shape::line(pts, Stroke::new(1.5, theme::ACCENT)));
}

fn speed_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip, sub: &str) {
    match sub {
        "curve" => {
            if let Some(c) = tiles(ui, SPEED_CURVES, exs(clip, "speedCurve", "none"), curve_glyph) {
                cset(e, "speedCurve", json!(c));
            }
            hint(ui, "Las curvas de velocidad marcan aceleraciones progresivas dentro del clip.");
        }
        "fx" => {
            if let Some(c) = tiles(ui, SPEED_FX, exs(clip, "speedFx", "none"), |p, r, id| {
                let g = match id {
                    "none" => "⊘",
                    "flash" => "⚡",
                    "motion" => "➶",
                    "fade" => "◐",
                    "zoom" => "⊕",
                    _ => "≈",
                };
                p.text(r.center(), egui::Align2::CENTER_CENTER, g, egui::FontId::proportional(18.0), theme::TEXT);
            }) {
                cset(e, "speedFx", json!(c));
            }
        }
        _ => {
            section(ui, "Estándar");
            if let Some(v) = range(ui, e, "Velocidad", clip.speed, 10.0, 1000.0, 5.0, 0.01, "%") {
                cset(e, "speed", json!(v));
            }
            ui.label(egui::RichText::new(format!("{:.2}×", clip.speed)).size(11.0).color(theme::MUTED));
            info_row(ui, "Duración", &format_time(clip.duration));
            let on = ex(clip, "preservePitch").as_bool() != Some(false);
            if switch_row(ui, "Cambiar tono del audio", on) {
                cset(e, "preservePitch", json!(!on));
            }
            note(ui, "Si se desactiva, el audio mantiene su tono al cambiar la velocidad.");
            hint(ui, "Cámara lenta fluida: solo se admiten videos en cámara lenta.");
        }
    }
}

// ---------------- Animación ----------------

fn animation_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip, sub: &str) {
    let (list, prop) = match sub {
        "in" => (ANIM_IN, "animIn"),
        "out" => (ANIM_OUT, "animOut"),
        _ => (ANIM_COMBO, "animCombo"),
    };
    let active = exs(clip, prop, "").to_string();
    let mut items: Vec<(&str, &str)> = vec![("", "Ninguno")];
    items.extend(list.iter().map(|(id, name, _)| (*id, *name)));
    let mut pick: Option<Option<String>> = None;
    grid(ui, &items, 72.0, |ui, (id, name), w| {
        let none = id.is_empty();
        let on = if none { active.is_empty() } else { active == *id };
        let (img, r) = card(ui, w, w * 0.62, name, on);
        let p = ui.painter_at(img);
        paint::rounded_gradient(&p, img, 6, Color32::from_rgb(0x1f, 0x2b, 0x2e), Color32::from_rgb(0x14, 0x18, 0x1a));
        if r.hovered() && !none {
            // Vista previa en vivo: un cuadrado de muestra con la animación en bucle.
            let t = ui.input(|i| i.time);
            let ca = crate::clip_anim::preview(sub, id, t);
            let c = img.center() + egui::vec2(ca.dx * img.width(), ca.dy * img.height());
            let hs = img.height() * 0.28 * ca.scale;
            let (sn, cs) = ca.rot.to_radians().sin_cos();
            let pts: Vec<egui::Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|(x, y): &(f32, f32)| c + egui::vec2((x * cs - y * sn) * hs, (x * sn + y * cs) * hs))
                .collect();
            let col = theme::ACCENT.gamma_multiply(ca.alpha.clamp(0.0, 1.0));
            p.add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
            ui.ctx().request_repaint();
        } else {
            p.text(img.center(), egui::Align2::CENTER_CENTER, if none { "⊘" } else { "✦" }, egui::FontId::proportional(18.0), theme::TEXT);
        }
        if r.on_hover_text(*name).clicked() {
            pick = Some(if none { None } else { Some(id.to_string()) });
        }
    });
    match pick {
        Some(None) => cset(e, prop, Value::Null),
        Some(Some(id)) => cset(e, prop, json!(id)),
        None => {}
    }
}

// ---------------- Efectos ----------------

/// Biblioteca de efectos parametrizables por categoría (`fx_library`). La
/// cuadrícula elige el efecto (clave `effect`, compartida con las animaciones) y
/// los deslizadores editan sus propiedades en `effectParams`; ambos alimentan el
/// mismo motor de render que la vista previa y la exportación.
/// Escena de muestra procedural (cielo, sol, colinas, rejilla de color).
fn sample_scene(w: usize, h: usize) -> Vec<u8> {
    let mut v = vec![255u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let mut c = [40.0 + 120.0 * fy, 90.0 + 100.0 * (1.0 - fy), 200.0 - 80.0 * fy];
            let (dx, dy) = (fx - 0.7, fy - 0.3);
            if dx * dx + dy * dy < 0.018 {
                c = [255.0, 220.0, 90.0];
            }
            let hill = 0.72 + 0.1 * (fx * 7.0).sin();
            if fy > hill {
                c = [50.0 + 60.0 * fx, 150.0 - 40.0 * fy, 60.0];
            }
            if (fx - 0.28).abs() < 0.1 && (fy - 0.55).abs() < 0.14 {
                c = [220.0, 60.0 + 120.0 * fx, 80.0];
            }
            let i = (y * w + x) * 4;
            v[i] = c[0] as u8;
            v[i + 1] = c[1] as u8;
            v[i + 2] = c[2] as u8;
        }
    }
    v
}

/// Miniatura del efecto: motor geométrico + cadena de color sobre la escena.
fn fx_thumb(ctx: &egui::Context, id: &str) -> egui::TextureHandle {
    let key = egui::Id::new(("fxthumb", id));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return t;
    }
    let (w, h) = (96usize, 60usize);
    let mut px = sample_scene(w, h);
    // Con sus valores por defecto algunos efectos son la identidad (p. ej. la
    // escala al 100 %); la miniatura usa una muestra que sí se ve.
    let demo = (id == "mov_scaleassist").then(|| json!({ "scale": 150.0 }));
    if let Some(wp) = paint::warp_for(id, demo.as_ref()) {
        px = paint::warp_rgba(&px, w, h, &wp);
    }
    if let Some(css) = crate::fx_library::css_for(id, None) {
        let fx = paint::parse_css(&css);
        if !fx.is_identity() {
            for c in px.chunks_exact_mut(4) {
                let o = paint::apply_fx(Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]), &fx).to_srgba_unmultiplied();
                c.copy_from_slice(&o);
            }
        }
    }
    let t = ctx.load_texture(format!("fxthumb_{id}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &px), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    t
}

fn effects_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip, cat: &str) {
    let active = exs(clip, "effect", "").to_string();
    section(ui, "Biblioteca de efectos");
    let mut items: Vec<Option<&'static crate::fx_library::FxDef>> = vec![None];
    items.extend(crate::fx_library::in_category(cat).map(Some));
    let mut pick: Option<Option<&'static str>> = None;
    grid(ui, &items, 84.0, |ui, it, w| {
        let (id, label, on) = match it {
            None => ("", "Ninguno", active.is_empty()),
            Some(d) => (d.id, d.label, active == d.id),
        };
        let (img, r) = card(ui, w, w * 0.56, label, on);
        let p = ui.painter_at(img);
        paint::rounded_gradient(&p, img, 6, Color32::from_rgb(0x20, 0x2a, 0x30), Color32::from_rgb(0x13, 0x17, 0x1b));
        if id.is_empty() {
            p.text(img.center(), egui::Align2::CENTER_CENTER, "⊘", egui::FontId::proportional(18.0), theme::TEXT);
        } else {
            let tex = fx_thumb(ui.ctx(), id);
            p.image(tex.id(), img.shrink(1.0), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        if r.on_hover_text(label).clicked() {
            pick = Some((!id.is_empty()).then_some(id));
        }
    });
    match pick {
        Some(None) => {
            cset(e, "effect", Value::Null);
            cset(e, "effectParams", Value::Null);
        }
        Some(Some(id)) => {
            cset(e, "effect", json!(id));
            // Nuevo efecto: vuelve a sus valores por defecto.
            cset(e, "effectParams", Value::Null);
        }
        None => {}
    }
    // Propiedades del efecto activo (solo si pertenece a la biblioteca).
    if let Some(def) = crate::fx_library::by_id(&active) {
        ui.add_space(4.0);
        section(ui, "Propiedades");
        if def.approx {
            hint(ui, "Aproximado con color/desenfoque: este efecto depende del tiempo o del texto y aún no tiene motor propio.");
        }
        let mut m = obj(clip, "effectParams");
        let mut dirty = false;
        for prm in def.params {
            if !prm.choices.is_empty() {
                let cur = (mn(&m, prm.key, prm.default) as usize).min(prm.choices.len() - 1);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(crate::i18n::tr(&prm.label)).size(12.0).color(theme::TEXT));
                    egui::ComboBox::from_id_salt(("fx-choice", prm.key)).selected_text(prm.choices[cur]).show_ui(ui, |ui| {
                        for (n, name) in prm.choices.iter().enumerate() {
                            if ui.selectable_label(n == cur, *name).clicked() && n != cur {
                                m.insert(prm.key.into(), json!(n));
                                dirty = true;
                            }
                        }
                    });
                });
                continue;
            }
            nested_range(ui, e, &mut m, &mut dirty, prm.label, prm.key, prm.default, prm.min, prm.max, 1.0, prm.unit);
        }
        if dirty {
            cset(e, "effectParams", Value::Object(m));
        }
        ui.add_space(2.0);
        if secondary_button(ui, "↺ Restablecer propiedades").clicked() {
            cset(e, "effectParams", Value::Null);
        }
    } else if !active.is_empty() {
        hint(ui, "Este efecto no tiene propiedades configurables.");
    }
}

// ---------------- Ajustar ----------------

fn adjust_basic(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let mut patch = Map::new();
    let mut slider = |ui: &mut egui::Ui, e: &mut Edit, label: &str, key: &str, def: f64, min: f64, max: f64, unit: &str| {
        if let Some(v) = range(ui, e, label, adj(clip, key, def), min, max, 1.0, 1.0, unit) {
            patch.insert(key.to_string(), json!(v));
        }
    };
    section(ui, "Básico");
    slider(ui, e, "Intensidad", "intensity", 80.0, 0.0, 100.0, "");
    ui.horizontal(|ui| {
        let on = flag(clip, "colorMix");
        if switch_row(ui, "Combinación de colores", on) {
            cset(e, "colorMix", json!(!on));
        }
    });
    slider(ui, e, "Corrección del color", "correction", 80.0, 0.0, 100.0, "");

    // LUT (`data-clip-filter`).
    let cur = cupcraft_core::FILTERS
        .iter()
        .find(|f| f.key == clip.filter)
        .map(|f| f.label)
        .unwrap_or("Ninguno");
    let mut lut: Option<&str> = None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr("LUT")).size(11.5).color(theme::MUTED));
        egui::ComboBox::from_id_salt("clip-lut").selected_text(cur).width(170.0).show_ui(ui, |ui| {
            if ui.selectable_label(clip.filter == "none", crate::i18n::tr("Ninguno")).clicked() {
                lut = Some("none");
            }
            for f in cupcraft_core::FILTERS.iter() {
                if ui.selectable_label(f.key == clip.filter, f.label).clicked() {
                    lut = Some(f.key);
                }
            }
        });
    });
    if let Some(k) = lut {
        e.clip.insert("filter".into(), json!(k));
        e.clip.insert("lutKey".into(), json!(k));
    }

    section(ui, "Ajustar");
    slider(ui, e, "Temp.", "temperature", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Tinte", "tint", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Saturación", "saturate", 100.0, 0.0, 300.0, "");
    slider(ui, e, "Claridad", "clarity", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Exposición", "exposure", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Contraste", "contrast", 100.0, 0.0, 200.0, "");
    slider(ui, e, "Luminar", "highlight", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Sombra", "shadow", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Blancos", "whites", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Negros", "blacks", 0.0, -100.0, 100.0, "");
    slider(ui, e, "Luminosidad", "brightness", 100.0, 0.0, 200.0, "%");
    section(ui, "Efectos");
    slider(ui, e, "Enfoque", "sharpen", 0.0, 0.0, 100.0, "");
    slider(ui, e, "Partículas", "particle", 0.0, 0.0, 100.0, "");
    slider(ui, e, "Desvanecer", "fade", 0.0, 0.0, 100.0, "");
    slider(ui, e, "Viñeta", "vignette", 0.0, 0.0, 100.0, "");
    if !patch.is_empty() {
        cset(e, "adjust", Value::Object(patch));
    }
}

fn hsl_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let mut m = obj(clip, "hsl");
    let mut dirty = false;
    let active = m.get("color").and_then(Value::as_str).unwrap_or("red").to_string();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (id, label, hex) in HSL_COLORS {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
            ui.painter().circle_filled(r.center(), 9.0, paint::hex(hex));
            if active == *id {
                ui.painter().circle_stroke(r.center(), 10.5, Stroke::new(2.0, theme::TEXT));
            }
            if resp.on_hover_text(*label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                m.insert("color".into(), json!(id));
                dirty = true;
            }
        }
    });
    ui.add_space(4.0);
    nested_range(ui, e, &mut m, &mut dirty, "Matiz", "hue", 0.0, -180.0, 180.0, 1.0, "°");
    nested_range(ui, e, &mut m, &mut dirty, "Saturación", "sat", 0.0, -100.0, 100.0, 1.0, "");
    nested_range(ui, e, &mut m, &mut dirty, "Brillo", "light", 0.0, -100.0, 100.0, 1.0, "");
    if dirty {
        cset(e, "hsl", Value::Object(m));
    }
    hint(ui, "Ajusta cada familia de color por separado (como el HSL de CapCut).");
}

fn points_of(v: Option<&Value>) -> Vec<[f32; 2]> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    let p = p.as_array()?;
                    Some([p.first()?.as_f64()? as f32, p.get(1)?.as_f64()? as f32])
                })
                .collect()
        })
        .unwrap_or_default()
}

fn round2(v: f32) -> f64 {
    ((v as f64) * 100.0).round() / 100.0
}

fn curves_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let channel = exs(clip, "curveChannel", "luma").to_string();
    ui.horizontal(|ui| {
        for (id, label, _) in CURVE_CHANNELS {
            if crate::panels::chip(ui, label, channel == *id).clicked() {
                cset(e, "curveChannel", json!(id));
            }
        }
    });
    let color = CURVE_CHANNELS.iter().find(|c| c.0 == channel).map(|c| c.2).unwrap_or(theme::TEXT);
    let mut curves = obj(clip, "curves");
    let mut pts = points_of(curves.get(&channel));

    let side = ui.available_width().min(260.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, theme::FIELD);
    for i in 1..4 {
        let t = i as f32 / 4.0;
        let gs = Stroke::new(1.0, theme::PANEL3);
        p.vline(rect.left() + rect.width() * t, rect.y_range(), gs);
        p.hline(rect.x_range(), rect.top() + rect.height() * t, gs);
    }
    let to_screen = |q: [f32; 2]| Pos2::new(rect.left() + q[0] * rect.width(), rect.bottom() - q[1] * rect.height());
    let from_screen = |s: Pos2| [((s.x - rect.left()) / rect.width()).clamp(0.0, 1.0), ((rect.bottom() - s.y) / rect.height()).clamp(0.0, 1.0)];

    // Curva suave como `curvePathFromPoints` (Bézier con control en x medio).
    let mut sorted = pts.clone();
    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]));
    if sorted.is_empty() {
        p.line_segment([rect.left_bottom(), rect.right_top()], Stroke::new(1.5, color));
    } else {
        let mut line = vec![to_screen(sorted[0])];
        for w in sorted.windows(2) {
            let (a, b) = (w[0], w[1]);
            let cx = (a[0] + b[0]) / 2.0;
            for k in 1..=16 {
                let t = k as f32 / 16.0;
                let mt = 1.0 - t;
                let x = mt * mt * mt * a[0] + 3.0 * mt * mt * t * cx + 3.0 * mt * t * t * cx + t * t * t * b[0];
                let y = mt * mt * mt * a[1] + 3.0 * mt * mt * t * a[1] + 3.0 * mt * t * t * b[1] + t * t * t * b[1];
                line.push(to_screen([x, y]));
            }
        }
        p.add(egui::Shape::line(line, Stroke::new(1.5, color)));
    }
    for q in &pts {
        p.circle_filled(to_screen(*q), 4.5, color);
        p.circle_stroke(to_screen(*q), 4.5, Stroke::new(1.0, Color32::BLACK));
    }

    let drag_id = egui::Id::new(("curve-drag", &clip.id));
    let near = |pos: Pos2, pts: &[[f32; 2]]| pts.iter().position(|q| to_screen(*q).distance(pos) < 8.0);
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.double_clicked() {
            if let Some(i) = near(pos, &pts) {
                pts.remove(i);
                changed = true;
            }
        } else if resp.drag_started() {
            ui.data_mut(|d| d.insert_temp(drag_id, near(pos, &pts)));
        } else if resp.dragged() {
            if let Some(Some(i)) = ui.data(|d| d.get_temp::<Option<usize>>(drag_id)) {
                if i < pts.len() {
                    pts[i] = from_screen(pos);
                    changed = true;
                    e.silent = true;
                }
            }
        } else if resp.clicked() && near(pos, &pts).is_none() {
            if pts.is_empty() {
                pts = vec![[0.0, 0.0], [1.0, 1.0]];
            }
            pts.push(from_screen(pos));
            changed = true;
        }
    }
    if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<Option<usize>>(drag_id));
        e.save = true;
    }
    if changed {
        pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let arr: Vec<Value> = pts.iter().map(|q| json!([round2(q[0]), round2(q[1])])).collect();
        curves.insert(channel, Value::Array(arr));
        cset(e, "curves", Value::Object(curves));
    }
    hint(ui, "Haz clic en la curva para añadir un punto. Arrastra los círculos para ajustar la curva y haz doble clic en un círculo para quitarlo.");
}

fn wheel_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let mut wheels = obj(clip, "wheels");
    let mut dirty = false;
    let w = ((ui.available_width() - 12.0) / 2.0).clamp(80.0, 130.0);
    for row in WHEELS.chunks(2) {
        ui.horizontal(|ui| {
            for (id, label) in row {
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.label(egui::RichText::new(*label).size(11.5).color(theme::MUTED));
                    let v = wheels.get(*id).and_then(Value::as_object).cloned().unwrap_or_default();
                    let (x, y, z) = (mn(&v, "x", 0.0), mn(&v, "y", 0.0), mn(&v, "z", 0.0));
                    let d = w - 10.0;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(d), Sense::click_and_drag());
                    let p = ui.painter_at(rect);
                    let c = rect.center();
                    let r = d / 2.0 - 2.0;
                    // Disco de matiz.
                    let n = 48;
                    for i in 0..n {
                        let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
                        let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
                        let col = egui::ecolor::Hsva::new(i as f32 / n as f32, 0.55, 0.75, 1.0);
                        let pts = vec![c, c + Vec2::angled(a0) * r, c + Vec2::angled(a1) * r];
                        p.add(egui::Shape::convex_polygon(pts, Color32::from(col), Stroke::NONE));
                    }
                    p.circle_filled(c, r * 0.35, Color32::from_black_alpha(90));
                    p.circle_stroke(c, r, Stroke::new(1.0, theme::PANEL3));
                    let dot = Pos2::new(c.x + x as f32 * r * 0.8, c.y + y as f32 * r * 0.8);
                    p.circle_filled(dot, 5.0, Color32::WHITE);
                    p.circle_stroke(dot, 5.0, Stroke::new(1.0, Color32::BLACK));
                    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.clicked() || resp.dragged()) {
                        let nx = ((pos.x - c.x) / (r * 0.8)).clamp(-1.0, 1.0);
                        let ny = ((pos.y - c.y) / (r * 0.8)).clamp(-1.0, 1.0);
                        let mut nv = v.clone();
                        nv.insert("x".into(), json!(round2(nx)));
                        nv.insert("y".into(), json!(round2(ny)));
                        wheels.insert(id.to_string(), Value::Object(nv));
                        dirty = true;
                        if resp.dragged() && !resp.drag_started() {
                            e.silent = true;
                        }
                    }
                    if resp.drag_stopped() {
                        e.save = true;
                    }
                    if resp.double_clicked() {
                        wheels.insert(id.to_string(), json!({ "x": 0, "y": 0 }));
                        dirty = true;
                    }
                    ui.label(egui::RichText::new(format!("{x:.2}  {y:.2}  {z:.2}")).size(10.5).color(theme::MUTED2).monospace());
                });
            }
        });
        ui.add_space(6.0);
    }
    if dirty {
        cset(e, "wheels", Value::Object(wheels));
    }
    hint(ui, "Arrastra el punto del círculo para teñir sombras, medios y luces.");
}

// ---------------- Pie ----------------

/// Transición al inicio + meta + «Restablecer ajustes». Devuelve `true` si
/// se pulsó restablecer.
fn footer(ui: &mut egui::Ui, _e: &mut Edit, clip: &Clip) -> bool {
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(format!("Inicio {} · Duración {}", format_time(clip.start), format_time(clip.duration)))
            .size(11.0)
            .color(theme::MUTED),
    );
    ui.add_space(4.0);
    let w = ui.available_width();
    ui.add_sized([w, 26.0], egui::Button::new(egui::RichText::new(crate::i18n::tr("↺ Restablecer ajustes")).color(theme::TEXT)).frame(true))
        .clicked()
}

/// `data-reset-clip`: el ajuste de color vuelve a sus valores por defecto
/// (sin los extras de presets), igual que reemplazar `adjust` en el original.
fn reset_clip(app: &mut CupApp, id: &str) {
    let patch = json!({
        "filter": "none", "effect": "none",
        "opacity": 1.0, "scale": 1.0, "rotate": 0.0, "x": 0.0, "y": 0.0,
        "volume": 1.0, "speed": 1.0, "fadeIn": 0.0, "fadeOut": 0.0,
    });
    app.mutate_project(|p| {
        p.update_clip(id, patch);
        if let Some(c) = p.find_clip_mut(id) {
            if let Ok(a) = serde_json::from_value(json!({ "brightness": 100, "contrast": 100, "saturate": 100 })) {
                c.adjust = a;
            }
        }
    });
    app.save_current();
}
