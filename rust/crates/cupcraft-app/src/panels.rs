//! Paneles del editor: Medios, Texto, Filtros, Ajustar, Stickers, Efectos, Transiciones.

use crate::app::CupApp;
use crate::paint;
use crate::text_styles;
use crate::theme;
use cupcraft_core::{ClipKind, filters};
use eframe::egui;
use serde_json::json;

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    match app.panel_tab {
        crate::app::PanelTab::Media       => media_panel(ui, app),
        crate::app::PanelTab::Text        => text_panel(ui, app),
        crate::app::PanelTab::Filters     => filters_panel(ui, app),
        crate::app::PanelTab::Adjust      => adjust_panel(ui, app),
        crate::app::PanelTab::Stickers    => stickers_panel(ui, app),
        crate::app::PanelTab::Effects     => effects_panel(ui, app),
        crate::app::PanelTab::Transitions => transitions_panel(ui, app),
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
        let (b, c, s, temp, hue, sharp, vig) = app.editor.project().and_then(|p| {
            p.find_clip(&id).map(|(_, cl)| {
                let extra_f = |k: &str| cl.adjust.extra.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
                (
                    cl.adjust.brightness,
                    cl.adjust.contrast,
                    cl.adjust.saturate,
                    extra_f("temperature"),
                    extra_f("hue"),
                    extra_f("sharpen"),
                    extra_f("vignette"),
                )
            })
        }).unwrap_or((100.0, 100.0, 100.0, 0.0, 0.0, 0.0, 0.0));
        let mut bb = b; let mut cc = c; let mut ss = s;
        let mut t_temp = temp; let mut h_hue = hue; let mut sh_sharp = sharp; let mut v_vig = vig;
        ui.label("Brillo");
        if ui.add(egui::Slider::new(&mut bb, 0.0..=200.0).suffix("%")).changed() {
            apply_adjust(app, &id, "brightness", bb);
        }
        ui.label("Contraste");
        if ui.add(egui::Slider::new(&mut cc, 0.0..=200.0).suffix("%")).changed() {
            apply_adjust(app, &id, "contrast", cc);
        }
        ui.label("Saturación");
        if ui.add(egui::Slider::new(&mut ss, 0.0..=200.0).suffix("%")).changed() {
            apply_adjust(app, &id, "saturate", ss);
        }
        ui.label("Temperatura");
        if ui.add(egui::Slider::new(&mut t_temp, -100.0..=100.0)).changed() {
            apply_adjust(app, &id, "temperature", t_temp);
        }
        ui.label("Tono / Matiz");
        if ui.add(egui::Slider::new(&mut h_hue, -180.0..=180.0).suffix("°")).changed() {
            apply_adjust(app, &id, "hue", h_hue);
        }
        ui.label("Nitidez");
        if ui.add(egui::Slider::new(&mut sh_sharp, 0.0..=100.0).suffix("%")).changed() {
            apply_adjust(app, &id, "sharpen", sh_sharp);
        }
        ui.label("Viñeta");
        if ui.add(egui::Slider::new(&mut v_vig, 0.0..=100.0).suffix("%")).changed() {
            apply_adjust(app, &id, "vignette", v_vig);
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

// ─────────────────────────── Stickers ────────────────────────────────────────

/// Catálogo de emojis/stickers. Un clic añade el sticker en el playhead actual.
fn stickers_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new("Stickers").color(theme::TEXT).strong());
    ui.label(egui::RichText::new("Haz clic para añadir en el playhead.").color(theme::MUTED).size(11.0));
    ui.add_space(4.0);

    // Grupos de stickers igual que el JS legacy (icons.js STICKERS)
    const GROUPS: &[(&str, &[&str])] = &[
        ("Expresiones",   &["😀","😂","🥹","😍","🤩","😎","🥳","😭","😱","🤔","😴","🤯","😇","🫶","❤️","💔","🔥","✨","💯","👍"]),
        ("Naturaleza",    &["🌸","🌺","🌻","🍀","🌴","⭐","🌙","☀️","🌈","🦋","🌊","❄️","🍁","🌿","🪐"]),
        ("Comida",        &["🍕","🍔","🌮","🍣","🍰","☕","🧋","🍓","🍉","🥑"]),
        ("Objetos",       &["🎵","🎶","🎤","🎬","📸","💎","👑","🏆","🎯","🎁","🪄","💫"]),
        ("Texto/formas",  &["💬","❗","❓","⬆️","⬇️","➡️","⭕","❌","✅","🔴","🟡","🟢"]),
    ];

    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
        for (group_name, emojis) in GROUPS {
            ui.label(egui::RichText::new(*group_name).color(theme::MUTED).size(11.0));
            ui.horizontal_wrapped(|ui| {
                for &emoji in *emojis {
                    let btn = ui.add_sized(
                        egui::vec2(36.0, 36.0),
                        egui::Button::new(egui::RichText::new(emoji).size(20.0)),
                    );
                    let clicked = btn.clicked();
                    btn.on_hover_text(emoji);
                    if clicked {
                        let ph = app.playhead;
                        let content = emoji.to_string();
                        app.editor.mutate(|p| {
                            p.add_sticker_clip(&content, Some(4.0), Some(ph), None);
                        });
                        app.save_current();
                        app.toast(format!("Sticker añadido: {emoji}"));
                    }
                }
            });
            ui.add_space(4.0);
        }
    });
}

