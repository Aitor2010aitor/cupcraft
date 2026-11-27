//! Detalles de un clip de texto (`TEXT_DETAIL_TABS` / `textDetailTabBody` de
//! `src/editor/panels.js`):
//!
//! * Texto: Básico (`textControls`) · Burbuja · Efectos
//! * Animación: Entrada · Bucle · Salida · Subtítulos
//! * Seguimiento · Texto a voz · Avatares de IA
//!
//! Todos los cambios van a `clip.text` (o a la transformación del clip) con
//! `update_clip`. Arrastrar un deslizador deja un solo paso de deshacer.

use crate::app::{CupApp, LeftView, PanelTab};
use crate::panels::{chip, grid, hint, primary_button, secondary_button, section, text_card};
use crate::paint;
use crate::text_styles as cat;
use crate::theme;
use cupcraft_core::Clip;
use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Vec2};
use serde_json::{json, Map, Value};

const TABS: &[(&str, &str, &[(&str, &str)])] = &[
    ("text", "Texto", &[("basic", "Básico"), ("bubble", "Burbuja"), ("effects", "Efectos")]),
    ("textanim", "Animación", &[("in", "Entrada"), ("loop", "Bucle"), ("out", "Salida"), ("caption", "Subtítulos")]),
    ("tracking", "Seguimiento", &[("tracking", "Seguimiento")]),
    ("tts", "Texto a voz", &[("basic", "Básico")]),
    ("avatar", "Avatares de IA", &[("basic", "Básico")]),
];

#[derive(Clone)]
struct TabState {
    tab: usize,
    subs: [usize; 5],
}

impl Default for TabState {
    fn default() -> Self {
        Self { tab: 0, subs: [0; 5] }
    }
}

/// `TEXT_DEFAULTS` de `src/herramientas/store.js`.
fn defaults() -> Value {
    json!({
        "size": 72, "weight": 800, "color": "#ffffff", "align": "center", "font": "Inter",
        "x": 0, "y": 0, "letterSpacing": 0, "lineSpacing": 1.18,
        "bold": false, "italic": false, "underline": false, "textCase": "none", "opacity": 1,
        "shadow": true, "shadowColor": "#000000", "shadowAlpha": 0.58, "shadowBlur": 0.2,
        "shadowDistance": 0.05, "shadowAngle": 90,
        "strokeColor": "#000000", "strokeWidth": 0, "strokeAlpha": 1, "strokeMode": "outer", "strokeSmoothing": 0,
        "background": "transparent", "backgroundAlpha": 1, "backgroundStyle": "rounded",
        "backgroundRadius": 0.16, "backgroundPadding": 0.22,
        "backgroundBorderColor": "#000000", "backgroundBorderWidth": 0,
        "glow": false, "glowColor": "#00e5d0", "glowBlur": 0.35,
        "curve": 0, "flipX": false, "flipY": false,
        "animation": "none", "animationPhase": "in", "animationDuration": 0.6
    })
}

/// Cambios acumulados durante un fotograma.
#[derive(Default)]
pub(crate) struct Edit {
    pub text: Map<String, Value>,
    pub clip: Map<String, Value>,
    /// Arrastre en curso: no apila historial en cada fotograma.
    pub silent: bool,
    /// Fin de arrastre: guardar en disco.
    pub save: bool,
    pub toast: Option<String>,
}

impl Edit {
    pub fn track(&mut self, r: &egui::Response) -> bool {
        if r.drag_stopped() {
            self.save = true;
        }
        if r.changed() && r.dragged() && !r.drag_started() {
            self.silent = true;
        }
        r.changed()
    }
    fn set(&mut self, key: &str, v: Value) {
        self.text.insert(key.to_string(), v);
    }
    fn merge(&mut self, patch: &Value) {
        if let Some(o) = patch.as_object() {
            for (k, v) in o {
                self.text.insert(k.clone(), v.clone());
            }
        }
    }
}

