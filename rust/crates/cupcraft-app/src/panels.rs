//! Panel de recursos del editor (las 10 pestañas de `panels.js` /
//! `capcut-panels.js`): Archivos, Audio, Texto, Stickers, Efectos,
//! Transiciones, Subtítulos, Filtros, Ajuste y Plantillas. Todo nativo (egui).

use crate::app::{CupApp, Modal, PanelTab};
use crate::format::{format_bytes, format_time};
use crate::icons;
use crate::paint;
use crate::text_styles as cat;
use crate::theme;
use cupcraft_core::{filters, ClipKind, Project, TrackType};
use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Vec2};
use serde_json::{json, Value};
use std::sync::Mutex;

// ----------------------------------------------------------------------
// Catálogos (copiados de la versión web)
// ----------------------------------------------------------------------

pub const STICKER_GROUPS: &[(&str, &[&str])] = &[
    ("Expresiones", &["😀", "😂", "🥹", "😍", "🤩", "😎", "🥳", "😭", "😱", "🤔", "😴", "🤯", "😇", "🫶", "❤️", "💔", "🔥", "✨", "💯", "👍"]),
    ("Naturaleza", &["🌸", "🌺", "🌻", "🍀", "🌴", "⭐", "🌙", "☀️", "🌈", "🦋", "🌊", "❄️", "🍁", "🌿", "🪐"]),
    ("Comida", &["🍕", "🍔", "🌮", "🍣", "🍰", "☕", "🧋", "🍓", "🍉", "🥑"]),
    ("Objetos", &["🎵", "🎶", "🎤", "🎬", "📸", "💎", "👑", "🏆", "🎯", "🎁", "🪄", "💫"]),
    ("Texto/formas", &["💬", "❗", "❓", "⬆️", "⬇️", "➡️", "⭕", "❌", "✅", "🔴", "🟡", "🟢"]),
];

pub const POST_FX: &[(&str, &str, &str)] = &[
    ("radial_blur", "Desenfoque radial", "🌀"),
    ("directional_blur", "Desenfoque lineal", "↔"),
    ("rgb_split", "Separación RGB", "🌈"),
    ("chromatic", "Aberración cromática", "🔴"),
    ("vhs", "VHS", "📼"),
    ("zoom_pulse", "Pulso zoom", "🔎"),
    ("pixelate", "Pixelar", "▦"),
    ("mirror", "Espejo", "🪞"),
    ("vignette", "Viñeta", "⭕"),
    ("grain", "Grano", "🌾"),
    ("posterize", "Posterizar", "🎨"),
    ("wave", "Ola", "〜"),
    ("bloom", "Bloom/brillo", "✨"),
];

pub const CAPCUT_FX: &[(&str, &str, &str)] = &[
    ("bruma_borgona", "Bruma borgoña", "🍷"),
    ("atardecer_ensueno", "Atardecer ensueño", "🌅"),
    ("brillo_brumoso", "Brillo brumoso", "☁"),
    ("pixel_noir", "Pixel Noir", "🎞"),
    ("carino_retro", "Cariño retro", "💌"),
    ("sueno_caqui", "Sueño caqui", "🌿"),
    ("jengibre_suave", "Jengibre suave", "🍊"),
    ("lienzo_embrujado", "Lienzo embrujado", "🎭"),
];

pub const TRANSITION_CATS: &[&str] = &["Todos", "Básicas", "Movimiento", "Zoom", "Cortinilla", "Estilizada", "Impacto", "Elementos", "Tendencia", "Distorsión", "Luz", "Formas", "Drift"];

/// (id, etiqueta, categoría) — idéntico a `TRANSITIONS` de `src/filtros/library.js`.
pub const TRANSITIONS: &[(&str, &str, &str)] = &[
    ("fade", "Fundido", "Básicas"),
    ("dissolve", "Disolver", "Básicas"),
    ("black", "Fundido a negro", "Básicas"),
    ("white", "Fundido a blanco", "Básicas"),
    ("flash", "Flash", "Básicas"),
    ("blur", "Desenfoque", "Básicas"),
    ("slide-left", "Deslizar ←", "Movimiento"),
    ("slide-right", "Deslizar →", "Movimiento"),
    ("slide-up", "Deslizar ↑", "Movimiento"),
    ("slide-down", "Deslizar ↓", "Movimiento"),
    ("push-left", "Empujar ←", "Movimiento"),
    ("push-right", "Empujar →", "Movimiento"),
    ("push-up", "Empujar ↑", "Movimiento"),
    ("push-down", "Empujar ↓", "Movimiento"),
    ("diagonal", "Diagonal", "Movimiento"),
    ("swing", "Balanceo", "Movimiento"),
    ("zoom-in", "Acercar", "Zoom"),
    ("zoom-out", "Alejar", "Zoom"),
    ("spin", "Girar", "Zoom"),
    ("rotate-in", "Rotar y entrar", "Zoom"),
    ("crosszoom", "Zoom cruzado", "Zoom"),
    ("pulse", "Pulso", "Zoom"),
    ("warp", "Deformar", "Zoom"),
    ("dolly", "Dolly", "Zoom"),
    ("wipe-left", "Barrido ←", "Cortinilla"),
    ("wipe-right", "Barrido →", "Cortinilla"),
    ("wipe-up", "Barrido ↑", "Cortinilla"),
    ("wipe-down", "Barrido ↓", "Cortinilla"),
    ("iris", "Iris", "Cortinilla"),
    ("barn", "Puertas", "Cortinilla"),
    ("blinds", "Persianas", "Cortinilla"),
    ("clock", "Reloj", "Cortinilla"),
    ("split", "Diagonal partida", "Cortinilla"),
    ("curtain", "Cortina", "Cortinilla"),
    ("blocks", "Bloques", "Cortinilla"),
    ("glitch", "Glitch", "Estilizada"),
    ("film-burn", "Quemadura", "Estilizada"),
    ("vignette", "Viñeta", "Estilizada"),
    ("grain", "Grano", "Estilizada"),
    ("slam-hit", "Golpe Slam", "Impacto"),
    ("shake-punch", "Sacudida Punch", "Impacto"),
    ("earthquake", "Terremoto", "Impacto"),
    ("fire-burst", "Llamarada Fuego", "Elementos"),
    ("lightning-flash", "Rayo Eléctrico", "Elementos"),
    ("smoke-cloud", "Humo Volumétrico", "Elementos"),
    ("glitch-rgb", "Glitch RGB Split", "Tendencia"),
    ("rgb-pulse", "Pulso Cromático", "Tendencia"),
    ("pixelate", "Mosaico Pixel", "Tendencia"),
    ("whip-left", "Latigazo ←", "Movimiento"),
    ("whip-right", "Latigazo →", "Movimiento"),
    ("zoom-snap", "Zoom Snap Rebote", "Zoom"),
    ("spin-slam", "Giro 360° Golpe", "Zoom"),
    ("swirl-warp", "Vórtice Espiral", "Distorsión"),
    ("kaleido", "Caleidoscopio", "Distorsión"),
    ("lens-flare", "Destello Anamórfico", "Luz"),
    ("strobe", "Estroboscopio", "Luz"),
    ("heart-iris", "Iris Corazón", "Formas"),
    ("star-iris", "Iris Estrella", "Formas"),
    ("page-curl", "Pliegue Página", "Cortinilla"),
    ("drift-voronoi", "Voronoi Shatter", "Drift"),
    ("drift-plasma", "Plasma Burn", "Drift"),
    ("drift-liquid", "Liquid Smudge", "Drift"),
    ("drift-matrix", "Matrix Rain", "Drift"),
    ("drift-ink", "Ink Bleed", "Drift"),
    ("drift-interference", "Analog Interference", "Drift"),
    ("drift-honeycomb", "Honeycomb Hexagon", "Drift"),
    ("drift-bubbles", "Bubbling Dissolve", "Drift"),
    ("drift-vhs", "VHS Scanline", "Drift"),
    ("drift-swirl", "Cross Zoom Swirl", "Drift"),
];

/// Plantillas de `src/editor/templates.js`.
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub ratio: &'static str,
    pub size: f64,
    pub content: &'static str,
    pub subtitle: &'static str,
    pub color: &'static str,
    pub accent: &'static str,
    /// Degradado (150°): tres paradas.
    pub art: [&'static str; 3],
}

