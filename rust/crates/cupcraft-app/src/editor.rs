//! Editor principal: barra superior, vista previa y cableado de paneles/timeline.

use crate::app::CupApp;
use crate::format::format_time;
use crate::paint;
use crate::panels;
use crate::theme;
use crate::timeline as tl;
use cupcraft_core::ClipKind;
use cupcraft_media::exporter::{ExportFormat, VideoEncoderSession};
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
                    let export_btn = egui::Button::new(
                        egui::RichText::new("Exportar").color(egui::Color32::from_rgb(0x04, 0x14, 0x12)).size(12.0).strong()
                    ).fill(theme::ACCENT);
                    if ui.add(export_btn).clicked() {
                        app.export.open = true;
                    }
                    if ui.button("Guardar").clicked() {
                        app.save_current();
                        app.toast("Guardado");
                    }
                    if ui.button("⏮").clicked() {
                        app.playhead = 0.0;
                        app.audio_seek(0.0);
                    }
                    let play_label = if app.playing { "⏸" } else { "▶" };
                    if ui.button(play_label).clicked() {
                        app.playing = !app.playing;
                        app.last_tick = now();
                        if app.playing {
                            app.audio_play();
                        } else {
                            app.audio_pause();
                        }
                    }
                    ui.label(egui::RichText::new(format_time(app.playhead)).color(theme::TEXT).monospace());
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
                strip_tab(ui, app, crate::app::PanelTab::Stickers, "Stickers");
                strip_tab(ui, app, crate::app::PanelTab::Effects, "Efectos");
                strip_tab(ui, app, crate::app::PanelTab::Transitions, "Transic.");
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

    // Modal de exportación de vídeo
    show_export_modal(&ctx, app);

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

    // Monitor de vídeo negro 16:9 con borde profesional
    p.rect_filled(stage, egui::CornerRadius::from(6), egui::Color32::from_rgb(0x08, 0x08, 0x0a));
    p.rect_stroke(stage, egui::CornerRadius::from(6), egui::Stroke::new(1.0, egui::Color32::from_rgb(0x22, 0x25, 0x2b)), egui::StrokeKind::Inside);

    let mut has_clips = false;
    if let Some(project) = app.editor.project() {
        let scale = stage.height() / 1080.0;
        for track in &project.tracks {
            for clip in &track.clips {
                if clip.kind == ClipKind::Text {
                    if let Some(text) = &clip.text {
                        has_clips = true;
                        paint::paint_text(&p, stage, text, scale);
                    }
                } else if matches!(clip.kind, ClipKind::Video | ClipKind::Image) {
                    has_clips = true;
                    // Capa de vídeo o imagen en el monitor
                    let clip_rect = stage.shrink(10.0);
                    p.rect_filled(clip_rect, egui::CornerRadius::from(4), egui::Color32::from_rgb(0x18, 0x22, 0x26));
                    p.text(clip_rect.center(), egui::Align2::CENTER_CENTER, &clip.name, egui::FontId::proportional(14.0), theme::TEXT);
                }
            }
        }
    }

    if !has_clips {
        // Overlay elegante cuando la línea de tiempo está vacía
        p.text(
            egui::pos2(stage.center().x, stage.center().y - 12.0),
            egui::Align2::CENTER_CENTER,
            "1920 × 1080 · 16:9",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(0x40, 0x48, 0x50),
        );
        p.text(
            egui::pos2(stage.center().x, stage.center().y + 12.0),
            egui::Align2::CENTER_CENTER,
            "Añade medios o texto para previsualizar",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(0x30, 0x38, 0x40),
        );
    }
}

