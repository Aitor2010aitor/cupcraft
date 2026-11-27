//! Panel derecho "Detalles" (al estilo CapCut): ficha del proyecto cuando no
//! hay selección o del clip seleccionado. Los clips de texto usan las
//! pestañas de `inspector_text` (Texto · Animación · Seguimiento · Texto a
//! voz · Avatares de IA), igual que `TEXT_DETAIL_TABS` del original.
//!
//! El ajuste de color (brillo/contraste/saturación/...) vive en la pestaña
//! izquierda "Ajustar" (`panels.rs::adjust_panel`), igual que en CapCut: su
//! panel de "Detalles" de la derecha es informativo, no donde se gradúa el
//! color.

use crate::app::CupApp;
use crate::format::format_time;
use crate::theme;
use cupcraft_core::ClipKind;
use eframe::egui;
use serde_json::json;

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let sel = app.editor.selection().map(str::to_string);
    match sel {
        Some(id) => clip_inspector(ui, app, &id),
        None => project_inspector(ui, app),
    }
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::i18n::tr(&label)).color(theme::MUTED).size(11.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr(&value)).color(theme::TEXT).size(11.0));
        });
    });
}

// ---------------- Proyecto (sin selección) ----------------
fn project_inspector(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.label(egui::RichText::new(crate::i18n::tr("Proyecto")).color(theme::TEXT).strong());
    ui.add_space(6.0);

    let snapshot = app.editor.project().map(|p| {
        (
            p.name.clone(),
            p.ratio.clone(),
            p.resolution.clone(),
            p.fps,
            p.timeline_length(),
            p.media.len(),
            p.tracks.len(),
            p.tracks.iter().map(|t| t.clips.len()).sum::<usize>(),
        )
    });

    if let Some((name0, ratio, resolution, fps, duration, media_count, track_count, clip_count)) = snapshot {
        ui.label(egui::RichText::new(crate::i18n::tr("Nombre")).color(theme::MUTED).size(11.0));
        let mut name = name0.clone();
        if ui.text_edit_singleline(&mut name).changed() && name != name0 {
            if let Some(p) = app.editor.project_mut() {
                p.name = name;
                p.updated_at = cupcraft_core::now_millis();
            }
            app.save_current();
        }

        ui.add_space(10.0);
        info_row(ui, "Resolución", &resolution);
        info_row(ui, "Proporción", &ratio);
        info_row(ui, "FPS", &fps.to_string());
        info_row(ui, "Duración", &format_time(duration));
        info_row(ui, "Medios", &media_count.to_string());
        info_row(ui, "Pistas", &track_count.to_string());
        info_row(ui, "Clips", &clip_count.to_string());
    } else {
        ui.label(egui::RichText::new(crate::i18n::tr("Sin proyecto abierto.")).color(theme::MUTED).size(11.0));
    }

    ui.add_space(14.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(crate::i18n::tr("Selecciona un clip en la línea de tiempo para ver y editar sus propiedades aquí."))
            .color(theme::MUTED2)
            .size(11.0),
    );
}

// ---------------- Clip (con selección) ----------------
fn clip_inspector(ui: &mut egui::Ui, app: &mut CupApp, id: &str) {
    let snapshot = app
        .editor
        .project()
        .and_then(|p| p.find_clip(id).map(|(t, c)| (t.name.clone(), c.clone())));
    let Some((track_name, clip)) = snapshot else {
        project_inspector(ui, app);
        return;
    };
    if clip.kind == ClipKind::Text {
        crate::inspector_text::show(ui, app, &clip);
        return;
    }
    crate::inspector_clip::show(ui, app, &clip, &track_name);
}

/// Parche de animación de texto (`data-detail-text-anim` del original).
pub fn anim_patch(a: &serde_json::Value) -> serde_json::Value {
    json!({
        "animation": a.get("id").cloned().unwrap_or(json!("none")),
        "animationPhase": a.get("phase").cloned().unwrap_or(json!("in")),
        "animationDuration": a.get("duration").cloned().unwrap_or(json!(0.6)),
    })
}