pub const TEMPLATES: &[Template] = &[
    Template { id: "travel", name: "Viaje cinematográfico", ratio: "16:9", size: 112.0, content: "EL MUNDO\nES TUYO", subtitle: "Un viaje. Mil recuerdos.", color: "#ffffff", accent: "#d9a441", art: ["#0c4a5e", "#2e9a9a", "#d9a441"] },
    Template { id: "vlog", name: "Vlog vertical", ratio: "9:16", size: 92.0, content: "MI DÍA\nEN 60 SEGUNDOS", subtitle: "Hoy en el canal", color: "#ffffff", accent: "#ffb45e", art: ["#48204f", "#b0477f", "#ffb45e"] },
    Template { id: "promo", name: "Promo de producto", ratio: "1:1", size: 116.0, content: "NUEVO\nDROP", subtitle: "Ya disponible", color: "#00e5d0", accent: "#00e5d0", art: ["#0d2b2b", "#0c5b56", "#00e5d0"] },
    Template { id: "quote", name: "Cita inspiradora", ratio: "4:5", size: 74.0, content: "“La creatividad\nno se detiene.”", subtitle: "cupcraft Studio", color: "#ffffff", accent: "#e5c79a", art: ["#3a2a1c", "#8a6136", "#e5c79a"] },
    Template { id: "gaming", name: "Clip de gaming", ratio: "9:16", size: 108.0, content: "CLUTCH\nMAESTRO", subtitle: "Gameplay highlights", color: "#c6a6ff", accent: "#7b4dff", art: ["#17102f", "#4a26a8", "#c6a6ff"] },
    Template { id: "podcast", name: "Episodio de podcast", ratio: "16:9", size: 86.0, content: "EPISODIO 01", subtitle: "Conversaciones que importan", color: "#ffffff", accent: "#f0a35e", art: ["#2a1a14", "#8a4c2b", "#f0a35e"] },
    Template { id: "fashion", name: "Lookbook de moda", ratio: "4:5", size: 96.0, content: "NUEVA\nCOLECCIÓN", subtitle: "Otoño / Invierno", color: "#f5e9df", accent: "#c9a38d", art: ["#241813", "#6f4a3b", "#c9a38d"] },
    Template { id: "food", name: "Receta rápida", ratio: "9:16", size: 82.0, content: "RECETA\nEN 30 SEG", subtitle: "Fácil y delicioso", color: "#ffe0a3", accent: "#f5b84a", art: ["#3a1d12", "#a84f24", "#f5b84a"] },
    Template { id: "tech", name: "Review de tecnología", ratio: "16:9", size: 90.0, content: "LO PROBAMOS\nTODO", subtitle: "Review sin filtros", color: "#8ffcff", accent: "#00b8c4", art: ["#0c1a1d", "#12535c", "#00b8c4"] },
    Template { id: "fitness", name: "Rutina fitness", ratio: "1:1", size: 104.0, content: "SIN\nEXCUSAS", subtitle: "Rutina de 15 minutos", color: "#d8ff6b", accent: "#a8e22e", art: ["#1a2310", "#4d6b1e", "#d8ff6b"] },
];

// ----------------------------------------------------------------------
// Widgets comunes
// ----------------------------------------------------------------------

pub fn chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let font = egui::FontId::proportional(12.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, theme::TEXT);
    let size = Vec2::new(galley.size().x + 20.0, 24.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = if active {
        theme::ACCENT_SOFT
    } else if resp.hovered() {
        theme::ELEV
    } else {
        theme::PANEL2
    };
    ui.painter().rect_filled(rect, 12.0, fill);
    let color = if active { theme::ACCENT } else if resp.hovered() { theme::TEXT } else { theme::MUTED };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).size(12.0).color(theme::MUTED).strong());
    ui.add_space(2.0);
}

pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(11.5).color(theme::MUTED2));
}

pub fn search_box(ui: &mut egui::Ui, buf: &mut String, hint_text: &str) {
    let w = ui.available_width();
    egui::Frame::NONE
        .fill(theme::FIELD)
        .corner_radius(CornerRadius::from(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::icon(ui, "search", 14.0, theme::MUTED);
                ui.add(
                    egui::TextEdit::singleline(buf)
                        .hint_text(hint_text)
                        .frame(egui::Frame::NONE)
                        .desired_width(w - 44.0),
                );
            });
        });
}

pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(theme::ACCENT_INK).strong().size(12.5))
            .fill(theme::ACCENT)
            .corner_radius(CornerRadius::from(6))
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

pub fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(theme::TEXT).size(12.5))
            .fill(theme::FIELD)
            .corner_radius(CornerRadius::from(6))
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

/// Tarjeta de cuadrícula: devuelve (rect de la imagen, respuesta).
pub(crate) fn card(ui: &mut egui::Ui, w: f32, h: f32, label: &str, active: bool) -> (Rect, egui::Response) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h + 18.0), Sense::click());
    let img = Rect::from_min_size(rect.min, Vec2::new(w, h));
    let p = ui.painter();
    p.rect_filled(img, 6.0, theme::PANEL3);
    if active {
        p.rect_stroke(img, 6.0, Stroke::new(2.0, theme::ACCENT), StrokeKind::Inside);
    } else if resp.hovered() {
        p.rect_stroke(img, 6.0, Stroke::new(1.0, theme::MUTED2), StrokeKind::Inside);
    }
    let font = egui::FontId::proportional(11.0);
    let txt = paint::ellipsize(p, label, &font, w);
    p.text(
        egui::pos2(rect.center().x, img.bottom() + 9.0),
        egui::Align2::CENTER_CENTER,
        txt,
        font,
        if active { theme::ACCENT } else { theme::MUTED },
    );
    (img, resp.on_hover_cursor(egui::CursorIcon::PointingHand))
}

fn grid_cols(ui: &egui::Ui, min_w: f32) -> (usize, f32) {
    let avail = ui.available_width();
    let gap = 8.0;
    let cols = ((avail + gap) / (min_w + gap)).floor().max(1.0) as usize;
    let w = (avail - gap * (cols as f32 - 1.0)) / cols as f32;
    (cols, w.floor())
}

pub(crate) fn grid<T>(ui: &mut egui::Ui, items: &[T], min_w: f32, mut f: impl FnMut(&mut egui::Ui, &T, f32)) {
    let (cols, w) = grid_cols(ui, min_w);
    for row in items.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for it in row {
                f(ui, it, w);
            }
        });
        ui.add_space(6.0);
    }
}

fn selected_clip(app: &CupApp) -> Option<(String, ClipKind)> {
    let id = app.editor.selection()?.to_string();
    let kind = app.editor.project()?.find_clip(&id)?.1.kind;
    Some((id, kind))
}

// ----------------------------------------------------------------------
// Contenedor
// ----------------------------------------------------------------------

/// Tira de pestañas (iconos + etiqueta corta) encima del panel de recursos.
/// Ajusta un texto a `max_w`: reduce la fuente hasta `min_px` (como las
/// media queries de `.tab`) y, si aún no cabe, recorta con "…"
/// (`text-overflow: ellipsis`).
pub fn fit_label(p: &egui::Painter, text: &str, max_w: f32, px: f32, min_px: f32) -> (String, f32) {
    let width = |t: &str, f: f32| p.layout_no_wrap(t.to_string(), egui::FontId::proportional(f), Color32::WHITE).size().x;
    let mut f = px;
    while f > min_px && width(text, f) > max_w {
        f -= 0.5;
    }
    if width(text, f) <= max_w {
        return (text.to_string(), f);
    }
    let chars: Vec<char> = text.chars().collect();
    for n in (1..chars.len()).rev() {
        let t: String = chars[..n].iter().collect::<String>() + "…";
        if width(&t, f) <= max_w {
            return (t, f);
        }
    }
    ("…".into(), f)
}

pub fn tab_strip(ui: &mut egui::Ui, app: &mut CupApp) {
    let n = PanelTab::ALL.len() as f32;
    let w = (ui.available_width() / n).floor().max(30.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for tab in PanelTab::ALL {
            let active = app.panel_tab == tab;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 50.0), Sense::click());
            let p = ui.painter();
            if resp.hovered() && !active {
                p.rect_filled(rect.shrink(2.0), 6.0, theme::PANEL2);
            }
            let c = if active { theme::ACCENT } else if resp.hovered() { theme::TEXT } else { theme::MUTED };
            icons::paint(p, tab.icon(), egui::pos2(rect.center().x, rect.top() + 18.0), 18.0, c);
            let (label, fpx) = fit_label(p, tab.short(), w - 4.0, 10.5, 8.0);
            p.text(
                egui::pos2(rect.center().x, rect.top() + 38.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(fpx),
                c,
            );
            if active {
                p.line_segment(
                    [egui::pos2(rect.left() + 8.0, rect.bottom() - 1.0), egui::pos2(rect.right() - 8.0, rect.bottom() - 1.0)],
                    Stroke::new(2.0, theme::ACCENT),
                );
            }
            if resp.on_hover_text(tab.title()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                app.panel_tab = tab;
            }
        }
    });
}

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    tab_strip(ui, app);
    ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, theme::LINE));
    ui.add_space(6.0);
    // Cabecera: título + ayuda.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(app.panel_tab.title()).size(14.0).color(theme::TEXT).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::icon_button(ui, "info", 15.0, theme::MUTED, "Ayuda").clicked() {
                app.modal = Some(Modal::Help(app.panel_tab));
            }
        });
    });
    ui.add_space(4.0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_width(ui.available_width() - 6.0);
        match app.panel_tab {
            PanelTab::Media => media_panel(ui, app),
            PanelTab::Audio => audio_panel(ui, app),
            PanelTab::Text => text_panel(ui, app),
            PanelTab::Stickers => stickers_panel(ui, app),
            PanelTab::Effects => effects_panel(ui, app),
            PanelTab::Transitions => transitions_panel(ui, app),
            PanelTab::Captions => captions_panel(ui, app),
            PanelTab::Filters => filters_panel(ui, app),
            PanelTab::Adjust => adjust_panel(ui, app),
            PanelTab::Templates => templates_panel(ui, app),
        }
        ui.add_space(12.0);
    });
}