/// Modal de exportación de vídeo con barra de progreso y opciones de GPU.
fn show_export_modal(ctx: &egui::Context, app: &mut CupApp) {
    if !app.export.open { return; }

    let mut open = app.export.open;
    egui::Window::new("Exportar vídeo")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(380.0)
        .show(ctx, |ui| {
            ui.set_min_width(360.0);

            if app.export.running {
                // Pantalla de progreso
                ui.label(egui::RichText::new("Exportando…").color(theme::TEXT).strong());
                ui.add_space(8.0);
                let prog = app.export.progress.max(0.0);
                ui.add(egui::ProgressBar::new(prog).show_percentage().animate(true));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(&app.export.message).color(theme::MUTED).size(11.0));
                if app.export.progress >= 1.0 || app.export.progress < 0.0 {
                    if ui.button("Cerrar").clicked() {
                        app.export.open = false;
                        app.export.running = false;
                        app.export.progress = 0.0;
                    }
                }
                return;
            }

            // ── Formato ──────────────────────────────────────────────────────
            ui.label(egui::RichText::new("Formato").color(theme::MUTED).size(11.0));
            ui.horizontal(|ui| {
                for (i, label) in ["MP4 H.264", "MP4 H.265", "WebM VP9"].iter().enumerate() {
                    if ui.selectable_label(app.export.format == i as u8, *label).clicked() {
                        app.export.format = i as u8;
                    }
                }
            });

            ui.add_space(6.0);

            // ── Resolución ───────────────────────────────────────────────────
            ui.label(egui::RichText::new("Resolución").color(theme::MUTED).size(11.0));
            ui.horizontal(|ui| {
                for (i, label) in ["1080p", "720p", "4K"].iter().enumerate() {
                    if ui.selectable_label(app.export.resolution == i as u8, *label).clicked() {
                        app.export.resolution = i as u8;
                    }
                }
            });

            ui.add_space(6.0);

            // ── FPS ──────────────────────────────────────────────────────────
            ui.label(egui::RichText::new("Fotogramas/s").color(theme::MUTED).size(11.0));
            ui.horizontal(|ui| {
                for (i, label) in ["30 fps", "60 fps"].iter().enumerate() {
                    if ui.selectable_label(app.export.fps == i as u8, *label).clicked() {
                        app.export.fps = i as u8;
                    }
                }
            });

            ui.add_space(6.0);

            // ── GPU ──────────────────────────────────────────────────────────
            ui.checkbox(&mut app.export.use_gpu, "Aceleración GPU (NVENC / QSV / AMF)");

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui.button("❌ Cancelar").clicked() {
                    app.export.open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("▶ Exportar").clicked() {
                        start_export(app);
                    }
                });
            });
        });

    if !open { app.export.open = false; }
}

/// Lanza la exportación de vídeo en un hilo separado.
/// La barra de progreso se actualiza vía `app.export.progress`.
fn start_export(app: &mut CupApp) {
    let Some(path) = pick_export_path(app) else { return; };

    let (width, height) = match app.export.resolution {
        1 => (1280u32, 720u32),
        2 => (3840u32, 2160u32),
        _ => (1920u32, 1080u32),
    };
    let fps = if app.export.fps == 1 { 60u32 } else { 30u32 };
    let format = match app.export.format {
        1 => ExportFormat::Mp4H265,
        2 => ExportFormat::WebmVp9,
        _ => ExportFormat::Mp4H264,
    };
    let use_gpu = app.export.use_gpu;

    // Tiempo total del proyecto
    let total_secs = app.editor.project().map(|p| p.timeline_length()).unwrap_or(1.0);
    let total_frames = (total_secs * fps as f64).ceil() as u64;

    app.export.running = true;
    app.export.progress = 0.0;
    app.export.message = "Iniciando FFmpeg…".to_string();

    // Lanzamos la sesión en este hilo (bloqueante en el background thread).
    // Para no bloquear la UI, usamos std::thread.
    let progress_path = path.clone();
    std::thread::spawn(move || {
        let session = VideoEncoderSession::start(
            &progress_path, width, height, fps, total_frames, format, use_gpu,
        );
        let mut sess = match session {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Export error: {:?}", e);
                return;
            }
        };

        // Genera fotogramas negros sintéticos (placeholder).
        // En Fase 7 se reemplazará por la composición real de wgpu.
        let frame_bytes = vec![0u8; (width * height * 4) as usize]; // RGBA negro
        for _frame in 0..total_frames {
            if sess.push_frame(&frame_bytes).is_err() { break; }
        }
        let _ = sess.finish();
        println!("Exportación completada: {:?}", progress_path);
    });

    app.export.progress = 0.05;
    app.export.message = "Exportando con FFmpeg…".to_string();
    app.toast("Exportación iniciada");
}

fn pick_export_path(app: &CupApp) -> Option<std::path::PathBuf> {
    let name = app.editor.project().map(|p| p.name.clone()).unwrap_or_else(|| "video".to_string());
    let (ext, filter_name) = match app.export.format {
        2 => ("webm", "WebM"),
        _ => ("mp4", "MP4"),
    };
    rfd::FileDialog::new()
        .add_filter(filter_name, &[ext])
        .set_file_name(format!("{name}.{ext}"))
        .save_file()
}