// ─────────────────────────── Efectos ─────────────────────────────────────────

/// Panel de efectos visuales. Muestra los 13 efectos de post-proceso + 8 de CapCut.
/// Un clic aplica el efecto como clip de efecto en la pista Overlay.
fn effects_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new("Efectos").color(theme::TEXT).strong());
    ui.label(egui::RichText::new("Haz clic para añadir efecto en el playhead.").color(theme::MUTED).size(11.0));
    ui.add_space(4.0);

    // Efectos de post-proceso (13 shaders WGSL portados)
    const POST_FX: &[(&str, &str, &str)] = &[
        ("radial_blur",    "Desenfoque radial",    "🌀"),
        ("directional_blur","Desenfoque lineal",   "↔"),
        ("rgb_split",      "Separación RGB",       "🌈"),
        ("chromatic",      "Aberración cromática",  "🔴"),
        ("vhs",            "VHS",                  "📼"),
        ("zoom_pulse",     "Pulso zoom",            "🔎"),
        ("pixelate",       "Pixelar",              "▦"),
        ("mirror",         "Espejo",               "🪞"),
        ("vignette",       "Viñeta",               "⭕"),
        ("grain",          "Grano",                "🌾"),
        ("posterize",      "Posterizar",           "🎨"),
        ("wave",           "Ola",                  "〜"),
        ("bloom",          "Bloom/brillo",         "✨"),
    ];

    // Efectos firma CapCut (8 LUT/shaders especiales)
    const CAPCUT_FX: &[(&str, &str, &str)] = &[
        ("bruma_borgona",    "Bruma borgoña",      "🍷"),
        ("atardecer_ensueno","Atardecer ensueño",  "🌅"),
        ("brillo_brumoso",   "Brillo brumoso",     "☁"),
        ("pixel_noir",       "Pixel Noir",         "🎞"),
        ("carino_retro",     "Cariño retro",       "💌"),
        ("sueno_caqui",      "Sueño caqui",        "🌿"),
        ("jengibre_suave",   "Jengibre suave",     "🍊"),
        ("lienzo_embrujado", "Lienzo embrujado",   "🎭"),
    ];

    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
        ui.label(egui::RichText::new("Post-proceso").color(theme::MUTED).size(11.0));
        effect_grid(ui, app, POST_FX);
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Firma CapCut").color(theme::MUTED).size(11.0));
        effect_grid(ui, app, CAPCUT_FX);
    });
}