pub(crate) fn apply(app: &mut CupApp, id: &str, e: Edit) {
    if let Some(msg) = e.toast {
        app.toast(msg);
    }
    if e.text.is_empty() && e.clip.is_empty() {
        if e.save {
            app.save_current();
        }
        return;
    }
    let mut patch = e.clip;
    if !e.text.is_empty() {
        patch.insert("text".into(), Value::Object(e.text));
    }
    let patch = Value::Object(patch);
    if e.silent {
        // Solo lo que afecta al audio obliga a clonar el proyecto al motor.
        let audio = patch.as_object().is_some_and(|o| {
            o.keys().any(|k| !matches!(k.as_str(), "text" | "x" | "y" | "scale" | "rotate" | "opacity" | "fadeIn" | "fadeOut" | "filter" | "adjust" | "extra" | "keyframes" | "transition" | "transitionDuration" | "blend"))
        });
        app.editor.mutate_silent(|p| p.update_clip(id, patch));
        if audio {
            app.sync_engine();
        }
        if e.save {
            app.save_current();
        }
    } else {
        app.mutate_project(|p| p.update_clip(id, patch));
        app.save_current();
    }
}

fn n(v: &Value, k: &str) -> f64 {
    paint::num_of(v, k, paint::num_of(&defaults(), k, 0.0))
}
fn s<'a>(v: &'a Value, k: &str, d: &'a str) -> &'a str {
    paint::str_of(v, k, d)
}
fn b(v: &Value, k: &str, d: bool) -> bool {
    paint::bool_of(v, k, d)
}

pub(crate) fn to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Fila «etiqueta ……… valor» + deslizador (`textRangeField`): el valor
/// guardado es `mostrado * factor`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn range(ui: &mut egui::Ui, e: &mut Edit, label: &str, cur: f64, min: f64, max: f64, step: f64, factor: f64, unit: &str) -> Option<f64> {
    range_ex(ui, e, label, cur, min, max, step, factor, unit, None).0
}

/// Estado del rombo de fotogramas clave de un control.
#[derive(Clone, Copy)]
pub(crate) struct KfState {
    /// La propiedad tiene fotogramas clave (está animada).
    pub animated: bool,
    /// Hay un fotograma clave justo en el cursor de reproducción.
    pub on_key: bool,
}

/// Rombo de fotogramas clave: relleno = hay uno aquí, contorno = animada,
/// gris = fija. Devuelve `true` si se pulsó.
fn kf_diamond(ui: &mut egui::Ui, st: KfState) -> bool {
    let (rect, r) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
    let c = rect.center();
    let hot = r.hovered();
    let rad = if hot { 6.5 } else { 5.5 };
    let pts = vec![egui::pos2(c.x, c.y - rad), egui::pos2(c.x + rad, c.y), egui::pos2(c.x, c.y + rad), egui::pos2(c.x - rad, c.y)];
    let col = if st.animated { theme::ACCENT } else if hot { theme::TEXT } else { theme::MUTED };
    let fill = if st.on_key { col } else { Color32::TRANSPARENT };
    ui.painter().add(egui::Shape::convex_polygon(pts, fill, Stroke::new(1.4, col)));
    let tip = if st.on_key {
        crate::i18n::tr("Quitar fotograma clave aquí")
    } else if st.animated {
        crate::i18n::tr("Añadir fotograma clave en este instante")
    } else {
        crate::i18n::tr("Animar: crear fotograma clave en este instante")
    };
    r.on_hover_text(tip.to_string()).clicked()
}

/// Como `range`, con el rombo de fotogramas clave junto a la etiqueta.
/// Devuelve (nuevo valor, rombo pulsado).
#[allow(clippy::too_many_arguments)]
pub(crate) fn range_ex(ui: &mut egui::Ui, e: &mut Edit, label: &str, cur: f64, min: f64, max: f64, step: f64, factor: f64, unit: &str, kf: Option<KfState>) -> (Option<f64>, bool) {
    let mut shown = cur / factor;
    let mut diamond = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr(&label)).size(11.5).color(theme::MUTED));
        if let Some(st) = kf {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                diamond = kf_diamond(ui, st);
            });
        }
    });
    let r = ui.add(
        egui::Slider::new(&mut shown, min..=max)
            .step_by(step)
            .suffix(unit)
            .fixed_decimals(if step < 1.0 { 2 } else { 0 }),
    );
    (e.track(&r).then_some(shown * factor), diamond)
}