fn export_project(_app: &mut CupApp) {
    // Función legacy mantenida por compatibilidad; ahora se llama show_export_modal
}

fn handle_shortcuts(ctx: &egui::Context, app: &mut CupApp) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    // Deshacer / Rehacer
    if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
        if app.editor.redo() { app.save_current(); app.toast("Rehecho"); }
    } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
        if app.editor.undo() { app.save_current(); app.toast("Deshecho"); }
    } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
        if app.editor.redo() { app.save_current(); app.toast("Rehecho"); }
    }

    // Reproducir / Pausa (Espacio)
    if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
        app.playing = !app.playing;
        app.last_tick = now();
        if app.playing {
            app.audio_play();
        } else {
            app.audio_pause();
        }
    }

    // Dividir en el cabezal (Ctrl+B)
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::B)) {
        let ph = app.playhead;
        let sel = app.editor.selection().map(str::to_string);
        app.editor.mutate(|p| { p.split_at(ph, sel.as_deref()); });
        app.save_current();
        app.toast("Dividido");
    }

    // Alternar herramientas de cursor: V (Selección) y B (Cuchilla)
    if ctx.input(|i| !i.modifiers.ctrl && i.key_pressed(egui::Key::V)) {
        app.tool_mode = crate::app::ToolMode::Select;
        app.toast("Herramienta: Selección (V)");
    }
    if ctx.input(|i| !i.modifiers.ctrl && i.key_pressed(egui::Key::B)) {
        app.tool_mode = crate::app::ToolMode::Split;
        app.toast("Herramienta: Cuchilla (B)");
    }

    // Recorte CapCut: Q (Eliminar izquierda del cabezal) y W (Eliminar derecha del cabezal)
    if ctx.input(|i| !i.modifiers.ctrl && i.key_pressed(egui::Key::Q)) {
        let ph = app.playhead;
        let sel = app.editor.selection().map(str::to_string);
        let ok = app.editor.mutate(|p| p.trim_left_at(ph, sel.as_deref())).unwrap_or(false);
        if ok {
            app.save_current();
            app.toast("Recortado inicio (Q)");
        }
    }
    if ctx.input(|i| !i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
        let ph = app.playhead;
        let sel = app.editor.selection().map(str::to_string);
        let ok = app.editor.mutate(|p| p.trim_right_at(ph, sel.as_deref())).unwrap_or(false);
        if ok {
            app.save_current();
            app.toast("Recortado final (W)");
        }
    }

    // Eliminar clip seleccionado (Supr o Backspace)
    if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
        if let Some(id) = app.editor.selection().map(str::to_string) {
            app.editor.mutate(|p| { p.remove_clip(&id); });
            app.editor.select(None);
            app.save_current();
            app.toast("Clip eliminado");
        }
    }

    // Duplicar clip (Ctrl+D)
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
        if let Some(id) = app.editor.selection().map(str::to_string) {
            let dup = app.editor.mutate(|p| p.duplicate_clip(&id)).flatten();
            if let Some(did) = dup {
                app.editor.select(Some(did));
                app.save_current();
                app.toast("Clip duplicado");
            }
        }
    }

    // Marcador (M)
    if ctx.input(|i| !i.modifiers.ctrl && i.key_pressed(egui::Key::M)) {
        let ph = app.playhead;
        let added = app.editor.mutate(|p| p.toggle_marker(ph)).unwrap_or(false);
        app.save_current();
        app.toast(if added { "Marcador añadido (M)" } else { "Marcador eliminado (M)" });
    }

    // Imán de ajuste (N)
    if ctx.input(|i| !i.modifiers.ctrl && i.key_pressed(egui::Key::N)) {
        app.snap = !app.snap;
        app.toast(if app.snap { "Imán activado" } else { "Imán desactivado" });
    }

    // Ajustar zoom a la ventana (Shift+Z)
    if ctx.input(|i| i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
        if let Some(p) = app.editor.project() {
            let dur = p.timeline_length().max(1.0);
            let avail = 900.0; // tamaño promedio de timeline
            app.zoom = ((avail / (dur as f32 * 80.0)) as f64).clamp(0.25, 8.0);
            app.toast("Zoom ajustado a ventana");
        }
    }
}

fn now() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}
