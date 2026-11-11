//! Tema visual de cupcraft.
//!
//! Replica las variables CSS de `src/styles.css` (líneas 1-31) y las aplica
//! al `Context` de egui. Las tipografías se leen del sistema en tiempo de
//! ejecución (no se redistribuye ninguna fuente).

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
    Visuals,
};
use std::sync::Arc;

// --- variables de styles.css ---------------------------------------------
pub const BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
pub const BG2: Color32 = Color32::from_rgb(0x10, 0x10, 0x10);
pub const PANEL: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const PANEL2: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
pub const PANEL3: Color32 = Color32::from_rgb(0x30, 0x30, 0x30);
pub const FIELD: Color32 = Color32::from_rgb(0x3b, 0x3b, 0x3b);
pub const ELEV: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
pub const LINE: Color32 = Color32::from_rgb(0x1d, 0x1d, 0x1d);
pub const LINE_SOFT: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
pub const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf2);
pub const MUTED: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x8a);
pub const MUTED2: Color32 = Color32::from_rgb(0x6c, 0x6c, 0x6c);
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0xe5, 0xf0);
pub const ACCENT2: Color32 = Color32::from_rgb(0x00, 0xc1, 0xcd);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x12, 0x3b, 0x3e);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x04, 0x14, 0x12);
pub const BLUE: Color32 = Color32::from_rgb(0x4d, 0x8d, 0xff);
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x5c, 0x5c);
pub const WARN: Color32 = Color32::from_rgb(0xff, 0xc8, 0x57);
pub const PLAYHEAD: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xe8);
pub const GUIDE: Color32 = Color32::from_rgb(0xff, 0xd1, 0x66);

pub const RADIUS: u8 = 8;

/// Aplica el tema completo (tipografías + estilo) al contexto.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(TEXT);
    v.faint_bg_color = PANEL2;
    v.extreme_bg_color = FIELD;
    v.text_edit_bg_color = Some(FIELD);
    v.window_fill = PANEL;
    v.panel_fill = PANEL;
    v.window_corner_radius = CornerRadius::from(RADIUS);
    v.menu_corner_radius = CornerRadius::from(RADIUS);
    v.hyperlink_color = ACCENT2;
    v.warn_fg_color = WARN;
    v.error_fg_color = DANGER;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x00, 0xc1, 0xcd, 70);
    v.selection.stroke = Stroke::new(1.0, ACCENT2);

    v.widgets.noninteractive.bg_fill = PANEL2;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    v.widgets.noninteractive.corner_radius = CornerRadius::from(6);

    v.widgets.inactive.bg_fill = PANEL3;
    v.widgets.inactive.weak_bg_fill = PANEL3;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE_SOFT);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.corner_radius = CornerRadius::from(6);

    v.widgets.hovered.bg_fill = FIELD;
    v.widgets.hovered.weak_bg_fill = FIELD;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT2);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.hovered.corner_radius = CornerRadius::from(6);

    v.widgets.active.bg_fill = ACCENT_SOFT;
    v.widgets.active.weak_bg_fill = ACCENT_SOFT;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT2);
    v.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.corner_radius = CornerRadius::from(6);

    ctx.set_visuals(v);

    let sans = FontFamily::Name(CUP_SANS.into());
    let sans_bold = FontFamily::Name(CUP_SANS_BOLD.into());
    let mono = FontFamily::Name(CUP_MONO.into());
    ctx.all_styles_mut(move |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size = egui::vec2(28.0, 24.0);
        style.text_styles.insert(TextStyle::Body, FontId::new(14.0, sans.clone()));
        style.text_styles.insert(TextStyle::Button, FontId::new(13.5, sans.clone()));
        style.text_styles.insert(TextStyle::Small, FontId::new(11.5, sans.clone()));
        style.text_styles.insert(TextStyle::Heading, FontId::new(20.0, sans_bold.clone()));
        style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, mono.clone()));
    });
}

pub const CUP_SANS: &str = "cup-sans";
pub const CUP_SANS_BOLD: &str = "cup-sans-bold";
pub const CUP_MONO: &str = "cup-mono";
pub const CUP_SERIF: &str = "cup-serif";

/// `FontId` para un peso dado: la negrita usa una familia distinta porque
/// egui no tiene variantes de peso dentro de una familia.
pub fn font_for(weight: f64, size: f32) -> FontId {
    let family = if weight >= 600.0 {
        FontFamily::Name(CUP_SANS_BOLD.into())
    } else {
        FontFamily::Name(CUP_SANS.into())
    };
    FontId::new(size, family)
}

/// Familia por nombre de fuente del catálogo (`Inter`, `Georgia`…).
pub fn font_family_for(name: &str) -> FontFamily {
    match name.to_lowercase().as_str() {
        "georgia" | "times new roman" | "palatino linotype" | "cambria" => {
            FontFamily::Name(CUP_SERIF.into())
        }
        "consolas" | "courier new" => FontFamily::Name(CUP_MONO.into()),
        _ => FontFamily::Name(CUP_SANS.into()),
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let sans = load(&mut fonts, CUP_SANS, r"C:\Windows\Fonts\segoeui.ttf");
    let bold = load(&mut fonts, CUP_SANS_BOLD, r"C:\Windows\Fonts\segoeuib.ttf");
    let mono = load(&mut fonts, CUP_MONO, r"C:\Windows\Fonts\consola.ttf");
    let serif = load(&mut fonts, CUP_SERIF, r"C:\Windows\Fonts\georgia.ttf");

    let mut sans_keys = vec![CUP_SANS.to_owned()];
    if sans {
        sans_keys.push("Ubuntu-Light".to_owned());
    }
    let mut bold_keys = vec![CUP_SANS_BOLD.to_owned()];
    if sans {
        bold_keys.push(CUP_SANS.to_owned());
    }
    bold_keys.push("Ubuntu-Light".to_owned());
    let mut mono_keys = vec![CUP_MONO.to_owned()];
    if mono {
        mono_keys.push("Hack".to_owned());
    }
    let mut serif_keys = vec![CUP_SERIF.to_owned()];
    if sans {
        serif_keys.push(CUP_SANS.to_owned());
    }
    serif_keys.push("Ubuntu-Light".to_owned());

    for (name, keys) in [
        (CUP_SANS, sans_keys),
        (CUP_SANS_BOLD, bold_keys),
        (CUP_MONO, mono_keys),
        (CUP_SERIF, serif_keys),
    ] {
        // Si la fuente del sistema no existe, caemos siempre a una del
        // paquete por defecto de egui para no dejar la familia vacía.
        let keys: Vec<String> = keys
            .into_iter()
            .filter(|k| fonts.font_data.contains_key(k))
            .collect();
        let keys = if keys.is_empty() {
            vec!["Ubuntu-Light".to_owned()]
        } else {
            keys
        };
        fonts
            .families
            .insert(FontFamily::Name(name.into()), keys);
    }

    ctx.set_fonts(fonts);
}

fn load(fonts: &mut FontDefinitions, key: &str, path: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            fonts.font_data.insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
            true
        }
        Err(_) => false,
    }
}