fn text_range(ui: &mut egui::Ui, e: &mut Edit, t: &Value, label: &str, key: &str, min: f64, max: f64, step: f64, factor: f64, unit: &str) {
    if let Some(v) = range(ui, e, label, n(t, key), min, max, step, factor, unit) {
        e.set(key, json!(v));
    }
}

fn color_row(ui: &mut egui::Ui, e: &mut Edit, label: &str, key: &str, cur: &str) {
    ui.horizontal(|ui| {
        let mut c = paint::hex(cur);
        if ui.color_edit_button_srgba(&mut c).changed() {
            e.set(key, json!(to_hex(c)));
        }
        ui.label(egui::RichText::new(crate::i18n::tr(&label)).size(11.5).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr(&cur)).size(11.0).color(theme::TEXT).monospace());
        });
    });
}

pub(crate) fn switch_row(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr(&label)).size(12.0).color(theme::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, r) = ui.allocate_exact_size(Vec2::new(30.0, 16.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, 8.0, if on { theme::ACCENT } else { theme::FIELD });
            let x = if on { rect.right() - 8.0 } else { rect.left() + 8.0 };
            p.circle_filled(egui::pos2(x, rect.center().y), 6.0, if on { theme::ACCENT_INK } else { theme::MUTED });
            clicked = r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        });
    });
    clicked
}

/// Grupo plegable (`<details class="tx-group">`).
pub(crate) fn group(ui: &mut egui::Ui, title: &str, open: bool, body: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(egui::RichText::new(crate::i18n::tr(&title)).size(12.5).color(theme::TEXT).strong())
        .id_salt(("tx-group", title))
        .default_open(open)
        .show(ui, body);
}

/// Pestañas superiores con subrayado cian (`.detail-tab`).
pub(crate) fn tab_row(ui: &mut egui::Ui, labels: &[&str], active: usize, size: f32) -> Option<usize> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        for (i, l) in labels.iter().enumerate() {
            let font = egui::FontId::proportional(size);
            let g = ui.painter().layout_no_wrap(l.to_string(), font.clone(), theme::TEXT);
            let (rect, r) = ui.allocate_exact_size(Vec2::new(g.size().x, size + 10.0), Sense::click());
            let on = i == active;
            let col = if on { theme::TEXT } else if r.hovered() { theme::TEXT } else { theme::MUTED };
            ui.painter().text(egui::pos2(rect.left(), rect.top() + 2.0), egui::Align2::LEFT_TOP, *l, font, col);
            if on {
                ui.painter().hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(2.0, theme::ACCENT));
            }
            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                out = Some(i);
            }
        }
    });
    out
}