// ----------------------------------------------------------------------
// Archivos multimedia
// ----------------------------------------------------------------------

fn media_thumb(ui: &mut egui::Ui, app: &mut CupApp, m: &cupcraft_core::Media, rect: Rect) {
    let kind = m.kind.as_deref().unwrap_or("");
    let ctx = ui.ctx().clone();
    let key = format!("thumb:{}", m.id);
    if kind == "image" || kind == "video" {
        if app.video.texture(&ctx, &key).is_none() {
            if let Some(path) = app.media_path(m) {
                if kind == "image" {
                    app.video.request_image(&key, &path);
                } else {
                    let (w, h) = crate::services::video::fit_dims(crate::services::media::media_dims(m), 192, 108);
                    let t = m.duration.unwrap_or(1.0).min(1.0) * 0.5;
                    app.video.request_video(&key, &path, t, w, h, false);
                }
            }
        }
    }
    let p = ui.painter_at(rect);
    if let Some((tex, size)) = app.video.texture(&ctx, &key) {
        let s = (rect.width() / size.x).max(rect.height() / size.y);
        let r = Rect::from_center_size(rect.center(), size * s);
        p.image(tex, r, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    } else {
        let icon = match kind {
            "audio" => "music",
            "image" => "image",
            _ => "film",
        };
        icons::paint(&p, icon, rect.center(), 24.0, theme::MUTED);
    }
    if let Some(d) = m.duration.filter(|_| kind != "image") {
        let txt = format_time(d);
        let font = egui::FontId::proportional(10.0);
        let g = p.layout_no_wrap(txt.clone(), font.clone(), Color32::WHITE);
        let br = Rect::from_min_size(
            egui::pos2(rect.right() - g.size().x - 10.0, rect.bottom() - 18.0),
            Vec2::new(g.size().x + 6.0, 14.0),
        );
        p.rect_filled(br, 3.0, Color32::from_black_alpha(170));
        p.text(br.center(), egui::Align2::CENTER_CENTER, txt, font, Color32::WHITE);
    }
}

fn media_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.horizontal(|ui| {
        if primary_button(ui, "+ Importar").clicked() {
            app.import_media_dialog();
        }
        if secondary_button(ui, "Proyecto de CapCut").clicked() {
            app.open_capcut();
        }
    });
    hint(ui, "También puedes arrastrar archivos a la ventana.");
    ui.add_space(6.0);
    search_box(ui, &mut app.media_search, "Buscar en tus medios");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        for (i, l) in ["Todo", "Video", "Imagen", "Audio"].iter().enumerate() {
            if chip(ui, l, app.media_filter == i as u8).clicked() {
                app.media_filter = i as u8;
            }
        }
    });
    if app.pending_jobs > 0 {
        ui.horizontal(|ui| {
            ui.spinner();
            hint(ui, &format!("Procesando {} archivo(s)…", app.pending_jobs));
        });
    }
    ui.add_space(6.0);
    let q = app.media_search.to_lowercase();
    let filt = app.media_filter;
    let list: Vec<cupcraft_core::Media> = app
        .editor
        .project()
        .map(|p| p.media.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|m| {
            let k = m.kind.as_deref().unwrap_or("");
            let ok_kind = match filt {
                1 => k == "video",
                2 => k == "image",
                3 => k == "audio",
                _ => true,
            };
            ok_kind && (q.is_empty() || m.name.as_deref().unwrap_or("").to_lowercase().contains(&q))
        })
        .collect();
    if list.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            icons::icon(ui, "upload", 30.0, theme::MUTED2);
            ui.label(egui::RichText::new("Importa medios para empezar").color(theme::MUTED));
            hint(ui, "Video, imagen o audio desde tu equipo");
        });
        return;
    }
    let mut add: Option<String> = None;
    let mut remove: Option<String> = None;
    grid(ui, &list, 96.0, |ui, m, w| {
        let name = m.name.clone().unwrap_or_else(|| "Medio".into());
        let (img, resp) = card(ui, w, w * 0.62, &name, false);
        media_thumb(ui, app, m, img);
        if resp.hovered() {
            icons::paint(ui.painter(), "plus", egui::pos2(img.right() - 12.0, img.top() + 12.0), 16.0, theme::ACCENT);
        }
        let size = m.extra.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        let resp = resp.on_hover_text(format!("{name}\n{} · clic para añadir a la línea de tiempo", format_bytes(size)));
        if resp.clicked() {
            add = Some(m.id.clone());
        }
        resp.context_menu(|ui| {
            if ui.button("Añadir a la línea de tiempo").clicked() {
                add = Some(m.id.clone());
                ui.close();
            }
            if ui.button("Eliminar del proyecto").clicked() {
                remove = Some(m.id.clone());
                ui.close();
            }
        });
    });
    if let Some(id) = add {
        app.add_media_to_timeline(&id);
    }
    if let Some(id) = remove {
        app.remove_media(&id);
    }
}

// ----------------------------------------------------------------------
// Audio
// ----------------------------------------------------------------------

fn audio_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.horizontal(|ui| {
        if primary_button(ui, "+ Importar audio").clicked() {
            app.import_audio_dialog();
        }
    });
    section(ui, "Grabar voz en off");
    if app.recorder.is_some() {
        let secs = app.recording_secs();
        ui.horizontal(|ui| {
            let t = ui.input(|i| i.time);
            let a = (0.5 + 0.5 * (t * 4.0).sin()) as f32;
            let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 5.0, paint::with_alpha(theme::DANGER, 0.4 + 0.6 * a));
            ui.label(egui::RichText::new(format!("Grabando {}", format_time(secs))).color(theme::TEXT));
            ui.ctx().request_repaint();
        });
        ui.horizontal(|ui| {
            if primary_button(ui, "Detener y añadir").clicked() {
                app.stop_recording(true);
            }
            if secondary_button(ui, "Descartar").clicked() {
                app.stop_recording(false);
            }
        });
    } else {
        ui.horizontal(|ui| {
            if secondary_button(ui, "🎙 Grabar con el micrófono").clicked() {
                app.start_recording();
            }
        });
        hint(ui, "La grabación se inserta en el cursor como un clip de audio.");
    }
    section(ui, "Audio del proyecto");
    search_box(ui, &mut app.audio_search, "Buscar audio");
    ui.add_space(4.0);
    let q = app.audio_search.to_lowercase();
    let list: Vec<cupcraft_core::Media> = app
        .editor
        .project()
        .map(|p| p.media.iter().filter(|m| m.kind.as_deref() == Some("audio")).cloned().collect())
        .unwrap_or_default();
    let list: Vec<_> = list
        .into_iter()
        .filter(|m| q.is_empty() || m.name.as_deref().unwrap_or("").to_lowercase().contains(&q))
        .collect();
    if list.is_empty() {
        hint(ui, "Aún no hay audio. Importa música o efectos de sonido desde tu equipo.");
    }
    let mut add = None;
    let mut remove = None;
    for m in &list {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect, 6.0, if resp.hovered() { theme::ELEV } else { theme::PANEL2 });
        let ir = Rect::from_min_size(rect.min + Vec2::new(6.0, 6.0), Vec2::splat(32.0));
        p.rect_filled(ir, 5.0, theme::ACCENT_SOFT);
        icons::paint(p, "music", ir.center(), 16.0, theme::ACCENT);
        let font = egui::FontId::proportional(12.5);
        let name = paint::ellipsize(p, m.name.as_deref().unwrap_or("Audio"), &font, rect.width() - 110.0);
        p.text(egui::pos2(ir.right() + 8.0, rect.top() + 15.0), egui::Align2::LEFT_CENTER, name, font, theme::TEXT);
        p.text(
            egui::pos2(ir.right() + 8.0, rect.top() + 31.0),
            egui::Align2::LEFT_CENTER,
            format_time(m.duration.unwrap_or(0.0)),
            egui::FontId::proportional(11.0),
            theme::MUTED,
        );
        if resp.hovered() {
            icons::paint(p, "plus", egui::pos2(rect.right() - 18.0, rect.center().y), 16.0, theme::ACCENT);
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.clicked() {
            add = Some(m.id.clone());
        }
        resp.context_menu(|ui| {
            if ui.button("Añadir a la línea de tiempo").clicked() {
                add = Some(m.id.clone());
                ui.close();
            }
            if ui.button("Eliminar del proyecto").clicked() {
                remove = Some(m.id.clone());
                ui.close();
            }
        });
        ui.add_space(4.0);
    }
    if let Some(id) = add {
        app.add_media_to_timeline(&id);
    }
    if let Some(id) = remove {
        app.remove_media(&id);
    }
}

