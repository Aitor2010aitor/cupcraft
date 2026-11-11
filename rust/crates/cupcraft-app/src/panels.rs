//! Paneles del editor: Medios, Texto, Filtros, Ajustar.

use crate::app::CupApp;
use crate::paint;
use crate::text_styles;
use crate::theme;
use cupcraft_core::{ClipKind, filters};
use eframe::egui;
use serde_json::json;

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    match app.panel_tab {
        crate::app::PanelTab::Media => media_panel(ui, app),
        crate::app::PanelTab::Text => text_panel(ui, app),
        crate::app::PanelTab::Filters => filters_panel(ui, app),
        crate::app::PanelTab::Adjust => adjust_panel(ui, app),
    }
}

// ---------------- Media ----------------
fn media_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new("Medios").color(theme::TEXT).strong());
    if ui.button("Importar archivos").clicked() {
        if let Some(files) = rfd::FileDialog::new().pick_files() {
            for f in files {
                import_media_file(app, &f);
            }
        }
    }
    ui.add_space(4.0);
    ui.text_edit_singleline(&mut app.media_search);
    ui.add_space(4.0);
    let media_list: Vec<cupcraft_core::Media> = app.editor.project().map(|p| p.media.clone()).unwrap_or_default();
    let search = app.media_search.to_lowercase();
    for m in &media_list {
        let name = m.name.clone().unwrap_or_default();
        if !search.is_empty() && !name.to_lowercase().contains(&search) {
            continue;
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&name).color(theme::TEXT).size(12.0));
            if ui.small_button("Añadir").clicked() {
                let mid = m.id.clone();
                let kind = match m.kind.as_deref() {
                    Some("audio") => ClipKind::Audio,
                    Some("video") => ClipKind::Video,
                    _ => ClipKind::Image,
                };
                let dur = m.duration.unwrap_or(5.0);
                let namec = name.clone();
                app.editor.mutate(|p| {
                    p.add_media_clip(None, kind, &namec, dur, None, Some(mid));
                });
                app.save_current();
            }
        });
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Pistas").color(theme::MUTED).size(11.0));
    if let Some(project) = app.editor.project() {
        for t in &project.tracks {
            ui.label(egui::RichText::new(format!("{} ({})", t.name, t.clips.len())).color(theme::MUTED2).size(11.0));
        }
    }
}

fn import_media_file(app: &mut CupApp, path: &std::path::Path) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => { app.toast("No se pudo leer el archivo"); return; }
    };
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("media").to_string();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let (kind, mime) = match ext.as_str() {
        "mp3" | "wav" | "ogg" | "m4a" | "aac" => ("audio", "audio/mpeg"),
        "mp4" | "mov" | "mkv" | "webm" => ("video", "video/mp4"),
        "png" | "jpg" | "jpeg" | "webp" | "gif" => ("image", "image/png"),
        _ => ("image", "application/octet-stream"),
    };
    let id = cupcraft_core::new_id("med");
    let media = cupcraft_core::Media {
        id: id.clone(),
        kind: Some(kind.to_string()),
        name: Some(name.clone()),
        duration: if kind == "image" { None } else { Some(5.0) },
        thumb: None,
        extra: Default::default(),
    };
    if let Some(store) = app.store.as_mut() {
        let _ = store.save_media(&cupcraft_persist::MediaRecord {
            meta: media.clone(),
            mime: mime.to_string(),
            bytes,
        });
    }
    if let Some(project) = app.editor.project_mut() {
        project.media.push(media);
        project.updated_at = cupcraft_core::now_millis();
    }
    app.toast(format!("Importado: {name}"));
    app.save_current();
}