pub fn show(ui: &mut egui::Ui, app: &mut CupApp, clip: &Clip) {
    let id = clip.id.clone();
    let mut text = defaults();
    if let (Some(t), Some(o)) = (text.as_object_mut(), clip.text.as_ref().and_then(Value::as_object)) {
        for (k, v) in o {
            t.insert(k.clone(), v.clone());
        }
    }

    // Cabecera: punto · nombre · chip «Texto».
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 3.5, theme::ACCENT);
        let name = if clip.name.is_empty() { "Texto" } else { clip.name.as_str() };
        ui.label(egui::RichText::new(crate::i18n::tr(&name)).color(theme::TEXT).strong());
        egui::Frame::NONE
            .fill(theme::PANEL3)
            .corner_radius(CornerRadius::from(4))
            .inner_margin(egui::Margin::symmetric(6, 1))
            .show(ui, |ui| ui.label(egui::RichText::new(crate::i18n::tr("Texto")).size(10.5).color(theme::MUTED)));
    });
    ui.add_space(4.0);

    let state_id = egui::Id::new("text-detail-tabs");
    let mut st: TabState = ui.data_mut(|d| d.get_temp(state_id)).unwrap_or_default();
    let labels: Vec<&str> = TABS.iter().map(|t| t.1).collect();
    if let Some(i) = tab_row(ui, &labels, st.tab, 13.0) {
        st.tab = i;
    }
    let (tab, _, subs) = TABS[st.tab];
    if subs.len() > 1 {
        let sl: Vec<&str> = subs.iter().map(|x| x.1).collect();
        if let Some(i) = tab_row(ui, &sl, st.subs[st.tab], 12.0) {
            st.subs[st.tab] = i;
        }
    }
    let sub = subs[st.subs[st.tab].min(subs.len() - 1)].0;
    ui.data_mut(|d| d.insert_temp(state_id, st));
    ui.separator();
    ui.add_space(4.0);

    let mut e = Edit::default();
    match (tab, sub) {
        ("text", "bubble") => bubble_tab(ui, &mut e),
        ("text", "effects") => effects_tab(ui, &mut e),
        ("text", _) => basic_tab(ui, &mut e, clip, &text),
        ("textanim", g) => anim_tab(ui, &mut e, &text, g),
        ("tracking", _) => tracking_tab(ui, &mut e, clip),
        ("tts", _) => {
            tts_tab(ui, app, &text);
        }
        _ => hint(
            ui,
            "Los avatares de IA requieren procesamiento en la nube y no están disponibles en esta versión local. El resto de opciones de texto sí funcionan sin conexión.",
        ),
    }
    apply(app, &id, e);
}

