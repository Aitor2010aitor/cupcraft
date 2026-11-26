//! Iconos de la interfaz original (trazados SVG de `src/editor/icons.js`)
//! rasterizados de forma nativa con resvg/tiny-skia y cacheados como texturas egui.
//!
//! Mismos parámetros que la versión web: viewBox 24, `stroke="currentColor"`,
//! `stroke-width="1.7"`, extremos y uniones redondeados. El color se aplica como
//! tinte en el momento de pintar, así que solo se cachea una textura por
//! (icono, tamaño en píxeles físicos).

use crate::icon_data::ICONS;
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, TextureHandle, Vec2};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

type Cache = Mutex<HashMap<(String, u32), Option<TextureHandle>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn body_of(name: &str) -> &'static str {
    ICONS
        .iter()
        .find(|(n, _)| *n == name)
        .or_else(|| ICONS.iter().find(|(n, _)| *n == "info"))
        .map(|(_, b)| *b)
        .unwrap_or("")
}

fn rasterize(name: &str, px: u32) -> Option<egui::ColorImage> {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" viewBox="0 0 24 24" color="#ffffff" fill="none" stroke="#ffffff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
        body_of(name)
    );
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(px, px)?;
    let s = px as f32 / 24.0;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(s, s), &mut pixmap.as_mut());
    // tiny-skia usa alfa premultiplicado; egui también lo acepta como premultiplicado.
    Some(egui::ColorImage::from_rgba_premultiplied([px as usize, px as usize], pixmap.data()))
}

/// Devuelve la textura del icono para un tamaño lógico concreto.
pub fn texture(ctx: &egui::Context, name: &str, size: f32) -> Option<TextureHandle> {
    let px = (size * ctx.pixels_per_point()).round().max(8.0) as u32;
    let key = (name.to_string(), px);
    let mut map = cache().lock().ok()?;
    if let Some(t) = map.get(&key) {
        return t.clone();
    }
    let tex = rasterize(name, px).map(|img| {
        ctx.load_texture(format!("icon-{name}-{px}"), img, egui::TextureOptions::LINEAR)
    });
    map.insert(key, tex.clone());
    tex
}

/// Pinta un icono centrado en `center`.
pub fn paint(p: &egui::Painter, name: &str, center: Pos2, size: f32, color: Color32) {
    if let Some(tex) = texture(p.ctx(), name, size) {
        let rect = Rect::from_center_size(center, Vec2::splat(size));
        p.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), color);
    }
}

/// Icono suelto que ocupa su tamaño en el layout.
pub fn icon(ui: &mut egui::Ui, name: &str, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint(ui.painter(), name, rect.center(), size, color);
    resp
}

/// Botón de icono (equivalente a `.icon-btn` del CSS original: 28 px, radio 4,
/// fondo #2f2f2f al pasar el ratón).
pub fn icon_button(ui: &mut egui::Ui, name: &str, size: f32, color: Color32, tooltip: &str) -> Response {
    icon_button_sized(ui, name, size, Vec2::splat(28.0), color, tooltip, false)
}

pub fn icon_button_sized(
    ui: &mut egui::Ui,
    name: &str,
    size: f32,
    box_size: Vec2,
    color: Color32,
    tooltip: &str,
    active: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(box_size, Sense::click());
    let hovered = resp.hovered();
    if active {
        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(0x12, 0x3b, 0x3e));
    } else if hovered {
        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(0x2f, 0x2f, 0x2f));
    }
    let c = if active {
        crate::theme::ACCENT
    } else if hovered {
        Color32::WHITE
    } else {
        color
    };
    paint(ui.painter(), name, rect.center(), size, c);
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}

/// Logotipo de cupcraft (`src/logo.png`) embebido en el ejecutable.
pub fn logo(ctx: &egui::Context) -> Option<TextureHandle> {
    static LOGO: OnceLock<Mutex<Option<TextureHandle>>> = OnceLock::new();
    let slot = LOGO.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().ok()?;
    if guard.is_none() {
        let bytes = include_bytes!("logo.png");
        if let Ok(img) = image::load_from_memory(bytes) {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            *guard = Some(ctx.load_texture("cupcraft-logo", ci, egui::TextureOptions::LINEAR));
        }
    }
    guard.clone()
}

pub fn paint_logo(p: &egui::Painter, rect: Rect, tint: Color32) {
    if let Some(tex) = logo(p.ctx()) {
        p.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
    }
}

/// Icono de la ventana (barra de tareas de Windows) a partir del mismo logo.
pub fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("logo.png")).ok()?.to_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}
