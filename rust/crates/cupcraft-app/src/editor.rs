//! Editor principal: barra superior, vista previa y cableado de paneles/timeline.

use crate::app::CupApp;
use crate::format::format_time;
use crate::paint;
use crate::panels;
use crate::theme;
use crate::timeline as tl;
use cupcraft_core::ClipKind;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let ctx = ui.ctx().clone();

    // Top bar
    egui::Panel::top("topbar")
        .exact_size(42.0)
        .frame(egui::Frame::NONE.fill(theme::PANEL).stroke(egui::Stroke::new(1.0, theme::LINE)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button("← Inicio").clicked() {
                    app.back_home();
                }
                ui.separator();
                let name = app.editor.project().map(|p| p.name.clone()).unwrap_or_default();
                ui.label(egui::RichText::new(name).color(theme::TEXT).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Exportar").clicked() {
                        export_project(app);
                    }
                    if ui.button("Guardar").clicked() {
                        app.save_current();
                        app.toast("Guardado");
                    }
                    if ui.button(if app.playing { "⏸" } else { "▶" }).clicked() {
                        app.playing = !app.playing;
                        app.last_tick = now();
                    }
                    ui.label(egui::RichText::new(format_time(app.playhead)).color(theme::TEXT));
                });
            });
        });

    // Timeline at bottom (resizable)
    egui::Panel::bottom("timeline")
        .resizable(true)
        .default_size(360.0)
        .min_size(320.0)
        .max_size(440.0)
        .frame(egui::Frame::NONE.fill(theme::BG2))
        .show(ui, |ui| {
            tl::show(ui, app);
        });

    // Left strip + panels
    egui::Panel::left("leftpanel")
        .default_size(420.0)
        .min_size(340.0)
        .max_size(640.0)
        .resizable(true)
        .frame(egui::Frame::NONE.fill(theme::PANEL))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(44.0);
                strip_tab(ui, app, crate::app::PanelTab::Media, "Archivos");
                strip_tab(ui, app, crate::app::PanelTab::Text, "Texto");
                strip_tab(ui, app, crate::app::PanelTab::Filters, "Filtros");
                strip_tab(ui, app, crate::app::PanelTab::Adjust, "Ajustar");
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                panels::show(ui, app);
            });
        });

    // Central preview
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::BG))
        .show(ui, |ui| {
            preview(ui, app);
        });

    handle_shortcuts(&ctx, app);

    crate::app::show_toast(&ctx, app);
}

fn strip_tab(ui: &mut egui::Ui, app: &mut CupApp, tab: crate::app::PanelTab, label: &str) {
    let selected = app.panel_tab == tab;
    let color = if selected { theme::ACCENT } else { theme::MUTED };
    let resp = ui.selectable_label(selected, egui::RichText::new(label).color(color));
    if resp.clicked() {
        app.panel_tab = tab;
    }
    if selected {
        let r = resp.rect;
        ui.painter().line_segment(
            [egui::pos2(r.min.x, r.max.y), egui::pos2(r.max.x, r.max.y)],
            egui::Stroke::new(2.0, theme::ACCENT),
        );
    }
}

fn preview(ui: &mut egui::Ui, app: &mut CupApp) {
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    let p = ui.painter().with_clip_rect(rect);
    p.rect_filled(rect, egui::CornerRadius::ZERO, theme::BG2);
    let stage_w = rect.width().min(rect.height() * 16.0 / 9.0);
    let stage_h = stage_w * 9.0 / 16.0;
    let stage = egui::Rect::from_center_size(rect.center(), egui::vec2(stage_w, stage_h));
    p.rect_filled(stage, egui::CornerRadius::from(4), theme::PANEL3);
    let map = |c: egui::Color32| c;
    paint::scene(&p, stage, &map);
    if let Some(project) = app.editor.project() {
        let scale = stage.height() / 1080.0;
        for track in &project.tracks {
            for clip in &track.clips {
                if clip.kind == ClipKind::Text {
                    if let Some(text) = &clip.text {
                        paint::paint_text(&p, stage, text, scale);
                    }
                }
            }
        }
    }
}

fn export_project(app: &mut CupApp) {
    if let Some(project) = app.editor.project() {
        let id = project.id.clone();
        let name = project.name.clone();
        if let Some(store) = app.store.as_mut() {
            if let Ok(file) = store.export_project_file(&id) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("openCUP", &["json"])
                    .set_file_name(cupcraft_persist::export_file_name(&name))
                    .save_file()
                {
                    let _ = std::fs::write(path, file.to_text());
                    app.toast("Exportado");
                }
            }
        }
    }
}

fn handle_shortcuts(ctx: &egui::Context, app: &mut CupApp) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
        if app.editor.redo() { app.toast("Rehecho"); }
    } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
        if app.editor.undo() { app.toast("Deshacer"); }
    } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
        if app.editor.redo() { app.toast("Rehecho"); }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
        app.playing = !app.playing;
        app.last_tick = now();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
        if let Some(id) = app.editor.selection().map(str::to_string) {
            app.editor.mutate(|p| { p.remove_clip(&id); });
            app.save_current();
        }
    }
}

fn now() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}