// ---------------- Texto · Básico (`textControls`) ----------------
fn basic_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip, t: &Value) {
    section(ui, "Contenido");
    let mut content = s(t, "content", "").to_string();
    let r = ui.add(egui::TextEdit::multiline(&mut content).desired_rows(3).desired_width(f32::INFINITY).char_limit(400));
    if r.changed() {
        e.silent = true;
        e.set("content", json!(content));
    }
    if r.lost_focus() {
        e.save = true;
    }

    group(ui, "Fuente", true, |ui| {
        let font = s(t, "font", "Inter").to_string();
        egui::ComboBox::from_id_salt("tx-font").width(ui.available_width() - 8.0).selected_text(&font).show_ui(ui, |ui| {
            for f in cat::text_fonts() {
                let v = s(f, "value", "");
                if ui.selectable_label(v == font, s(f, "label", v)).clicked() {
                    e.set("font", json!(v));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Tamaño")).size(11.5).color(theme::MUTED));
            let mut size = n(t, "size").round();
            let r = ui.add(egui::DragValue::new(&mut size).range(8.0..=400.0).speed(1.0));
            if e.track(&r) {
                e.set("size", json!(size));
            }
            ui.label(egui::RichText::new(crate::i18n::tr("Grosor")).size(11.5).color(theme::MUTED));
            let weight = n(t, "weight") as i64;
            let wl = cat::text_weights()
                .iter()
                .find(|w| w.get("value").and_then(Value::as_i64) == Some(weight))
                .map(|w| s(w, "label", "").to_string())
                .unwrap_or_else(|| weight.to_string());
            egui::ComboBox::from_id_salt("tx-weight").selected_text(wl).show_ui(ui, |ui| {
                for w in cat::text_weights() {
                    let v = w.get("value").and_then(Value::as_i64).unwrap_or(400);
                    if ui.selectable_label(v == weight, s(w, "label", "")).clicked() {
                        e.set("weight", json!(v));
                    }
                }
            });
        });
    });

    group(ui, "Diseño", true, |ui| {
        ui.horizontal(|ui| {
            for (k, l, tip) in [("bold", "B", "Negrita"), ("italic", "I", "Cursiva"), ("underline", "U", "Subrayado")] {
                let on = b(t, k, false);
                if chip(ui, l, on).on_hover_text(tip).clicked() {
                    e.set(k, json!(!on));
                }
            }
        });
    });

    group(ui, "May./mín.", true, |ui| {
        ui.horizontal_wrapped(|ui| {
            let case = s(t, "textCase", "none");
            for c in cat::text_cases() {
                let v = s(c, "value", "");
                if chip(ui, s(c, "label", v), v == case).on_hover_text(s(c, "title", "")).clicked() {
                    e.set("textCase", json!(v));
                }
            }
        });
    });

    group(ui, "Color", true, |ui| color_row(ui, e, "Color del texto", "color", s(t, "color", "#ffffff")));

    group(ui, "Carácter y línea", true, |ui| {
        text_range(ui, e, t, "Carácter", "letterSpacing", -20.0, 60.0, 1.0, 0.01, "");
        text_range(ui, e, t, "Línea", "lineSpacing", 70.0, 220.0, 1.0, 0.01, "");
    });

    group(ui, "Alineación", true, |ui| {
        ui.horizontal_wrapped(|ui| {
            let align = s(t, "align", "center");
            for a in cat::text_aligns() {
                let v = s(a, "value", "");
                if chip(ui, s(a, "label", v), v == align).clicked() {
                    e.set("align", json!(v));
                }
            }
        });
    });

    group(ui, "Estilo preestablecido", true, |ui| {
        if secondary_button(ui, "⊘ Sin estilo").on_hover_text(crate::i18n::tr("Sin estilo (restablecer color, trazo y fondo)")).clicked() {
            let d = defaults();
            for k in ["color", "strokeColor", "strokeWidth", "background", "backgroundBorderWidth", "glow", "shadow"] {
                e.set(k, d[k].clone());
            }
            e.toast = Some("Estilo de texto restablecido".into());
        }
        ui.add_space(4.0);
        style_grid(ui, e, cat::text_styles(), "Estilo");
    });

    group(ui, "Transformación", true, |ui| {
        if secondary_button(ui, "↺ Restablecer transformación").clicked() {
            e.clip.insert("scale".into(), json!(1.0));
            e.clip.insert("rotate".into(), json!(0.0));
            e.clip.insert("x".into(), json!(0.0));
            e.clip.insert("y".into(), json!(0.0));
            e.set("x", json!(0));
            e.set("y", json!(0));
        }
        if let Some(v) = range(ui, e, "Escala", clip.scale, 10.0, 400.0, 1.0, 0.01, "%") {
            e.clip.insert("scale".into(), json!(v));
        }
        text_range(ui, e, t, "Posición X", "x", -100.0, 100.0, 1.0, 1.0, "%");
        text_range(ui, e, t, "Posición Y", "y", -100.0, 100.0, 1.0, 1.0, "%");
        if let Some(v) = range(ui, e, "Rotación", clip.rotate, -180.0, 180.0, 1.0, 1.0, "°") {
            e.clip.insert("rotate".into(), json!(v));
        }
    });

    group(ui, "Mezcla", true, |ui| {
        text_range(ui, e, t, "Opacidad", "opacity", 0.0, 100.0, 1.0, 0.01, "%");
    });

    group(ui, "Trazo", false, |ui| {
        color_row(ui, e, "Color", "strokeColor", s(t, "strokeColor", "#000000"));
        text_range(ui, e, t, "Grosor", "strokeWidth", 0.0, 25.0, 1.0, 0.01, "");
        let mode = s(t, "strokeMode", "outer");
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Posición")).size(11.5).color(theme::MUTED));
            for (v, l) in [("outer", "Fuera"), ("center", "Centro"), ("inside", "Dentro")] {
                if chip(ui, l, mode == v).clicked() {
                    e.set("strokeMode", json!(v));
                }
            }
        });
    });

    group(ui, "Fondo", false, |ui| {
        let bg = s(t, "background", "transparent");
        let has_bg = bg != "transparent" && !bg.is_empty();
        ui.horizontal(|ui| {
            let mut c = if has_bg { paint::hex(bg) } else { Color32::BLACK };
            if ui.color_edit_button_srgba(&mut c).changed() {
                e.set("background", json!(to_hex(c)));
            }
            if secondary_button(ui, if has_bg { "Quitar fondo" } else { "Transparente" }).clicked() {
                e.set("background", json!("transparent"));
            }
        });
        text_range(ui, e, t, "Opacidad", "backgroundAlpha", 0.0, 100.0, 1.0, 0.01, "%");
        text_range(ui, e, t, "Radio", "backgroundRadius", 0.0, 50.0, 1.0, 0.01, "");
        text_range(ui, e, t, "Margen", "backgroundPadding", 0.0, 60.0, 1.0, 0.01, "");
        let style = s(t, "backgroundStyle", "rounded");
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Estilo")).size(11.5).color(theme::MUTED));
            for (v, l) in [("rounded", "Redondeado"), ("square", "Recto"), ("pill", "Píldora")] {
                if chip(ui, l, style == v).clicked() {
                    e.set("backgroundStyle", json!(v));
                }
            }
        });
        color_row(ui, e, "Borde", "backgroundBorderColor", s(t, "backgroundBorderColor", "#000000"));
        text_range(ui, e, t, "Grosor del borde", "backgroundBorderWidth", 0.0, 5.0, 1.0, 0.01, "");
    });

    group(ui, "Brillo", false, |ui| {
        let on = b(t, "glow", false);
        if switch_row(ui, "Resplandor", on) {
            e.set("glow", json!(!on));
        }
        color_row(ui, e, "Color", "glowColor", s(t, "glowColor", "#00e5d0"));
        text_range(ui, e, t, "Intensidad", "glowBlur", 0.0, 120.0, 1.0, 0.01, "");
    });

    group(ui, "Sombra", false, |ui| {
        let on = b(t, "shadow", true);
        if switch_row(ui, "Sombra", on) {
            e.set("shadow", json!(!on));
        }
        color_row(ui, e, "Color", "shadowColor", s(t, "shadowColor", "#000000"));
        text_range(ui, e, t, "Distancia", "shadowDistance", 0.0, 30.0, 1.0, 0.01, "");
        text_range(ui, e, t, "Ángulo", "shadowAngle", -180.0, 180.0, 1.0, 1.0, "°");
        text_range(ui, e, t, "Desenfoque", "shadowBlur", 0.0, 80.0, 1.0, 0.01, "");
        text_range(ui, e, t, "Opacidad", "shadowAlpha", 0.0, 100.0, 1.0, 0.01, "%");
    });

    group(ui, "Curva", false, |ui| {
        text_range(ui, e, t, "Curva", "curve", -100.0, 100.0, 1.0, 1.0, "");
        hint(ui, "Valores positivos curvan el texto hacia arriba; negativos, hacia abajo.");
    });

    group(ui, "Animación", true, |ui| {
        let cur = s(t, "animation", "none").to_string();
        let cur_label = cat::text_animations()
            .iter()
            .find(|a| s(a, "id", "") == cur)
            .map(|a| format!("{} · {}", phase_label(s(a, "phase", "in")), s(a, "label", "")))
            .unwrap_or_else(|| "Sin animación".into());
        egui::ComboBox::from_id_salt("tx-anim").width(ui.available_width() - 8.0).selected_text(cur_label).show_ui(ui, |ui| {
            if ui.selectable_label(cur == "none", crate::i18n::tr("Sin animación")).clicked() {
                e.merge(&json!({ "animation": "none" }));
            }
            for a in cat::text_animations() {
                let aid = s(a, "id", "");
                let l = format!("{} · {}", phase_label(s(a, "phase", "in")), s(a, "label", aid));
                if ui.selectable_label(cur == aid, l).clicked() {
                    e.merge(&crate::inspector::anim_patch(a));
                }
            }
        });
        text_range(ui, e, t, "Duración", "animationDuration", 0.1, 3.0, 0.05, 1.0, "s");
    });

    ui.add_space(6.0);
    if secondary_button(ui, "↺ Restablecer estilo de texto").clicked() {
        let mut d = defaults();
        if let Some(o) = d.as_object_mut() {
            // Se conservan el contenido y la posición.
            o.remove("x");
            o.remove("y");
        }
        e.merge(&d);
        e.toast = Some("Estilo de texto restablecido".into());
    }
}