// ---------------- Text ----------------
fn text_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new("Texto").color(theme::TEXT).strong());
    ui.text_edit_singleline(&mut app.text_content);
    if ui.button("Añadir texto").clicked() {
        let content = if app.text_content.trim().is_empty() { "Tu título aquí".to_string() } else { app.text_content.clone() };
        app.editor.mutate(|p| {
            p.add_text_clip(&content, Some(app.playhead), 5.0, None);
        });
        app.save_current();
    }
    ui.add_space(6.0);
    ui.text_edit_singleline(&mut app.text_search);
    let search = app.text_search.to_lowercase();
    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        ui.label(egui::RichText::new("Estilos").color(theme::MUTED).size(11.0));
        for s in text_styles::text_styles() {
            let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let label = s.get("label").and_then(|v| v.as_str()).unwrap_or(id);
            let sample = s.get("sample").and_then(|v| v.as_str()).unwrap_or("Aa");
            if !search.is_empty() && !label.to_lowercase().contains(&search) { continue; }
            if ui.selectable_label(app.selected_text_style.as_deref() == Some(id), format!("{label}  {sample}")).clicked() {
                app.selected_text_style = Some(id.to_string());
                if let Some(patch) = s.get("patch") {
                    apply_text_patch(app, patch);
                }
            }
        }
        ui.label(egui::RichText::new("Burbujas").color(theme::MUTED).size(11.0));
        for b in text_styles::text_bubbles() {
            let label = b.get("label").and_then(|v| v.as_str()).unwrap_or("");
            if ui.small_button(label).clicked() {
                if let Some(patch) = b.get("patch") { apply_text_patch(app, patch); }
            }
        }
        ui.label(egui::RichText::new("Efectos").color(theme::MUTED).size(11.0));
        for e in text_styles::text_effects() {
            let label = e.get("label").and_then(|v| v.as_str()).unwrap_or("");
            if ui.small_button(label).clicked() {
                if let Some(patch) = e.get("patch") { apply_text_patch(app, patch); }
            }
        }
    });
    ui.label(egui::RichText::new("Fuente").color(theme::MUTED).size(11.0));
    egui::ComboBox::from_id_salt("font")
        .selected_text("Inter")
        .show_ui(ui, |ui| {
            for f in text_styles::text_fonts() {
                let v = f.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let l = f.get("label").and_then(|v| v.as_str()).unwrap_or(v);
                if ui.selectable_label(false, l).clicked() {
                    apply_text_patch(app, &json!({ "font": v }));
                }
            }
        });
    ui.horizontal(|ui| {
        for c in text_styles::text_cases() {
            let v = c.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let l = c.get("label").and_then(|v| v.as_str()).unwrap_or(v);
            if ui.small_button(l).clicked() {
                apply_text_patch(app, &json!({ "textCase": v }));
            }
        }
    });
    ui.horizontal(|ui| {
        for a in text_styles::text_aligns() {
            let v = a.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let l = a.get("label").and_then(|v| v.as_str()).unwrap_or(v);
            if ui.small_button(l).clicked() {
                apply_text_patch(app, &json!({ "align": v }));
            }
        }
    });
}

fn apply_text_patch(app: &mut CupApp, patch: &serde_json::Value) {
    let sel = app.editor.selection().map(str::to_string);
    if let Some(id) = sel {
        app.editor.mutate(|p| {
            let patch = json!({ "text": patch });
            p.update_clip(&id, patch);
        });
        app.save_current();
    } else {
        app.toast("Selecciona un clip de texto");
    }
}

