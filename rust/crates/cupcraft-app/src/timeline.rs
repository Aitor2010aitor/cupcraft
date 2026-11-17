//! Línea de tiempo interactiva (herramientas de corte CapCut, regla, pistas, clips, playhead, snap).

use crate::app::{CupApp, ToolMode};
use crate::format::format_time;
use crate::paint;
use crate::theme;
use cupcraft_core::ClipKind;
use eframe::egui;

const LABEL_W: f32 = 140.0;
const RULER_H: f32 = 26.0;
const ROW_H: f32 = 56.0;

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let project = match app.editor.project() {
        Some(p) => p.clone(),
        None => return,
    };

    // Barra superior de herramientas de la línea de tiempo (estilo CapCut)
    ui.horizontal(|ui| {
        ui.set_height(32.0);

        // Herramientas de cursor: Selección (V) y Cuchilla (B)
        let is_select = app.tool_mode == ToolMode::Select;
        if ui.selectable_label(is_select, "↖ V").on_hover_text("Herramienta Selección (V)").clicked() {
            app.tool_mode = ToolMode::Select;
        }
        let is_split = app.tool_mode == ToolMode::Split;
        if ui.selectable_label(is_split, "✂ B").on_hover_text("Cuchilla / Dividir (B)").clicked() {
            app.tool_mode = ToolMode::Split;
        }

        ui.separator();

        // Deshacer / Rehacer
        if ui.add_enabled(app.editor.can_undo(), egui::Button::new("↶")).on_hover_text("Deshacer (Ctrl+Z)").clicked() {
            if app.editor.undo() {
                app.save_current();
                app.toast("Deshecho");
            }
        }
        if ui.add_enabled(app.editor.can_redo(), egui::Button::new("↷")).on_hover_text("Rehacer (Ctrl+Y)").clicked() {
            if app.editor.redo() {
                app.save_current();
                app.toast("Rehecho");
            }
        }

        ui.separator();

        // Dividir en el cursor (Ctrl+B)
        if ui.button("✂ Dividir").on_hover_text("Dividir en el cabezal (Ctrl+B)").clicked() {
            let ph = app.playhead;
            let sel = app.editor.selection().map(|s| s.to_string());
            app.editor.mutate(|p| { p.split_at(ph, sel.as_deref()); });
            app.save_current();
            app.toast("Dividido");
        }

        // Eliminar a la izquierda del cabezal (Q)
        if ui.button("⇤ Izq").on_hover_text("Eliminar a la izquierda del cabezal (Q)").clicked() {
            let ph = app.playhead;
            let sel = app.editor.selection().map(|s| s.to_string());
            let ok = app.editor.mutate(|p| p.trim_left_at(ph, sel.as_deref())).unwrap_or(false);
            if ok {
                app.save_current();
                app.toast("Recortado inicio");
            }
        }

        // Eliminar a la derecha del cabezal (W)
        if ui.button("⇥ Der").on_hover_text("Eliminar a la derecha del cabezal (W)").clicked() {
            let ph = app.playhead;
            let sel = app.editor.selection().map(|s| s.to_string());
            let ok = app.editor.mutate(|p| p.trim_right_at(ph, sel.as_deref())).unwrap_or(false);
            if ok {
                app.save_current();
                app.toast("Recortado final");
            }
        }

        // Eliminar clip seleccionado (Supr)
        let has_sel = app.editor.selection().is_some();
        if ui.add_enabled(has_sel, egui::Button::new("🗑")).on_hover_text("Eliminar clip seleccionado (Supr)").clicked() {
            if let Some(id) = app.editor.selection().map(|s| s.to_string()) {
                app.editor.mutate(|p| { p.remove_clip(&id); });
                app.editor.select(None);
                app.save_current();
                app.toast("Clip eliminado");
            }
        }

        // Duplicar clip (Ctrl+D)
        if ui.add_enabled(has_sel, egui::Button::new("⧉")).on_hover_text("Duplicar clip (Ctrl+D)").clicked() {
            if let Some(id) = app.editor.selection().map(|s| s.to_string()) {
                let dup = app.editor.mutate(|p| p.duplicate_clip(&id)).flatten();
                if let Some(did) = dup {
                    app.editor.select(Some(did));
                    app.save_current();
                    app.toast("Clip duplicado");
                }
            }
        }

        ui.separator();

        // Herramientas de manipulación de clip
        if ui.add_enabled(has_sel, egui::Button::new("↻ 90°")).on_hover_text("Rotar clip 90°").clicked() {
            if let Some(id) = app.editor.selection().map(|s| s.to_string()) {
                app.editor.mutate(|p| { p.rotate_clip_90(&id); });
                app.save_current();
                app.toast("Rotado 90°");
            }
        }
        if ui.add_enabled(has_sel, egui::Button::new("⇄ Espejo")).on_hover_text("Efecto espejo (volteo horizontal)").clicked() {
            if let Some(id) = app.editor.selection().map(|s| s.to_string()) {
                app.editor.mutate(|p| { p.flip_clip_horizontal(&id); });
                app.save_current();
                app.toast("Volteado horizontal");
            }
        }
        if ui.add_enabled(has_sel, egui::Button::new("◀ Invertir")).on_hover_text("Invertir reproducción").clicked() {
            if let Some(id) = app.editor.selection().map(|s| s.to_string()) {
                app.editor.mutate(|p| { p.reverse_clip(&id); });
                app.save_current();
                app.toast("Reproducción invertida");
            }
        }

        ui.separator();

        // Marcador de ritmo (M)
        if ui.button("📍 Marcador").on_hover_text("Añadir o quitar marcador en el cabezal (M)").clicked() {
            let ph = app.playhead;
            let added = app.editor.mutate(|p| p.toggle_marker(ph)).unwrap_or(false);
            app.save_current();
            app.toast(if added { "Marcador añadido" } else { "Marcador eliminado" });
        }

        // Lado derecho de la barra de herramientas
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+").on_hover_text("Acercar zoom de tiempo").clicked() {
                app.zoom = (app.zoom * 1.25).min(8.0);
            }
            if ui.button("−").on_hover_text("Alejar zoom de tiempo").clicked() {
                app.zoom = (app.zoom / 1.25).max(0.2);
            }
            if ui.button("⤢").on_hover_text("Ajustar a la ventana (Shift+Z)").clicked() {
                let dur = project.timeline_length().max(1.0);
                let avail = (ui.available_width() - LABEL_W).max(100.0);
                app.zoom = ((avail / (dur as f32 * 80.0)) as f64).clamp(0.25, 8.0);
            }
            ui.separator();
            let snap_label = if app.snap { "🧲 Snap ON" } else { "🧲 Snap OFF" };
            if ui.selectable_label(app.snap, snap_label).on_hover_text("Imán de ajuste (N)").clicked() {
                app.snap = !app.snap;
            }
            ui.separator();
            for (label, kind) in [("+T", cupcraft_core::TrackType::Text), ("+A", cupcraft_core::TrackType::Audio), ("+V", cupcraft_core::TrackType::Video)] {
                if ui.small_button(label).on_hover_text(format!("Añadir pista de {:?}", kind)).clicked() {
                    app.editor.mutate(|p| { p.add_track(kind); });
                    app.save_current();
                }
            }
        });
    });

    ui.separator();

    // Área principal de la línea de tiempo (Regla + Pistas + Clips)
    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::BG2);

    let zoom = app.zoom.max(0.25);
    let px_per_sec = 80.0 * zoom as f32;
    let duration = project.timeline_length();
    let tracks = project.tracks.clone();

    // Regla de tiempo
    let ruler_top = rect.min.y;
    let ruler_rect = egui::Rect::from_min_size(egui::pos2(rect.min.x + LABEL_W, ruler_top), egui::vec2(rect.width() - LABEL_W, RULER_H));
    painter.rect_filled(ruler_rect, egui::CornerRadius::ZERO, theme::PANEL2);

    // Marcas de tiempo en la regla
    let mut t = 0.0;
    while t <= duration + 0.001 {
        let x = rect.min.x + LABEL_W + t as f32 * px_per_sec;
        if x > rect.max.x { break; }
        painter.line_segment([egui::pos2(x, ruler_top + RULER_H - 6.0), egui::pos2(x, ruler_top + RULER_H)], egui::Stroke::new(1.0, theme::MUTED));
        painter.text(egui::pos2(x + 2.0, ruler_top + 2.0), egui::Align2::LEFT_TOP, format_time(t), egui::FontId::monospace(9.0), theme::MUTED);
        t += 1.0;
    }

    // Dibujar marcadores en la regla (diamantes dorados)
    for &marker in &project.markers {
        let mx = rect.min.x + LABEL_W + marker as f32 * px_per_sec;
        if mx >= rect.min.x + LABEL_W && mx <= rect.max.x {
            painter.line_segment([egui::pos2(mx, ruler_top), egui::pos2(mx, rect.max.y)], egui::Stroke::new(1.0, theme::ACCENT2));
            painter.text(egui::pos2(mx, ruler_top + 10.0), egui::Align2::CENTER_CENTER, "◆", egui::FontId::proportional(11.0), theme::ACCENT2);
        }
    }

    // Pistas y clips
    let mut y = ruler_top + RULER_H;
    for track in &tracks {
        // Cabecera / Etiqueta de la pista
        let label_rect = egui::Rect::from_min_size(egui::pos2(rect.min.x, y), egui::vec2(LABEL_W, ROW_H));
        painter.rect_filled(label_rect, egui::CornerRadius::ZERO, theme::PANEL);
        painter.text(egui::pos2(rect.min.x + 8.0, y + 8.0), egui::Align2::LEFT_TOP, &track.name, egui::FontId::proportional(12.0), theme::TEXT);

        // Fondo de la fila de pista
        let row_rect = egui::Rect::from_min_size(egui::pos2(rect.min.x + LABEL_W, y), egui::vec2(rect.width() - LABEL_W, ROW_H));
        painter.rect_filled(row_rect, egui::CornerRadius::ZERO, theme::PANEL2);

        // Clips dentro de la pista
        for clip in &track.clips {
            let cx = rect.min.x + LABEL_W + clip.start as f32 * px_per_sec;
            let cw = (clip.duration as f32 * px_per_sec).max(4.0);
            let clip_rect = egui::Rect::from_min_size(egui::pos2(cx, y + 6.0), egui::vec2(cw, ROW_H - 12.0));
            let (top, bot, _border) = paint::clip_colors(clip.kind);
            paint::rounded_gradient(&painter, clip_rect, 6, top, bot);

            // Borde de selección
            let sel = app.editor.selection().map(|s| s == clip.id).unwrap_or(false);
            if sel {
                painter.rect_stroke(clip_rect, egui::CornerRadius::from(6), egui::Stroke::new(2.0, theme::ACCENT2), egui::StrokeKind::Inside);
            }

            // Nombre del clip
            let font = egui::FontId::proportional(11.0);
            let name = paint::ellipsize(&painter, &clip.name, &font, clip_rect.width() - 8.0);
            painter.text(egui::pos2(clip_rect.min.x + 4.0, clip_rect.min.y + 4.0), egui::Align2::LEFT_TOP, name, font, theme::TEXT);

            // Indicador de filtro aplicado
            let lk = cupcraft_core::filters::line_key_for_clip(&clip.filter, clip.gl_filter.as_deref(), clip.lut.as_deref());
            if lk.is_some() {
                let ir = egui::Rect::from_min_size(egui::pos2(clip_rect.min.x + 4.0, clip_rect.max.y - 14.0), egui::vec2(clip_rect.width() - 8.0, 8.0));
                painter.rect_filled(ir, egui::CornerRadius::from(3), theme::ACCENT2);
            }
        }
        y += ROW_H;
    }

    // Cabezal de reproducción (Playhead)
    let phx = rect.min.x + LABEL_W + app.playhead as f32 * px_per_sec;
    if phx >= rect.min.x + LABEL_W && phx <= rect.max.x {
        painter.line_segment([egui::pos2(phx, ruler_top), egui::pos2(phx, rect.max.y)], egui::Stroke::new(1.5, theme::PLAYHEAD));
        let handle = egui::Rect::from_min_size(egui::pos2(phx - 6.0, ruler_top - 2.0), egui::vec2(12.0, 10.0));
        painter.rect_filled(handle, egui::CornerRadius::from(2), theme::PLAYHEAD);
    }

    // Interacción con ratón: Selección, corte con cuchilla o mover playhead
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.y < ruler_top + RULER_H && pos.x > rect.min.x + LABEL_W {
                let t = ((pos.x - (rect.min.x + LABEL_W)) / px_per_sec).max(0.0) as f64;
                app.playhead = t;
            } else {
                let mut found = None;
                let mut yy = ruler_top + RULER_H;
                for track in &tracks {
                    for clip in &track.clips {
                        let cx = rect.min.x + LABEL_W + clip.start as f32 * px_per_sec;
                        let cw = (clip.duration as f32 * px_per_sec).max(4.0);
                        let cr = egui::Rect::from_min_size(egui::pos2(cx, yy + 6.0), egui::vec2(cw, ROW_H - 12.0));
                        if cr.contains(pos) {
                            found = Some((clip.id.clone(), ((pos.x - (rect.min.x + LABEL_W)) / px_per_sec) as f64));
                            break;
                        }
                    }
                    if found.is_some() { break; }
                    yy += ROW_H;
                }

                if let Some((clip_id, click_time)) = found {
                    if app.tool_mode == ToolMode::Split {
                        // Modo Cuchilla (B): divide el clip exactamente donde se hizo clic
                        app.editor.mutate(|p| { p.split_at(click_time, Some(&clip_id)); });
                        app.save_current();
                        app.toast("Clip dividido con cuchilla");
                    } else {
                        app.editor.select(Some(clip_id));
                    }
                } else {
                    app.editor.select(None);
                }
            }
        }
    }

    // Arrastrar el cabezal de reproducción
    if resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.x > rect.min.x + LABEL_W {
                let t = ((pos.x - (rect.min.x + LABEL_W)) / px_per_sec).max(0.0) as f64;
                app.playhead = t;
            }
        }
    }
}