fn phase_label(p: &str) -> &'static str {
    match p {
        "out" => "Salida",
        "loop" => "Bucle",
        "caption" => "Subtítulos (Pro)",
        _ => "Entrada",
    }
}

/// Cuadrícula de muestras «Aa» (`.tx-style-grid`).
fn style_grid(ui: &mut egui::Ui, e: &mut Edit, list: &[Value], what: &str) {
    let mut clicked: Option<(String, Value)> = None;
    let items: Vec<&Value> = list.iter().collect();
    grid(ui, &items, 74.0, |ui, v, w| {
        let label = s(v, "label", "").to_string();
        let patch = v.get("patch").cloned().unwrap_or(json!({}));
        if text_card(ui, w, &label, "Aa", &patch, false).on_hover_text(&label).clicked() {
            clicked = Some((label, patch));
        }
    });
    if let Some((label, patch)) = clicked {
        e.merge(&patch);
        e.toast = Some(format!("{what} “{label}” aplicado"));
    }
}

fn bubble_tab(ui: &mut egui::Ui, e: &mut Edit) {
    hint(ui, "Elige una burbuja: se aplica al texto seleccionado.");
    ui.add_space(4.0);
    style_grid(ui, e, cat::text_bubbles(), "Burbuja");
    ui.add_space(4.0);
    hint(ui, "También puedes ajustar fondo, borde y radio en Básico → Fondo.");
}