// ---------------- Filters ----------------
fn filters_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    let selected_is_media = app.editor.selection().and_then(|id| {
        app.editor.project()?.find_clip(id).map(|(_, c)| matches!(c.kind, ClipKind::Video | ClipKind::Image))
    }).unwrap_or(false);
    let hint = if !selected_is_media {
        "Selecciona un clip de video o imagen para aplicarle un filtro."
    } else {
        "Elige un filtro de la línea cupcraft para aplicarlo al clip seleccionado."
    };
    ui.label(egui::RichText::new(hint).color(theme::MUTED).size(11.0));
    ui.add_space(4.0);
    ui.text_edit_singleline(&mut app.filter_query);
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.small_button("Todos").clicked() { app.filter_category = None; }
        for &c in filters::CATEGORIES.iter() {
            if ui.small_button(c).clicked() { app.filter_category = Some(c.to_string()); }
        }
    });
    ui.add_space(4.0);
    let cat = app.filter_category.as_deref();
    let q = if app.filter_query.trim().is_empty() { None } else { Some(app.filter_query.as_str()) };
    let list = filters::search(cat, q);
    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for f in list {
                let key = f.key;
                let label = f.label;
                let css = f.css;
                let selected = app.editor.selection().and_then(|id| {
                    app.editor.project()?.find_clip(id).and_then(|(_, c)| {
                        filters::line_key_for_clip(&c.filter, c.gl_filter.as_deref(), c.lut.as_deref())
                    })
                }) == Some(key);
                let resp = ui.add_sized(egui::vec2(74.0, 60.0), egui::Button::new(egui::RichText::new(label).size(10.0)));
                let rect = resp.rect;
                // paint mini scene overlay
                let p = ui.painter().with_clip_rect(rect);
                let fx = paint::parse_css(css);
                let map = move |c: egui::Color32| paint::apply_fx(c, &fx);
                paint::scene(&p, rect.shrink(2.0), &map);
                if resp.clicked() {
                    if let Some(id) = app.editor.selection().map(str::to_string) {
                        if app.editor.project_mut().map(|pr| pr.apply_filter(&id, key)).unwrap_or(false) {
                            app.save_current();
                        }
                    } else {
                        app.toast("Selecciona un clip primero");
                    }
                }
                if selected {
                    ui.painter().rect_stroke(rect, egui::CornerRadius::from(6), egui::Stroke::new(2.0, theme::ACCENT), egui::StrokeKind::Inside);
                }
            }
        });
    });
    if ui.button("Quitar filtro").clicked() {
        if let Some(id) = app.editor.selection().map(str::to_string) {
            if app.editor.project_mut().map(|pr| pr.apply_filter(&id, "none")).unwrap_or(false) {
                app.save_current();
            }
        }
    }
}

// ---------------- Adjust ----------------
fn adjust_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new("Ajustar").color(theme::TEXT).strong());
    ui.horizontal_wrapped(|ui| {
        for c in text_styles::adjust_categories() {
            let Some(c) = c.as_str() else { continue; };
            if ui.small_button(c).clicked() { app.adjust_cat = c.to_string(); }
        }
    });
    ui.add_space(4.0);
    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for pr in text_styles::adjust_presets() {
            let cat = pr.get("category").and_then(|v| v.as_str()).unwrap_or("");
            if cat != app.adjust_cat { continue; }
            let id = pr.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let label = pr.get("label").and_then(|v| v.as_str()).unwrap_or(id);
            if ui.small_button(label).clicked() {
                if let Some(id) = app.editor.selection().map(str::to_string) {
                    let patch = if let Some(adj) = pr.get("adjust") {
                        if adj.is_null() {
                            json!({ "adjust": {} })
                        } else {
                            json!({ "adjust": adj })
                        }
                    } else {
                        json!({ "adjust": {} })
                    };
                    app.editor.mutate(|p| { p.update_clip(&id, patch); });
                    app.save_current();
                } else {
                    app.toast("Selecciona un clip");
                }
            }
        }
    });
    ui.add_space(6.0);
    let sel = app.editor.selection().map(str::to_string);
    if let Some(id) = sel {
        let (b, c, s) = app.editor.project().and_then(|p| {
            p.find_clip(&id).map(|(_, cl)| (cl.adjust.brightness, cl.adjust.contrast, cl.adjust.saturate))
        }).unwrap_or((100.0, 100.0, 100.0));
        let mut bb = b; let mut cc = c; let mut ss = s;
        ui.label("Brillo");
        if ui.add(egui::Slider::new(&mut bb, 0.0..=200.0)).changed() {
            apply_adjust(app, &id, "brightness", bb);
        }
        ui.label("Contraste");
        if ui.add(egui::Slider::new(&mut cc, 0.0..=200.0)).changed() {
            apply_adjust(app, &id, "contrast", cc);
        }
        ui.label("Saturación");
        if ui.add(egui::Slider::new(&mut ss, 0.0..=200.0)).changed() {
            apply_adjust(app, &id, "saturate", ss);
        }
    }
}

fn apply_adjust(app: &mut CupApp, id: &str, key: &str, val: f64) {
    app.editor.mutate(|p| {
        let patch = json!({ "adjust": { key: val } });
        p.update_clip(id, patch);
    });
    app.save_current();
}