// ----------------------------------------------------------------------
// Texto
// ----------------------------------------------------------------------

static VOICES: Mutex<Option<Value>> = Mutex::new(None);

fn voices(refresh: bool) -> Value {
    let mut g = VOICES.lock().unwrap_or_else(|e| e.into_inner());
    if refresh || g.is_none() {
        *g = Some(cupcraft_tts::list_voices(&CupApp::tts_root()));
    }
    g.clone().unwrap_or(Value::Null)
}

fn apply_text_patch(app: &mut CupApp, label: &str, sample: &str, patch: &Value) {
    if let Some((id, ClipKind::Text)) = selected_clip(app) {
        let p = patch.clone();
        app.mutate_project(|pr| pr.update_clip(&id, json!({ "text": p })));
        app.toast(format!("Estilo aplicado: {label}"));
        return;
    }
    let content = if app.text_content.trim().is_empty() { sample.to_string() } else { app.text_content.clone() };
    let ph = app.playhead;
    let p = patch.clone();
    let id = app.mutate_project(|pr| pr.add_text_clip(&content, Some(ph), 3.0, Some(&p))).flatten();
    if let Some(id) = id {
        app.editor.select(Some(id));
        app.toast(format!("Texto añadido: {label}"));
    }
}

pub(crate) fn text_card(ui: &mut egui::Ui, w: f32, label: &str, sample: &str, patch: &Value, active: bool) -> egui::Response {
    let (img, resp) = card(ui, w, w * 0.62, label, active);
    let p = ui.painter_at(img);
    p.rect_filled(img, 6.0, Color32::from_rgb(0x1c, 0x1c, 0x1c));
    let mut text = json!({ "content": sample, "size": 72, "weight": 800, "color": "#ffffff", "align": "center" });
    if let (Some(t), Some(pm)) = (text.as_object_mut(), patch.as_object()) {
        for (k, v) in pm {
            if k != "x" && k != "y" {
                t.insert(k.clone(), v.clone());
            }
        }
        t.insert("content".into(), json!(sample));
    }
    let size = paint::num_of(&text, "size", 72.0).max(20.0) as f32;
    let scale = (img.height() * 0.42 / size).min(0.5);
    paint::paint_text(&p, img, &text, scale);
    resp
}

/// Avatares locales (`avatarPanel` de panels.js): un emoji sobre un círculo
/// de color, añadido como clip de sticker en el cursor.
fn avatar_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    hint(ui, "Añade un personaje como capa con fondo de color. Después puedes moverlo, escalarlo y combinarlo con tus videos.");
    ui.add_space(6.0);
    let presets = crate::text_styles::avatar_presets();
    let mut pick: Option<(String, String, String)> = None;
    grid(ui, presets, 92.0, |ui, p, w| {
        let emoji = p["emoji"].as_str().unwrap_or("🙂");
        let name = p["name"].as_str().unwrap_or("");
        let color = p["color"].as_str().unwrap_or("#1d6d78");
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, w + 22.0), Sense::click());
        ui.painter().rect_filled(rect, 8.0, if resp.hovered() { theme::ELEV } else { theme::PANEL2 });
        let art = egui::Rect::from_min_size(rect.min + Vec2::new(8.0, 8.0), Vec2::splat(w - 16.0));
        let bg = Color32::from_hex(color).unwrap_or(theme::ACCENT_SOFT);
        ui.painter().circle_filled(art.center(), art.width() / 2.0, bg);
        let sticker = json!({ "content": emoji, "size": 190 });
        paint::paint_text(&ui.painter().with_clip_rect(art), art, &sticker, (art.width() * 0.55) / 190.0);
        ui.painter().text(egui::pos2(rect.center().x, rect.bottom() - 12.0), egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(11.0), theme::TEXT);
        if resp.on_hover_text(format!("Añadir {name}")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            pick = Some((emoji.to_string(), name.to_string(), color.to_string()));
        }
    });
    if let Some((emoji, name, color)) = pick {
        let ph = app.playhead;
        let preset = json!({ "content": emoji, "size": 190, "background": color, "shape": "circle", "color": "#ffffff" });
        let id = app
            .mutate_project(|p| {
                let id = p.add_sticker_clip(&emoji, Some(5.0), Some(ph), Some(preset))?;
                p.update_clip(&id, json!({ "x": -28.0, "y": 24.0, "scale": 1.0 }));
                Some(id)
            })
            .flatten();
        if let Some(id) = id {
            app.editor.select(Some(id));
            app.toast(format!("Avatar “{name}” añadido"));
        }
    }
    ui.add_space(8.0);
    hint(ui, "La generación de avatares animados con IA requiere conexión y no está disponible sin conexión.");
}