fn effects_tab(ui: &mut egui::Ui, e: &mut Edit) {
    hint(ui, "Efectos que se aplican al texto seleccionado.");
    ui.add_space(4.0);
    style_grid(ui, e, cat::text_effects(), "Efecto");
    section(ui, "Estilo preestablecido");
    style_grid(ui, e, cat::text_styles(), "Estilo");
}

// ---------------- Animación ----------------
fn anim_tab(ui: &mut egui::Ui, e: &mut Edit, t: &Value, group_id: &str) {
    let active = s(t, "animation", "none").to_string();
    if secondary_button(ui, "↺ Sin animación").clicked() {
        e.merge(&json!({ "animation": "none", "animationPhase": "in", "animationDuration": n(t, "animationDuration") }));
        e.toast = Some("Animación eliminada".into());
    }
    section(ui, phase_label(group_id));
    let items: Vec<&Value> = cat::text_animations().iter().filter(|a| s(a, "phase", "in") == group_id).collect();
    let mut clicked: Option<Value> = None;
    let clock = ui.input(|i| i.time);
    grid(ui, &items, 74.0, |ui, a, w| {
        let aid = s(a, "id", "");
        let on = active == aid;
        let (rect, r) = ui.allocate_exact_size(Vec2::new(w, w * 0.72 + 18.0), Sense::click());
        let img = Rect::from_min_size(rect.min, Vec2::new(w, w * 0.72));
        let p = ui.painter();
        p.rect_filled(img, 6.0, theme::PANEL3);
        if on {
            p.rect_stroke(img, 6.0, Stroke::new(2.0, theme::ACCENT), StrokeKind::Inside);
        } else if r.hovered() {
            p.rect_stroke(img, 6.0, Stroke::new(1.0, theme::MUTED2), StrokeKind::Inside);
        }
        // Vista previa en bucle al pasar el ratón (como las miniaturas de CapCut).
        let local = if r.hovered() {
            let d = a.get("duration").and_then(Value::as_f64).unwrap_or(0.6);
            if group_id == "out" { d - (clock % (d + 0.6)).min(d) } else { clock % (d + 0.6) }
        } else if group_id == "out" {
            0.0
        } else {
            10.0
        };
        let probe = json!({ "animation": aid, "content": "Aa" });
        let st = crate::services::raster::text_anim(&probe, local, if group_id == "out" { a.get("duration").and_then(Value::as_f64).unwrap_or(0.6) } else { 30.0 });
        let (alpha, dx, dy, sc) = st.map(|s| (s.alpha, s.dx, s.dy, s.scale)).unwrap_or((1.0, 0.0, 0.0, 1.0));
        let fsz = img.height() * 0.38 * sc;
        let shown = match st {
            Some(s) if s.reveal < 0.999 => &"Aa"[..((2.0 * s.reveal).floor() as usize).min(2)],
            _ => "Aa",
        };
        p.text(
            img.center() + Vec2::new(dx, dy) * fsz,
            egui::Align2::CENTER_CENTER,
            shown,
            egui::FontId::proportional(fsz.max(4.0)),
            paint::with_alpha(Color32::WHITE, alpha.clamp(0.0, 1.0)),
        );
        let font = egui::FontId::proportional(11.0);
        let label = paint::ellipsize(p, s(a, "label", aid), &font, w);
        p.text(egui::pos2(rect.center().x, img.bottom() + 9.0), egui::Align2::CENTER_CENTER, label, font, if on { theme::ACCENT } else { theme::MUTED });
        if r.hovered() {
            ui.ctx().request_repaint();
        }
        if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clicked = Some((*a).clone());
        }
    });
    if let Some(a) = clicked {
        e.merge(&crate::inspector::anim_patch(&a));
        e.toast = Some("Animación de texto aplicada".into());
    }
    ui.add_space(6.0);
    text_range(ui, e, t, "Duración", "animationDuration", 0.1, 3.0, 0.05, 1.0, "s");
}

