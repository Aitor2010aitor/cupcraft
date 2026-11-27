//! cupcraft — escritorio.
//!
//! Interfaz nativa (egui/eframe) sobre el núcleo `cupcraft-core` y el
//! almacén `cupcraft-persist`. El nombre visible es *cupcraft*; `openCUP`
//! solo aparece como nombre técnico interno.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod dialogs;
mod editor;
mod format;
mod fx_3d;
mod fx_engine;
mod fx_extra;
mod fx_text;
mod trans_preview;
mod fx_library;
mod fx_style;
mod fx_table;
mod home;
mod icon_data;
mod icons;
mod karaoke;
mod inspector;
mod inspector_clip;
mod inspector_text;
mod anim_catalog;
mod clip_anim;
mod voice_filters;
mod panels;
mod paint;
mod services;
mod text_styles;
mod theme;
mod timeline;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1120.0, 720.0])
            .with_title("cupcraft")
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(icons::window_icon().unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native(
        "cupcraft",
        options,
        Box::new(|cc| Ok(Box::new(app::CupApp::new(cc)))),
    )
}