fn text_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.horizontal(|ui| {
        for (i, l) in ["Estilos", "Burbujas", "Efectos", "Texto a voz", "Avatares"].iter().enumerate() {
            if chip(ui, l, app.text_tab == i as u8).clicked() {
                app.text_tab = i as u8;
            }
        }
    });
    ui.add_space(6.0);
    if app.text_tab == 3 {
        tts_panel(ui, app);
        return;
    }
    if app.text_tab == 4 {
        avatar_panel(ui, app);
        return;
    }
    let sel_text = matches!(selected_clip(app), Some((_, ClipKind::Text)));
    if app.text_tab == 0 {
        if primary_button(ui, "+ Añadir texto predeterminado").clicked() {
            let content = if app.text_content.trim().is_empty() { "Texto".to_string() } else { app.text_content.clone() };
            let ph = app.playhead;
            if let Some(id) = app.mutate_project(|p| p.add_text_clip(&content, Some(ph), 3.0, None)).flatten() {
                app.editor.select(Some(id));
                app.toast("Texto añadido");
            }
        }
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::multiline(&mut app.text_content)
                .hint_text("Escribe el texto que se insertará (opcional)")
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
    }
    hint(
        ui,
        if sel_text {
            "Se aplicará al texto seleccionado."
        } else {
            "Haz clic en un estilo para insertarlo en el cursor."
        },
    );
    ui.add_space(4.0);
    search_box(ui, &mut app.text_search, "Buscar estilos");
    ui.add_space(6.0);
    let q = app.text_search.to_lowercase();
    let list: Vec<&Value> = match app.text_tab {
        0 => cat::text_styles().iter().collect(),
        1 => cat::text_bubbles().iter().collect(),
        _ => cat::text_effects().iter().collect(),
    };
    let list: Vec<&Value> = list
        .into_iter()
        .filter(|v| q.is_empty() || paint::str_of(v, "label", "").to_lowercase().contains(&q))
        .collect();
    let mut clicked: Option<(String, String, Value, String)> = None;
    let sel_style = app.selected_text_style.clone();
    grid(ui, &list, 92.0, |ui, v, w| {
        let id = paint::str_of(v, "id", "").to_string();
        let label = paint::str_of(v, "label", "").to_string();
        let sample = paint::str_of(v, "sample", "Aa").to_string();
        let patch = v.get("patch").cloned().unwrap_or(json!({}));
        let resp = text_card(ui, w, &label, &sample, &patch, sel_style.as_deref() == Some(id.as_str()));
        if resp.clicked() {
            clicked = Some((label, sample, patch, id));
        }
    });
    if let Some((label, sample, patch, id)) = clicked {
        app.selected_text_style = Some(id);
        apply_text_patch(app, &label, &sample, &patch);
    }

    // Tipografía del texto seleccionado.
    if let Some((id, ClipKind::Text)) = selected_clip(app) {
        let text = app.editor.project().and_then(|p| p.find_clip(&id)).and_then(|(_, c)| c.text.clone()).unwrap_or(json!({}));
        section(ui, "Tipografía");
        let mut patch: Option<Value> = None;
        let font = paint::str_of(&text, "font", "Inter").to_string();
        egui::ComboBox::from_label("Fuente").selected_text(&font).show_ui(ui, |ui| {
            for f in cat::text_fonts() {
                let v = paint::str_of(f, "value", "");
                if ui.selectable_label(v == font, paint::str_of(f, "label", v)).clicked() {
                    patch = Some(json!({ "font": v }));
                }
            }
        });
        let weight = paint::num_of(&text, "weight", 800.0) as i64;
        egui::ComboBox::from_label("Grosor").selected_text(weight.to_string()).show_ui(ui, |ui| {
            for f in cat::text_weights() {
                let v = f.get("value").and_then(|v| v.as_i64()).unwrap_or(400);
                if ui.selectable_label(v == weight, paint::str_of(f, "label", "")).clicked() {
                    patch = Some(json!({ "weight": v }));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            let case = paint::str_of(&text, "textCase", "none").to_string();
            for f in cat::text_cases() {
                let v = paint::str_of(f, "value", "");
                if chip(ui, paint::str_of(f, "label", v), v == case).on_hover_text(paint::str_of(f, "title", "")).clicked() {
                    patch = Some(json!({ "textCase": v }));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            let align = paint::str_of(&text, "align", "center").to_string();
            for f in cat::text_aligns() {
                let v = paint::str_of(f, "value", "");
                if chip(ui, paint::str_of(f, "label", v), v == align).clicked() {
                    patch = Some(json!({ "align": v }));
                }
            }
        });
        section(ui, "Animación");
        ui.horizontal_wrapped(|ui| {
            let cur = paint::str_of(&text, "animation", "none").to_string();
            if chip(ui, "Sin animación", cur == "none").clicked() {
                patch = Some(json!({ "animation": "none" }));
            }
            for a in cat::text_animations() {
                let aid = paint::str_of(a, "id", "");
                let phase = paint::str_of(a, "phase", "in");
                let group = match phase {
                    "out" => "Salida",
                    "loop" => "Bucle",
                    "caption" => "Subtítulos",
                    _ => "Entrada",
                };
                if chip(ui, paint::str_of(a, "label", aid), cur == aid).on_hover_text(group).clicked() {
                    patch = Some(crate::inspector::anim_patch(a));
                }
            }
        });
        if let Some(pt) = patch {
            app.mutate_project(|p| p.update_clip(&id, json!({ "text": pt })));
        }
    }
}

fn tts_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    let v = voices(false);
    let available = v.get("available").and_then(|b| b.as_bool()).unwrap_or(false);
    let list: Vec<Value> = v.get("voices").and_then(|a| a.as_array()).cloned().unwrap_or_default();
    if !available || list.is_empty() {
        egui::Frame::NONE.fill(theme::PANEL2).corner_radius(CornerRadius::from(6)).inner_margin(10).show(ui, |ui| {
            ui.label(egui::RichText::new("Texto a voz sin conexión").color(theme::TEXT).strong());
            hint(
                ui,
                "Se usa Piper (motor local). Coloca piper.exe y las voces .onnx en la carpeta tools\\piper junto a cupcraft.exe. No se envía nada a internet.",
            );
            if secondary_button(ui, "Volver a buscar voces").clicked() {
                voices(true);
            }
        });
        return;
    }
    if app.tts_voice.is_empty() {
        app.tts_voice = list[0].get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
    }
    section(ui, "Voz");
    let cur_name = list
        .iter()
        .find(|x| paint::str_of(x, "id", "") == app.tts_voice)
        .map(|x| paint::str_of(x, "name", "").to_string())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt("tts-voice").width(ui.available_width() - 8.0).selected_text(cur_name).show_ui(ui, |ui| {
        for x in &list {
            let id = paint::str_of(x, "id", "");
            let label = format!("{} · {}", paint::str_of(x, "name", id), paint::str_of(x, "lang", ""));
            if ui.selectable_label(id == app.tts_voice, label).on_hover_text(paint::str_of(x, "description", "")).clicked() {
                app.tts_voice = id.to_string();
            }
        }
    });
    section(ui, "Texto");
    ui.add(egui::TextEdit::multiline(&mut app.tts_text).hint_text("Escribe lo que quieres que se lea").desired_rows(4).desired_width(f32::INFINITY));
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Velocidad").color(theme::MUTED));
        ui.add(egui::Slider::new(&mut app.tts_speed, 0.5..=2.0).step_by(0.05).suffix("×"));
    });
    ui.add_space(6.0);
    ui.add_enabled_ui(!app.tts_busy && !app.tts_text.trim().is_empty(), |ui| {
        if primary_button(ui, if app.tts_busy { "Generando…" } else { "Generar voz y añadir" }).clicked() {
            app.generate_tts();
        }
    });
    if app.tts_busy {
        ui.spinner();
    }
}

// ----------------------------------------------------------------------
// Stickers
// ----------------------------------------------------------------------

fn stickers_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    search_box(ui, &mut app.sticker_search, "Buscar categorías de stickers");
    ui.add_space(6.0);
    let q = app.sticker_search.to_lowercase();
    ui.horizontal_wrapped(|ui| {
        for (i, (name, _)) in STICKER_GROUPS.iter().enumerate() {
            if !q.is_empty() && !name.to_lowercase().contains(&q) {
                continue;
            }
            if chip(ui, name, app.sticker_group == i).clicked() {
                app.sticker_group = i;
            }
        }
    });
    ui.add_space(8.0);
    let (_, list) = STICKER_GROUPS[app.sticker_group.min(STICKER_GROUPS.len() - 1)];
    let mut pick = None;
    grid(ui, list, 52.0, |ui, e, w| {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(w), Sense::click());
        ui.painter().rect_filled(rect, 6.0, if resp.hovered() { theme::ELEV } else { theme::PANEL2 });
        let sticker = json!({ "content": e, "size": 160 });
        paint::paint_text(ui.painter(), rect, &sticker, (w * 0.55) / 160.0);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            pick = Some(e.to_string());
        }
    });
    if let Some(e) = pick {
        let ph = app.playhead;
        if let Some(id) = app.mutate_project(|p| p.add_sticker_clip(&e, Some(3.0), Some(ph), None)).flatten() {
            app.editor.select(Some(id));
            app.toast(format!("Sticker añadido {e}"));
        }
    }
}

// ----------------------------------------------------------------------
// Efectos
// ----------------------------------------------------------------------

fn effects_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.horizontal(|ui| {
        if chip(ui, "Efectos de video", app.effect_group == 0).clicked() {
            app.effect_group = 0;
        }
        if chip(ui, "Estilo CapCut", app.effect_group == 1).clicked() {
            app.effect_group = 1;
        }
    });
    ui.add_space(6.0);
    search_box(ui, &mut app.effect_search, "Buscar efectos");
    ui.add_space(6.0);
    hint(ui, "El efecto se añade como capa en el cursor y afecta a todo lo que hay debajo.");
    ui.add_space(4.0);
    let q = app.effect_search.to_lowercase();
    let src = if app.effect_group == 0 { POST_FX } else { CAPCUT_FX };
    let list: Vec<&(&str, &str, &str)> = src.iter().filter(|(_, l, _)| q.is_empty() || l.to_lowercase().contains(&q)).collect();
    let mut pick = None;
    grid(ui, &list, 92.0, |ui, (id, label, icon), w| {
        let (img, resp) = card(ui, w, w * 0.62, label, false);
        let p = ui.painter_at(img);
        paint::rounded_gradient(&p, img, 6, Color32::from_rgb(0x1f, 0x2b, 0x2e), Color32::from_rgb(0x14, 0x18, 0x1a));
        p.text(img.center(), egui::Align2::CENTER_CENTER, *icon, egui::FontId::proportional(24.0), Color32::WHITE);
        if resp.clicked() {
            pick = Some((id.to_string(), label.to_string()));
        }
    });
    if let Some((id, label)) = pick {
        let ph = app.playhead;
        if let Some(cid) = app.mutate_project(|p| p.add_effect_clip(&id, &label, Some(4.0), Some(ph), Some(1.0))).flatten() {
            app.editor.select(Some(cid));
        }
        app.toast(format!("Efecto añadido: {label}"));
    }
}

// ----------------------------------------------------------------------
// Transiciones
// ----------------------------------------------------------------------

/// `TRANSITION_FADES` de la web: solo las básicas llevan fundidos.
pub(crate) fn transition_fades(id: &str) -> (f64, f64) {
    match id {
        "fade" => (0.6, 0.6),
        "dissolve" => (0.8, 0.4),
        "black" => (0.9, 0.9),
        "white" => (0.7, 0.7),
        "flash" => (0.25, 0.25),
        _ => (0.0, 0.0),
    }
}