// ---------------- Seguimiento ----------------
fn tracking_tab(ui: &mut egui::Ui, e: &mut Edit, clip: &Clip) {
    let x = |k: &str, d: f64| clip.extra.get(k).and_then(Value::as_f64).unwrap_or(d);
    let tracking = clip.extra.get("textTracking").and_then(Value::as_str).unwrap_or("none").to_string();
    section(ui, "Tipo de seguimiento");
    ui.horizontal_wrapped(|ui| {
        for (v, l) in [("none", "Ninguno"), ("direction", "Dirección"), ("object", "Seguir objeto")] {
            if chip(ui, l, tracking == v).clicked() {
                e.clip.insert("textTracking".into(), json!(v));
            }
        }
    });
    if tracking != "none" {
        if let Some(v) = range(ui, e, "Dirección", x("textTrackAngle", 0.0).round(), -180.0, 180.0, 1.0, 1.0, "°") {
            e.clip.insert("textTrackAngle".into(), json!(v));
        }
        if let Some(v) = range(ui, e, "Escala", x("textTrackScale", 1.0), 10.0, 400.0, 1.0, 0.01, "%") {
            e.clip.insert("textTrackScale".into(), json!(v));
        }
        if let Some(v) = range(ui, e, "Distancia", x("textTrackDistance", 0.0).round(), -100.0, 100.0, 1.0, 1.0, "%") {
            e.clip.insert("textTrackDistance".into(), json!(v));
        }
    }
    ui.add_space(4.0);
    hint(ui, "El seguimiento automático de CapCut usa la nube. Aquí el texto se coloca y se anima en local con fotogramas clave.");
}

// ---------------- Texto a voz ----------------
fn tts_tab(ui: &mut egui::Ui, app: &mut CupApp, t: &Value) {
    section(ui, "Texto del clip");
    let mut content = s(t, "content", "").to_string();
    ui.add(egui::TextEdit::multiline(&mut content).desired_rows(4).desired_width(f32::INFINITY).interactive(false));
    ui.add_space(4.0);
    let label = if app.tts_busy { "Generando…" } else { "▶ Generar voz con este texto" };
    if primary_button(ui, label).clicked() && !app.tts_busy {
        app.tts_text = content;
        app.generate_tts();
    }
    if secondary_button(ui, "🎤 Abrir Texto a voz completo").clicked() {
        app.left_view = LeftView::Media;
        app.panel_tab = PanelTab::Text;
        app.text_tab = 3;
    }
    ui.add_space(4.0);
    hint(ui, "El panel izquierdo «Texto → Texto a voz» genera audio con Piper en local y con las voces instaladas en Windows.");
}
