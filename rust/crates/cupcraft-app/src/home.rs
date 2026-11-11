//! Pantalla de inicio: cuadrícula de proyectos, tarjetas, renombrar y borrar.

use crate::app::CupApp;
use crate::format::{format_bytes, time_ago};
use crate::theme;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::BG))
        .show(ui, |ui| {
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.add_space(32.0);
                ui.label(egui::RichText::new("cupcraft").color(theme::TEXT).size(28.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(32.0);
                    if ui.button(egui::RichText::new("+ Nuevo proyecto").color(theme::ACCENT)).clicked() {
                        app.new_project();
                    }
                    if ui.button("Importar .opencup.json").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("openCUP", &["json"])
                            .pick_file()
                        {
                            if let Ok(text) = std::fs::read_to_string(&path) {
                                if let Some(store) = app.store.as_mut() {
                                    match store.import_project_file(&text) {
                                        Ok(rep) => app.toast(format!("Importado: {}", rep.id)),
                                        Err(e) => app.toast(format!("Error: {e}")),
                                    }
                                }
                            }
                        }
                    }
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(16.0);

            let projects = app.store.as_ref().map(|s| s.list_projects().unwrap_or_default()).unwrap_or_default();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(24.0);
                    for p in &projects {
                        project_card(ui, app, p);
                    }
                    if projects.is_empty() {
                        ui.label(egui::RichText::new("No hay proyectos todavía. Crea uno nuevo.").color(theme::MUTED));
                    }
                });
            });
        });
    crate::app::show_toast(&ctx, app);
}

fn project_card(ui: &mut egui::Ui, app: &mut CupApp, p: &cupcraft_core::Project) {
    let id = p.id.clone();
    let renaming = app.rename_id.as_deref() == Some(id.as_str());
    egui::Frame::NONE
        .fill(theme::PANEL)
        .stroke(egui::Stroke::new(1.0, theme::LINE_SOFT))
        .corner_radius(egui::CornerRadius::from(10))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(220.0);
            ui.set_max_width(220.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(196.0, 110.0), egui::Sense::click());
            ui.painter().rect_filled(rect, egui::CornerRadius::from(6), theme::PANEL3);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "cupcraft", egui::FontId::proportional(16.0), theme::MUTED2);
            ui.add_space(8.0);
            if renaming {
                let resp = ui.text_edit_singleline(&mut app.rename_buf);
                if resp.lost_focus() {
                    let name = app.rename_buf.clone();
                    app.rename_project(&id, &name);
                    app.rename_id = None;
                    app.toast("Renombrado");
                }
            } else {
                ui.label(egui::RichText::new(&p.name).color(theme::TEXT).strong());
            }
            ui.label(egui::RichText::new(time_ago(p.updated_at)).color(theme::MUTED).size(11.0));
            ui.label(egui::RichText::new(format!("{} · {}", p.ratio, p.fps)).color(theme::MUTED2).size(10.0));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.small_button("Abrir").clicked() {
                    app.open_project(&id);
                }
                if ui.small_button("Renombrar").clicked() {
                    app.rename_id = Some(id.clone());
                    app.rename_buf = p.name.clone();
                }
                let deleting = app.pending_delete.as_ref().map(|(x, _)| x == &id).unwrap_or(false);
                let label = if deleting { "Confirmar" } else { "Eliminar" };
                if ui.small_button(label).clicked() {
                    if deleting {
                        app.delete_project(&id);
                        app.pending_delete = None;
                    } else {
                        app.pending_delete = Some((id.clone(), now_secs() + 4.0));
                    }
                }
            });
            let _ = format_bytes;
        });
}

fn now_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}