/// Parche para poner la transición `tid` (o "none") de `d` segundos en `clip`.
/// La transición es de ENTRADA: solo toca el fundido de entrada (que dura lo
/// mismo que la transición) y retira los fundidos que dejó la transición
/// anterior, sin pisar los que el usuario puso a mano.
pub(crate) fn transition_patch(clip: &cupcraft_core::Clip, tid: &str, d: f64) -> Value {
    let old = clip.extra.get("transition").and_then(Value::as_str).unwrap_or("none");
    let old_d = clip.extra.get("transitionDuration").and_then(Value::as_f64).unwrap_or(0.6);
    let (ofi, ofo) = transition_fades(old);
    let near = |a: f64, b: f64| (a - b).abs() < 0.011;
    let from_old_in = ofi > 0.0 && (near(clip.fade_in, ofi) || near(clip.fade_in, old_d));
    let (fi, _) = transition_fades(tid);
    let fade_in = if fi > 0.0 { d } else if from_old_in { 0.0 } else { clip.fade_in };
    let fade_out = if ofo > 0.0 && near(clip.fade_out, ofo) { 0.0 } else { clip.fade_out };
    let mut patch = json!({ "transition": tid, "transitionDuration": d, "fadeIn": fade_in, "fadeOut": fade_out });
    if tid == "none" {
        patch["transitionLead"] = json!(0.0);
    }
    patch
}

/// Última transición elegida (para «Aplicar a todos» sin clip seleccionado).
static LAST_TRANSITION: Mutex<String> = Mutex::new(String::new());

fn transitions_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    search_box(ui, &mut app.transition_search, "Buscar transiciones");
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for c in TRANSITION_CATS {
            if chip(ui, c, app.transition_category == *c).clicked() {
                app.transition_category = c.to_string();
            }
        }
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Duración").color(theme::MUTED));
        ui.add(egui::Slider::new(&mut app.transition_duration, 0.3..=2.0).step_by(0.1).suffix(" s"));
    });
    let sel = selected_clip(app);
    let current = sel
        .as_ref()
        .and_then(|(id, _)| app.editor.project()?.find_clip(id).map(|(_, c)| c.extra.get("transition").and_then(|v| v.as_str()).unwrap_or("").to_string()))
        .unwrap_or_default();
    hint(
        ui,
        if sel.is_some() {
            "Se aplica en el corte, entre el clip anterior y el seleccionado. También puedes pulsar «+» en el corte de la línea de tiempo."
        } else {
            "Selecciona un clip en la línea de tiempo para aplicar una transición."
        },
    );
    ui.add_space(4.0);
    let q = app.transition_search.to_lowercase();
    let list: Vec<&(&str, &str, &str)> = TRANSITIONS
        .iter()
        .filter(|(_, l, c)| if q.is_empty() { app.transition_category == "Todos" || *c == app.transition_category } else { l.to_lowercase().contains(&q) })
        .collect();
    let mut pick = None;
    grid(ui, &list, 80.0, |ui, (id, label, _), w| {
        let (img, resp) = card(ui, w, w * 0.62, label, current == *id);
        let p = ui.painter_at(img);
        // Miniatura: dos bloques que se cruzan.
        let half = Rect::from_min_max(img.min, egui::pos2(img.center().x, img.max.y));
        p.rect_filled(half.shrink(6.0), 4.0, Color32::from_rgb(0x2e, 0x55, 0x5a));
        let half2 = Rect::from_min_max(egui::pos2(img.center().x, img.min.y), img.max);
        p.rect_filled(half2.shrink(6.0), 4.0, Color32::from_rgb(0x55, 0x3a, 0x5e));
        icons::paint(&p, "transition", img.center(), 16.0, Color32::WHITE);
        // Al pasar el cursor: la transición real animada en la propia tarjeta.
        if resp.hovered() {
            if let Some(tex) = crate::trans_preview::frame(ui.ctx(), id, app.transition_duration as f64) {
                let r = img.shrink(1.0);
                p.rect_filled(r, 4.0, Color32::BLACK);
                let fit = (r.width() / 16.0).min(r.height() / 9.0);
                let dst = Rect::from_center_size(r.center(), Vec2::new(16.0 * fit, 9.0 * fit));
                p.image(tex, dst, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
        }
        if resp.clicked() {
            pick = Some((id.to_string(), label.to_string()));
        }
    });
    if let Some((tid, _)) = &pick {
        if let Ok(mut last) = LAST_TRANSITION.lock() {
            *last = tid.clone();
        }
    }
    if let Some((tid, label)) = pick {
        match sel {
            Some((cid, _)) => {
                let d = app.transition_duration as f64;
                app.mutate_project(|p| {
                    if let Some(patch) = p.find_clip(&cid).map(|(_, c)| transition_patch(c, &tid, d)) {
                        p.update_clip(&cid, patch);
                    }
                });
                app.toast(format!("Transición aplicada: {label}"));
            }
            None => app.toast("Selecciona un clip para aplicar la transición"),
        }
    }
    // «Aplicar a todos»: la transición del clip seleccionado (o la última
    // elegida) a todos los clips de vídeo e imagen del proyecto.
    let all_tid = if !current.is_empty() && current != "none" {
        current.clone()
    } else {
        LAST_TRANSITION.lock().map(|s| s.clone()).unwrap_or_default()
    };
    if !all_tid.is_empty() {
        ui.add_space(6.0);
        let label = TRANSITIONS.iter().find(|(i, _, _)| *i == all_tid).map(|(_, l, _)| *l).unwrap_or("");
        if primary_button(ui, &format!("Aplicar a todos · {label}")).clicked() {
            let d = app.transition_duration as f64;
            let tid = all_tid.clone();
            let n = app
                .mutate_project(|p| {
                    let ids: Vec<String> = p
                        .tracks
                        .iter()
                        .flat_map(|t| t.clips.iter())
                        .filter(|c| matches!(c.kind, ClipKind::Video | ClipKind::Image))
                        .map(|c| c.id.clone())
                        .collect();
                    for id in &ids {
                        if let Some(patch) = p.find_clip(id).map(|(_, c)| transition_patch(c, &tid, d)) {
                            p.update_clip(id, patch);
                        }
                    }
                    ids.len()
                })
                .unwrap_or(0);
            app.toast(format!("Transición «{label}» aplicada a {n} clips"));
        }
    }
    if let Some((cid, _)) = selected_clip(app) {
        if !current.is_empty() && current != "none" {
            ui.add_space(4.0);
            if secondary_button(ui, "Quitar transición").clicked() {
                app.mutate_project(|p| {
                    let patch = p.find_clip(&cid).map(|(_, c)| {
                        let d = c.extra.get("transitionDuration").and_then(Value::as_f64).unwrap_or(0.6);
                        transition_patch(c, "none", d)
                    });
                    if let Some(patch) = patch {
                        p.update_clip(&cid, patch);
                    }
                });
                app.toast("Transición eliminada");
            }
        }
    }
}

// ----------------------------------------------------------------------
// Subtítulos
// ----------------------------------------------------------------------

fn caption_patch(style: &Value) -> Value {
    let mut patch = json!({
        "size": style.get("size").cloned().unwrap_or(json!(56)),
        "weight": style.get("weight").cloned().unwrap_or(json!(600)),
        "color": style.get("color").cloned().unwrap_or(json!("#ffffff")),
        "background": style.get("background").cloned().unwrap_or(json!("transparent")),
        "shadow": style.get("shadow").cloned().unwrap_or(json!(true)),
        "y": 34,
    });
    let hl = paint::str_of(style, "highlight", "");
    if !hl.is_empty() {
        patch["highlight"] = json!(hl);
    }
    patch
}

pub fn apply_captions(app: &mut CupApp, items: Vec<Value>, replace: bool) {
    let style = cat::caption_styles()
        .iter()
        .find(|s| paint::str_of(s, "id", "") == app.caption_style)
        .cloned()
        .unwrap_or(json!({}));
    let patch = caption_patch(&style);
    let sid = app.caption_style.clone();
    let n = app
        .mutate_project(|p| {
            let ids = p.add_subtitle_clips(&items, Some(&sid), replace);
            for id in &ids {
                p.update_clip(id, json!({ "text": patch.clone() }));
            }
            ids.len()
        })
        .unwrap_or(0);
    if n > 0 {
        app.toast(format!("{n} subtítulo(s) añadido(s)"));
    } else {
        app.toast("No hay texto para crear subtítulos");
    }
}

fn parse_srt_time(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    Some(parts[0].parse::<f64>().ok()? * 3600.0 + parts[1].parse::<f64>().ok()? * 60.0 + parts[2].parse::<f64>().ok()?)
}

/// Lector SRT/VTT mínimo.
pub fn parse_srt(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let norm = text.replace("\r\n", "\n");
    for block in norm.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        let Some(ti) = lines.iter().position(|l| l.contains("-->")) else { continue };
        let (a, b) = lines[ti].split_once("-->").unwrap_or(("", ""));
        let b = b.split_whitespace().next().unwrap_or("");
        let (Some(start), Some(end)) = (parse_srt_time(a), parse_srt_time(b)) else { continue };
        let body = lines[ti + 1..].join("\n");
        if body.trim().is_empty() {
            continue;
        }
        out.push(json!({ "text": body, "start": start, "end": end }));
    }
    out
}