fn effect_grid(ui: &mut egui::Ui, app: &mut CupApp, list: &[(&str, &str, &str)]) {
    ui.horizontal_wrapped(|ui| {
        for &(id, label, icon) in list {
            let btn = ui.add_sized(
                egui::vec2(84.0, 56.0),
                egui::Button::new(egui::RichText::new(format!("{icon}\n{label}")).size(10.0)),
            );
            if btn.clicked() {
                let ph = app.playhead;
                let effect_id = id.to_string();
                let effect_name = label.to_string();
                app.editor.mutate(|p| {
                    p.add_effect_clip(&effect_id, &effect_name, Some(4.0), Some(ph), Some(1.0));
                });
                app.save_current();
                app.toast(format!("Efecto añadido: {label}"));
            }
        }
    });
}

// ─────────────────────────── Transiciones ────────────────────────────────────

/// Panel de transiciones. Aplica la transición al clip seleccionado o al siguiente.
fn transitions_panel(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new("Transiciones").color(theme::TEXT).strong());
    ui.label(
        egui::RichText::new("Selecciona un clip y haz clic para aplicar la transición a su salida.")
            .color(theme::MUTED).size(11.0)
    );
    ui.add_space(4.0);

    // Tabla de transiciones (67 tipos, igual que transitions.wgsl)
    const TRANSITIONS: &[(&str, &str, &str)] = &[
        // Básicas
        ("fade",          "Fundido",           "Básicas"),
        ("cut",           "Corte directo",     "Básicas"),
        ("dissolve",      "Disolución",        "Básicas"),
        ("crossfade",     "Crossfade",         "Básicas"),
        ("wipe_right",    "Barrido →",         "Básicas"),
        ("wipe_left",     "Barrido ←",         "Básicas"),
        ("wipe_up",       "Barrido ↑",         "Básicas"),
        ("wipe_down",     "Barrido ↓",         "Básicas"),
        // Movimiento
        ("slide_right",   "Deslizar →",        "Movimiento"),
        ("slide_left",    "Deslizar ←",        "Movimiento"),
        ("slide_up",      "Deslizar ↑",        "Movimiento"),
        ("slide_down",    "Deslizar ↓",        "Movimiento"),
        ("push_right",    "Empujar →",         "Movimiento"),
        ("push_left",     "Empujar ←",         "Movimiento"),
        ("push_up",       "Empujar ↑",         "Movimiento"),
        ("push_down",     "Empujar ↓",         "Movimiento"),
        // Zoom
        ("zoom_in",       "Zoom +",            "Zoom"),
        ("zoom_out",      "Zoom −",            "Zoom"),
        ("zoom_in_out",   "Zoom +/−",          "Zoom"),
        ("zoom_out_in",   "Zoom −/+",          "Zoom"),
        ("zoom_blur",     "Zoom borroso",      "Zoom"),
        // Creativas
        ("rotate_cw",     "Rotar horario",     "Creativas"),
        ("rotate_ccw",    "Rotar antihorario", "Creativas"),
        ("flip_h",        "Voltear H",         "Creativas"),
        ("flip_v",        "Voltear V",         "Creativas"),
        ("cube",          "Cubo",              "Creativas"),
        ("page_turn",     "Vuelta de página",  "Creativas"),
        ("door_open",     "Puerta abierta",    "Creativas"),
        ("door_close",    "Puerta cerrada",    "Creativas"),
        ("ripple",        "Ondas",             "Creativas"),
        ("swirl",         "Espiral",           "Creativas"),
        ("mosaic",        "Mosaico",           "Creativas"),
        // Drift (firma CapCut)
        ("drift_right",   "Drift →",           "Drift"),
        ("drift_left",    "Drift ←",           "Drift"),
        ("drift_up",      "Drift ↑",           "Drift"),
        ("drift_down",    "Drift ↓",           "Drift"),
        ("drift_in",      "Drift +",           "Drift"),
        ("drift_out",     "Drift −",           "Drift"),
        // Impacto
        ("flash",         "Flash",             "Impacto"),
        ("shake",         "Sacudida",          "Impacto"),
        ("bounce",        "Rebote",            "Impacto"),
        ("smash",         "Golpe",             "Impacto"),
        ("elastic",       "Elástico",          "Impacto"),
        ("crash_zoom",    "Crash zoom",        "Impacto"),
        ("impact_blur",   "Blur impacto",      "Impacto"),
        // Glitch
        ("glitch",        "Glitch",            "Glitch"),
        ("rgb_split_tr",  "RGB Split",         "Glitch"),
        ("vhs_glitch",    "VHS Glitch",        "Glitch"),
        ("digital",       "Digital",           "Glitch"),
        ("flicker",       "Parpadeo",          "Glitch"),
        ("noise_wipe",    "Ruido",             "Glitch"),
        // Extra hasta 67
        ("iris_in",       "Iris +",            "Creativas"),
        ("iris_out",      "Iris −",            "Creativas"),
        ("star_wipe",     "Estrella",          "Creativas"),
        ("heart_wipe",    "Corazón",           "Creativas"),
        ("clock_wipe",    "Reloj",             "Creativas"),
        ("radial_wipe",   "Radial",            "Creativas"),
        ("barn_open",     "Granero abre",      "Creativas"),
        ("barn_close",    "Granero cierra",    "Creativas"),
        ("pinwheel",      "Molinete",          "Creativas"),
        ("checker",       "Tablero",           "Creativas"),
        ("pixelate_tr",   "Pixelar",           "Glitch"),
        ("liquid",        "Líquido",           "Creativas"),
        ("morph",         "Morph",             "Creativas"),
        ("light_leak",    "Fuga de luz",       "Creativas"),
        ("lens_flare",    "Destello",          "Creativas"),
        ("static_noise",  "Estática",          "Glitch"),
    ];

    // Categorías únicas en orden
    const CATS: &[&str] = &["Básicas", "Movimiento", "Zoom", "Creativas", "Drift", "Impacto", "Glitch"];

    // Control de categoría activa (reutilizamos filter_category)
    let active_cat = app.filter_category.clone().unwrap_or_else(|| "Básicas".to_string());

    ui.horizontal_wrapped(|ui| {
        for &cat in CATS {
            let sel = active_cat == cat;
            if ui.selectable_label(sel, cat).clicked() {
                app.filter_category = Some(cat.to_string());
            }
        }
    });
    ui.add_space(4.0);

    // Control de duración de transición (guardado en app.zoom reutilizado como duración, 0.3–2.0 s)
    let mut tr_dur = app.zoom.clamp(0.3, 2.0) as f32;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Duración:").color(theme::MUTED).size(11.0));
        if ui.add(egui::Slider::new(&mut tr_dur, 0.3f32..=2.0).suffix(" s").step_by(0.1)).changed() {
            app.zoom = tr_dur as f64;
        }
    });
    ui.add_space(4.0);

    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let tr_duration = app.zoom.clamp(0.3, 2.0);
            for &(tr_id, label, cat) in TRANSITIONS {
                if cat != active_cat { continue; }
                let btn = ui.add_sized(
                    egui::vec2(84.0, 50.0),
                    egui::Button::new(egui::RichText::new(label).size(10.0)),
                );
                if btn.clicked() {
                    if let Some(clip_id) = app.editor.selection().map(str::to_string) {
                        app.editor.mutate(|p| {
                            if let Some(clip) = p.find_clip_mut(&clip_id) {
                                clip.extra.insert(
                                    "transition".to_string(),
                                    serde_json::json!(tr_id),
                                );
                                clip.extra.insert(
                                    "transition_duration".to_string(),
                                    serde_json::json!(tr_duration),
                                );
                            }
                        });
                        app.save_current();
                        app.toast(format!("Transición: {label}"));
                    } else {
                        app.toast("Selecciona un clip primero");
                    }
                }
            }
        });
    });
}
