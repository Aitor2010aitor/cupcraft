//! cupcraft — escritorio.
//!
//! Interfaz nativa (egui/eframe) sobre el núcleo `cupcraft-core` y el
//! almacén `cupcraft-persist`. El nombre visible es *cupcraft*; `openCUP`
//! solo aparece como nombre técnico interno.

mod app;
mod editor;
mod format;
mod home;
mod panels;
mod paint;
mod text_styles;
mod theme;
mod timeline;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1120.0, 720.0])
            .with_title("cupcraft"),
        ..Default::default()
    };
    eframe::run_native(
        "cupcraft",
        options,
        Box::new(|cc| Ok(Box::new(app::CupApp::new(cc)))),
    )
}