fn captions_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    section(ui, "Estilo");
    ui.horizontal_wrapped(|ui| {
        for s in cat::caption_styles() {
            let id = paint::str_of(s, "id", "");
            if chip(ui, paint::str_of(s, "label", id), app.caption_style == id).clicked() {
                app.caption_style = id.to_string();
            }
        }
    });
    section(ui, "Subtítulos manuales");
    hint(ui, "Una línea por subtítulo. Se colocan uno tras otro desde el cursor.");
    ui.add(egui::TextEdit::multiline(&mut app.caption_text).hint_text("Escribe tus subtítulos…").desired_rows(5).desired_width(f32::INFINITY));
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Duración de cada línea").color(theme::MUTED));
        ui.add(egui::Slider::new(&mut app.caption_duration, 0.5..=8.0).step_by(0.1).suffix(" s"));
    });
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if primary_button(ui, "Añadir subtítulos").clicked() {
            let mut t = app.playhead;
            let d = app.caption_duration;
            let items: Vec<Value> = app
                .caption_text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(|l| {
                    let v = json!({ "text": l, "start": t, "duration": d });
                    t += d;
                    v
                })
                .collect();
            apply_captions(app, items, false);
        }
        if secondary_button(ui, "Importar SRT/VTT").clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("Subtítulos", &["srt", "vtt"]).pick_file() {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let items = parse_srt(&text);
                        apply_captions(app, items, false);
                    }
                    Err(e) => app.toast_error(format!("No se pudo leer el archivo: {e}")),
                }
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if secondary_button(ui, "Quitar todos los subtítulos").clicked() {
            let n = app.mutate_project(|p| p.clear_subtitle_clips()).unwrap_or(0);
            app.toast(format!("{n} subtítulo(s) eliminado(s)"));
        }
    });
    section(ui, "Subtítulos automáticos");
    if let Some((msg, p)) = app.whisper_busy.clone() {
        hint(ui, &msg);
        if p >= 0.0 {
            ui.add(egui::ProgressBar::new(p).show_percentage());
        } else {
            ui.add(egui::Spinner::new());
        }
    } else if !crate::services::whisper::ready() {
        hint(ui, "Transcribe la voz del clip a subtítulos, 100 % en tu PC (Whisper). La primera vez hay que descargar el motor y el modelo (~80 MB).");
        if primary_button(ui, "Descargar motor y modelo").clicked() {
            app.install_whisper();
        }
    } else {
        hint(ui, "Selecciona un clip de vídeo o audio en la línea de tiempo.");
        ui.horizontal(|ui| {
            ui.label("Idioma");
            let langs = [("auto", "Detectar"), ("es", "Español"), ("en", "Inglés"), ("fr", "Francés"), ("de", "Alemán"), ("it", "Italiano"), ("pt", "Portugués")];
            let cur = langs.iter().find(|l| l.0 == app.whisper_lang).map(|l| l.1).unwrap_or("Detectar");
            egui::ComboBox::from_id_salt("whisper-lang").selected_text(cur).show_ui(ui, |ui| {
                for (id, label) in langs {
                    ui.selectable_value(&mut app.whisper_lang, id.to_string(), label);
                }
            });
        });
        let sel = selected_clip(app).filter(|(_, k)| matches!(k, ClipKind::Video | ClipKind::Audio));
        ui.add_enabled_ui(sel.is_some(), |ui| {
            if primary_button(ui, "Transcribir clip seleccionado").clicked() {
                if let Some((id, _)) = &sel {
                    app.transcribe_selected(id);
                }
            }
        });
    }
}

// ----------------------------------------------------------------------
// Filtros
// ----------------------------------------------------------------------

fn filters_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    let sel = selected_clip(app).filter(|(_, k)| matches!(k, ClipKind::Video | ClipKind::Image));
    hint(
        ui,
        if sel.is_none() {
            "Selecciona un clip de video o imagen para aplicarle un filtro."
        } else {
            "Elige un filtro de la línea cupcraft para aplicarlo al clip seleccionado."
        },
    );
    ui.add_space(4.0);
    search_box(ui, &mut app.filter_query, "Buscar filtros");
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if chip(ui, "Todos", app.filter_category.is_none()).clicked() {
            app.filter_category = None;
        }
        for &c in filters::CATEGORIES.iter() {
            if chip(ui, c, app.filter_category.as_deref() == Some(c)).clicked() {
                app.filter_category = Some(c.to_string());
            }
        }
    });
    ui.add_space(6.0);
    let current = sel.as_ref().and_then(|(id, _)| {
        let c = app.editor.project()?.find_clip(id)?.1;
        Some(filters::line_key_for_clip(&c.filter, c.gl_filter.as_deref(), c.lut.as_deref())?.to_string())
    });
    let q = app.filter_query.trim().to_string();
    let list = filters::search(app.filter_category.as_deref(), if q.is_empty() { None } else { Some(q.as_str()) });
    let mut pick: Option<(String, String)> = None;
    grid(ui, &list, 74.0, |ui, f, w| {
        let (img, resp) = card(ui, w, w * 0.81, f.label, current.as_deref() == Some(f.key));
        let fx = paint::parse_css(f.css);
        let p = ui.painter_at(img.shrink(1.0));
        paint::scene(&p, img.shrink(1.0), &|c| paint::apply_fx(c, &fx));
        if current.as_deref() == Some(f.key) {
            ui.painter().rect_stroke(img, 6.0, Stroke::new(2.0, theme::ACCENT), StrokeKind::Inside);
        }
        if resp.clicked() {
            pick = Some((f.key.to_string(), f.label.to_string()));
        }
    });
    if let Some((key, label)) = pick {
        match &sel {
            Some((id, _)) => {
                let id = id.clone();
                if app.mutate_project(|p| p.apply_filter(&id, &key)).unwrap_or(false) {
                    app.toast(format!("Filtro aplicado: {label}"));
                }
            }
            None => app.toast("Selecciona un clip de video o imagen"),
        }
    }
    if let Some((id, _)) = sel {
        ui.add_space(4.0);
        if secondary_button(ui, "Quitar filtro").clicked() {
            app.mutate_project(|p| p.apply_filter(&id, "none"));
            app.toast("Filtro eliminado");
        }
    }
}

// ----------------------------------------------------------------------
// Ajuste
// ----------------------------------------------------------------------

fn adjust_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    let sel = selected_clip(app).filter(|(_, k)| matches!(k, ClipKind::Video | ClipKind::Image));
    ui.horizontal_wrapped(|ui| {
        for c in cat::adjust_categories() {
            let c = c.as_str().unwrap_or("");
            if chip(ui, c, app.adjust_cat == c).clicked() {
                app.adjust_cat = c.to_string();
            }
        }
    });
    ui.add_space(6.0);
    let Some((id, _)) = sel else {
        hint(ui, "Selecciona un clip de video o imagen para ajustarlo.");
        return;
    };
    // Presets
    let presets: Vec<Value> = if app.adjust_cat == "Tuyos" {
        let mut v = vec![cat::adjust_presets().first().cloned().unwrap_or(json!({}))];
        v.extend(app.custom_adjusts.iter().cloned());
        v
    } else {
        cat::adjust_presets().iter().filter(|p| paint::str_of(p, "category", "") == app.adjust_cat).cloned().collect()
    };
    let mut pick: Option<(Value, String)> = None;
    let mut save = false;
    let mut delete: Option<usize> = None;
    let custom_n = app.custom_adjusts.len();
    let is_tuyos = app.adjust_cat == "Tuyos";
    grid(ui, &presets, 74.0, |ui, p, w| {
        let pid = paint::str_of(p, "id", "");
        let label = paint::str_of(p, "label", "").to_string();
        let (img, resp) = card(ui, w, w * 0.81, &label, false);
        let painter = ui.painter_at(img.shrink(1.0));
        if pid == "custom" {
            painter.rect_filled(img, 6.0, theme::PANEL2);
            icons::paint(&painter, "plus", img.center(), 20.0, theme::MUTED);
        } else if let Some(adj) = p.get("adjust").filter(|a| a.is_object()) {
            let a: cupcraft_core::Adjust = serde_json::from_value(adj.clone()).unwrap_or_default();
            let fx = paint::fx_from_adjust(&a);
            paint::scene(&painter, img.shrink(1.0), &|c| paint::apply_fx(c, &fx));
        }
        let resp = resp.on_hover_text(&label);
        if resp.clicked() {
            if pid == "custom" {
                save = true;
            } else {
                pick = Some((p.get("adjust").cloned().unwrap_or(json!({})), label.clone()));
            }
        }
        if is_tuyos && pid != "custom" {
            resp.context_menu(|ui| {
                if ui.button("Eliminar ajuste").clicked() {
                    if let Some(i) = (0..custom_n).find(|_| true).and(Some(0)) {
                        let _ = i;
                    }
                    delete = Some(pid.to_string().len()); // marcador; se resuelve abajo por id
                    ui.close();
                }
            });
            if delete.is_some() {
                delete = None;
                // Buscar por id
                let target = pid.to_string();
                ui.memory_mut(|m| m.data.insert_temp(egui::Id::new("adj-del"), target));
            }
        }
    });
    if let Some(target) = ui.memory_mut(|m| m.data.remove_temp::<String>(egui::Id::new("adj-del"))) {
        delete = app.custom_adjusts.iter().position(|v| paint::str_of(v, "id", "") == target);
    }
    if let Some(i) = delete {
        app.custom_adjusts.remove(i);
        app.save_custom_adjusts();
        app.toast("Ajuste eliminado");
    }
    if save {
        app.modal = Some(Modal::SaveAdjust { name: format!("Ajuste {}", app.custom_adjusts.len() + 1) });
    }
    if let Some((adj, label)) = pick {
        let mut full = json!({ "brightness": 100, "contrast": 100, "saturate": 100, "grayscale": 0, "sepia": 0, "hue": 0 });
        if let (Some(f), Some(a)) = (full.as_object_mut(), adj.as_object()) {
            for (k, v) in a {
                f.insert(k.clone(), v.clone());
            }
        }
        let cid = id.clone();
        app.mutate_project(|p| p.update_clip(&cid, json!({ "adjust": full })));
        app.toast(format!("Ajuste aplicado: {label}"));
    }

    // Controles manuales.
    section(ui, "Ajuste manual");
    let Some(clip) = app.editor.project().and_then(|p| p.find_clip(&id)).map(|(_, c)| c.clone()) else { return };
    let mut adj = serde_json::to_value(&clip.adjust).unwrap_or(json!({}));
    let mut changed = false;
    for (key, label, lo, hi, def) in [
        ("brightness", "Brillo", 0.0, 200.0, 100.0),
        ("contrast", "Contraste", 0.0, 200.0, 100.0),
        ("saturate", "Saturación", 0.0, 200.0, 100.0),
        ("hue", "Tono", -180.0, 180.0, 0.0),
        ("sepia", "Sepia", 0.0, 100.0, 0.0),
        ("grayscale", "Blanco y negro", 0.0, 100.0, 0.0),
    ] {
        let mut v = paint::num_of(&adj, key, def);
        ui.horizontal(|ui| {
            ui.add_sized([96.0, 18.0], egui::Label::new(egui::RichText::new(label).color(theme::MUTED)));
            if ui.add(egui::Slider::new(&mut v, lo..=hi).step_by(1.0)).changed() {
                adj[key] = json!(v);
                changed = true;
            }
        });
    }
    let mut opacity = clip.opacity * 100.0;
    let mut scale = clip.scale * 100.0;
    let mut rotate = clip.rotate;
    let mut geo: Option<Value> = None;
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 18.0], egui::Label::new(egui::RichText::new("Opacidad").color(theme::MUTED)));
        if ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).suffix(" %")).changed() {
            geo = Some(json!({ "opacity": opacity / 100.0 }));
        }
    });
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 18.0], egui::Label::new(egui::RichText::new("Escala").color(theme::MUTED)));
        if ui.add(egui::Slider::new(&mut scale, 10.0..=400.0).suffix(" %")).changed() {
            geo = Some(json!({ "scale": scale / 100.0 }));
        }
    });
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 18.0], egui::Label::new(egui::RichText::new("Rotación").color(theme::MUTED)));
        if ui.add(egui::Slider::new(&mut rotate, -180.0..=180.0).suffix("°")).changed() {
            geo = Some(json!({ "rotate": rotate }));
        }
    });
    if changed {
        let cid = id.clone();
        // Cambios continuos: sin entrada de historial por cada paso del deslizador.
        app.editor.mutate_silent(|p| p.update_clip(&cid, json!({ "adjust": adj })));
        app.sync_engine();
    }
    if let Some(g) = geo {
        let cid = id.clone();
        app.editor.mutate_silent(|p| p.update_clip(&cid, g));
        app.sync_engine();
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if secondary_button(ui, "Restablecer").clicked() {
            let cid = id.clone();
            app.mutate_project(|p| {
                p.update_clip(&cid, json!({ "adjust": { "brightness": 100, "contrast": 100, "saturate": 100, "grayscale": 0, "sepia": 0, "hue": 0 } }))
            });
            app.toast("Ajustes restablecidos");
        }
        if secondary_button(ui, "Guardar como ajuste").clicked() {
            app.modal = Some(Modal::SaveAdjust { name: format!("Ajuste {}", app.custom_adjusts.len() + 1) });
        }
    });
}

// ----------------------------------------------------------------------
// Plantillas
// ----------------------------------------------------------------------

pub fn template_card(ui: &mut egui::Ui, t: &Template, w: f32) -> egui::Response {
    let h = w * 0.62;
    let (img, resp) = card(ui, w, h, t.name, false);
    let p = ui.painter_at(img);
    let c0 = paint::hex(t.art[0]);
    let c1 = paint::hex(t.art[1]);
    let c2 = paint::hex(t.art[2]);
    // Degradado diagonal aproximado en franjas.
    let n = 24;
    for i in 0..n {
        let f = i as f32 / n as f32;
        let col = if f < 0.55 { paint::lerp(c0, c1, f / 0.55) } else { paint::lerp(c1, c2, (f - 0.55) / 0.45) };
        let x0 = img.left() + img.width() * f;
        let r = Rect::from_min_max(egui::pos2(x0, img.top()), egui::pos2(x0 + img.width() / n as f32 + 1.0, img.bottom()));
        p.rect_filled(r, 0.0, col);
    }
    let text = json!({ "content": t.content, "size": t.size, "weight": 900, "color": t.color, "align": "center", "shadow": true });
    paint::paint_text(&p, img.translate(Vec2::new(0.0, -h * 0.08)), &text, (h * 0.18) / t.size as f32);
    let sub = json!({ "content": t.subtitle, "size": 40, "weight": 500, "color": t.accent, "align": "center" });
    paint::paint_text(&p, img.translate(Vec2::new(0.0, h * 0.3)), &sub, (h * 0.07) / 40.0);
    p.text(
        img.right_top() + Vec2::new(-6.0, 6.0),
        egui::Align2::RIGHT_TOP,
        t.ratio,
        egui::FontId::proportional(10.0),
        Color32::from_white_alpha(200),
    );
    resp
}

/// Crea un proyecto nuevo a partir de la plantilla (puerto de `applyTemplate`).
pub fn apply_template(app: &mut CupApp, t: &Template) {
    app.save_current();
    app.new_project_named(t.name);
    let ratio = t.ratio.to_string();
    let content = t.content.to_string();
    let subtitle = t.subtitle.to_string();
    let main = json!({ "size": t.size, "weight": 900, "color": t.color, "x": 0, "y": -6 });
    let sub = json!({ "size": (t.size * 0.42).round(), "weight": 500, "color": if t.accent.is_empty() { "#ffffff" } else { t.accent }, "x": 0, "y": 32 });
    let first = app.mutate_project(move |p: &mut Project| {
        p.ratio = ratio;
        p.resolution = "1080p".into();
        p.fps = 30;
        let first = p.add_text_clip(&content, Some(0.4), 6.0, Some(&main));
        if !subtitle.is_empty() {
            let track = p.add_track(TrackType::Text);
            if let Some(id) = p.add_text_clip(&subtitle, Some(0.4), 6.0, Some(&sub)) {
                p.move_clip(&id, Some(0.4), Some(&track));
            }
        }
        first
    });
    app.fps = 30;
    if let Some(Some(id)) = first {
        let start = app.editor.project().and_then(|p| p.find_clip(&id)).map(|(_, c)| c.start).unwrap_or(0.4);
        app.seek(start + 0.05);
        app.editor.select(Some(id));
    }
    app.save_current();
    app.panel_tab = PanelTab::Text;
    app.toast("Plantilla aplicada. Añade tus medios para completarla.");
}

fn templates_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    hint(ui, "Se creará un proyecto nuevo con la plantilla; el proyecto actual se guarda antes.");
    ui.add_space(6.0);
    let mut pick = None;
    grid(ui, TEMPLATES, 130.0, |ui, t, w| {
        if template_card(ui, t, w).on_hover_text(format!("{} · {}", t.name, t.ratio)).clicked() {
            pick = Some(t.id);
        }
    });
    if let Some(id) = pick {
        if let Some(t) = TEMPLATES.iter().find(|t| t.id == id) {
            apply_template(app, t);
        }
    }
}
