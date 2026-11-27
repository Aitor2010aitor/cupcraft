//! Línea de tiempo nativa: réplica de `src/editor/timeline.js` + `.timeline*` de `styles.css`.
//!
//! Barra de herramientas con los mismos iconos y acciones, etiquetas de pista
//! (icono, nombre, ocultar/silenciar, bloquear), fila «Efectos», fila «Filtros»
//! (bajo la última pista de vídeo, con editor de intensidad), regla adaptativa,
//! clips con miniatura, sombreado, onda de audio, vista previa de texto/sticker,
//! rombos de fotograma clave, insignia «fx», tiradores de recorte, arrastre entre
//! pistas con imán (guía amarilla), caja de selección, marcadores y cabezal.

use crate::app::{CupApp, PanelTab, ToolMode};
use crate::format::format_time;
use crate::{icons, paint, theme};
use cupcraft_core::{tracks_compatible, Clip, ClipKind, Project, TrackType, TrimEdge};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

const LABEL_W: f32 = 140.0;
const LABEL_W_EMPTY: f32 = 64.0;
const RULER_H: f32 = 26.0;
const RULER_H_EMPTY: f32 = 20.0;
const ROW_H: f32 = 56.0;
const ADD_ROW_H: f32 = 38.0;
const TOOLBAR_H: f32 = 34.0;
const HANDLE_W: f32 = 7.0;
const PX_BASE: f32 = 80.0; // app.zoom = 1 → 80 px/s
const PX_MIN: f32 = 20.0;
const PX_MAX: f32 = 320.0;
const SCROLLBAR: f32 = 8.0;

fn c(hex: &str) -> Color32 {
    paint::hex(hex)
}

// ───────────────────────────── estado entre fotogramas ─────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    Move,
    Trim(TrimEdge),
}

#[derive(Clone)]
struct ClipDrag {
    clip_id: String,
    mode: DragMode,
    origin: Pos2,
    orig_start: f64,
    orig_dur: f64,
    orig_in: f64,
    kind: TrackType,
    media_dur: Option<f64>,
    moved: bool,
}

#[derive(Clone, Default)]
struct TlState {
    scroll_x: f32,
    scroll_y: f32,
    drag: Option<ClipDrag>,
    marquee: Option<(Pos2, Pos2)>,
    marquee_base: Vec<String>,
    multi: Vec<String>,
    guide: Option<f64>,
    drag_over: Option<String>,
    filter_editor: Option<(String, Pos2)>,
    filter_snapshot: bool,
    hscroll_drag: Option<(f32, f32)>,
    vscroll_drag: Option<(f32, f32)>,
    /// Arrastre del borde de una transición: (clip, inicio del clip, máximo).
    trans_drag: Option<(String, f64, f64)>,
    /// Transición agarrada por el cuerpo (id del clip, x del puntero): se
    /// suelta en el corte más cercano.
    trans_move: Option<(String, f32)>,
    /// Clip sobre el que se abrió el menú de clic derecho.
    ctx_clip: Option<String>,
    /// Portapapeles de clips (Copiar / Cortar / Pegar del menú).
    clipboard: Option<Clip>,
}

/// Clip de vídeo/imagen cuyo inicio (corte) queda más cerca de `t`.
fn nearest_cut(project: &Project, t: f64) -> Option<(String, f64)> {
    project
        .tracks
        .iter()
        .filter(|tr| !tr.locked)
        .flat_map(|tr| tr.clips.iter())
        .filter(|c| matches!(c.kind, ClipKind::Video | ClipKind::Image))
        .min_by(|a, b| (a.start - t).abs().total_cmp(&(b.start - t).abs()))
        .map(|c| (c.id.clone(), c.start))
}

fn state_id() -> egui::Id {
    egui::Id::new("cupcraft-timeline-state")
}

fn load_state(ctx: &egui::Context) -> TlState {
    ctx.data(|d| d.get_temp::<TlState>(state_id())).unwrap_or_default()
}

fn store_state(ctx: &egui::Context, st: TlState) {
    ctx.data_mut(|d| d.insert_temp(state_id(), st));
}

// ───────────────────────────── filas ─────────────────────────────

#[derive(Clone)]
enum Row {
    Fx,
    Track(usize),
    Filters,
    /// Capa de transiciones (una pieza por corte, como los filtros).
    Transitions,
}

fn build_rows(project: &Project, has_filters: bool, has_trans: bool) -> Vec<Row> {
    let mut rows = vec![Row::Fx];
    let last_video = project.tracks.iter().rposition(|t| t.kind == TrackType::Video);
    for i in 0..project.tracks.len() {
        rows.push(Row::Track(i));
        if Some(i) == last_video {
            if has_trans {
                rows.push(Row::Transitions);
            }
            if has_filters {
                rows.push(Row::Filters);
            }
        }
    }
    if last_video.is_none() {
        if has_trans {
            rows.push(Row::Transitions);
        }
        if has_filters {
            rows.push(Row::Filters);
        }
    }
    rows
}

/// Filtro activo de un clip (`activeFilter` del original): (tipo, nombre, intensidad).
fn active_filter(clip: &Clip) -> Option<(&'static str, String, f64)> {
    if clip.gl_filter.is_some() {
        return Some(("gl", clip.gl_filter_name.clone().unwrap_or_else(|| "Filtro".into()), clip.gl_filter_amount.unwrap_or(1.0)));
    }
    if clip.lut.is_some() {
        return Some(("lut", clip.lut_name.clone().unwrap_or_else(|| "Filtro".into()), clip.lut_amount.unwrap_or(1.0)));
    }
    if !clip.filter.is_empty() && clip.filter != "none" {
        let label = cupcraft_core::filter_by_key(&clip.filter).map(|f| f.label.to_string()).unwrap_or_else(|| clip.filter.clone());
        return Some(("css", label, 1.0));
    }
    None
}

fn track_meta(kind: TrackType) -> (&'static str, &'static str) {
    match kind {
        TrackType::Text => ("Añade texto desde la pestaña Texto", "text"),
        TrackType::Video => ("Arrastra tus videos e imágenes aquí", "film"),
        TrackType::Audio => ("Arrastra música o audio aquí", "music"),
        TrackType::Overlay => ("Capa extra: stickers, texto o efectos", "layers"),
    }
}

fn effect_clips(project: &Project) -> Vec<(String, String, f64, f64)> {
    project
        .extra
        .get("effectClips")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|fx| {
                    Some((
                        fx.get("id")?.as_str()?.to_string(),
                        fx.get("name").and_then(Value::as_str).unwrap_or("Efecto").to_string(),
                        fx.get("start").and_then(Value::as_f64).unwrap_or(0.0),
                        fx.get("duration").and_then(Value::as_f64).unwrap_or(1.0),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn remove_effect_clip(app: &mut CupApp, id: &str) {
    let id = id.to_string();
    app.editor.mutate(move |p| {
        if let Some(Value::Array(arr)) = p.extra.get_mut("effectClips") {
            arr.retain(|fx| fx.get("id").and_then(Value::as_str) != Some(id.as_str()));
        }
    });
    app.save_current();
    app.toast("Efecto quitado");
}

fn clear_clip_filter(app: &mut CupApp, id: &str) {
    let id = id.to_string();
    app.editor.mutate(move |p| {
        if let Some(cl) = p.find_clip_mut(&id) {
            cl.filter = "none".into();
            cl.lut = None;
            cl.lut_name = None;
            cl.lut_amount = Some(1.0);
            cl.gl_filter = None;
            cl.gl_filter_name = None;
            cl.gl_filter_amount = Some(1.0);
        }
    });
    app.save_current();
    app.toast("Filtro quitado del clip");
}

// ───────────────────────────── onda de audio (caché) ─────────────────────────────

const WAVE_PPS: usize = 100;

fn wave_cache() -> &'static Mutex<HashMap<String, Arc<Vec<f32>>>> {
    static W: OnceLock<Mutex<HashMap<String, Arc<Vec<f32>>>>> = OnceLock::new();
    W.get_or_init(|| Mutex::new(HashMap::new()))
}

fn wave_for(app: &CupApp, media_id: &str) -> Option<Arc<Vec<f32>>> {
    let mut map = wave_cache().lock().ok()?;
    if let Some(w) = map.get(media_id) {
        return Some(w.clone());
    }
    let buf = app.audio_buffers.get(media_id)?;
    let wf = cupcraft_audio::Waveform::from_buffer(buf, WAVE_PPS);
    let peaks: Vec<f32> = wf.points.iter().map(|p| p.min.abs().max(p.max.abs())).collect();
    let peak = peaks.iter().cloned().fold(0.0f32, f32::max).max(0.0001);
    let arc = Arc::new(peaks.into_iter().map(|v| v / peak).collect::<Vec<_>>());
    map.insert(media_id.to_string(), arc.clone());
    Some(arc)
}

// ───────────────────────────── miniaturas ─────────────────────────────

fn clip_thumb(app: &mut CupApp, ctx: &egui::Context, project: &Project, clip: &Clip) -> Option<(egui::TextureId, Vec2)> {
    let mid = clip.media_id.as_deref()?;
    let m = project.media.iter().find(|m| m.id == mid)?;
    let kind = m.kind.as_deref().unwrap_or("");
    if kind != "image" && kind != "video" {
        return None;
    }
    let key = format!("thumb:{}", m.id);
    if let Some(t) = app.video.texture(ctx, &key) {
        return Some(t);
    }
    if let Some(path) = app.media_path(m) {
        if kind == "image" {
            app.video.request_image(&key, &path);
        } else {
            let (w, h) = crate::services::video::fit_dims(crate::services::media::media_dims(m), 192, 108);
            let t = m.duration.unwrap_or(1.0).min(1.0) * 0.5;
            app.video.request_video(&key, &path, t, w, h, false);
        }
    }
    None
}

// ───────────────────────────── barra de herramientas ─────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum BtnStyle {
    Plain,
    Cyan,
}

fn tl_sep(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(9.0, 16.0), Sense::hover());
    ui.painter().rect_filled(Rect::from_center_size(r.center(), Vec2::new(1.0, 16.0)), 0.0, c("#3d3d3d"));
}

fn tl_btn(ui: &mut egui::Ui, icon: &str, size: f32, tip: &str, enabled: bool, active: bool, style: BtnStyle) -> bool {
    tl_btn_w(ui, icon, size, 26.0, tip, enabled, active, style, false)
}

#[allow(clippy::too_many_arguments)]
fn tl_btn_w(ui: &mut egui::Ui, icon: &str, size: f32, w: f32, tip: &str, enabled: bool, active: bool, style: BtnStyle, chevron: bool) -> bool {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 26.0), sense);
    let hovered = enabled && resp.hovered();
    let p = ui.painter();
    let (bg, fg) = match style {
        BtnStyle::Cyan => {
            let bg = if hovered { c("#00d3e0") } else if active { theme::ACCENT2 } else { theme::ACCENT };
            (Some(bg), c("#073b3e"))
        }
        BtnStyle::Plain => {
            let bg = if hovered { Some(c("#363636")) } else { None };
            let fg = if !enabled {
                c("#b5b5b5").gamma_multiply(0.35)
            } else if active {
                theme::ACCENT
            } else if hovered {
                Color32::WHITE
            } else {
                c("#b5b5b5")
            };
            (bg, fg)
        }
    };
    if let Some(bg) = bg {
        p.rect_filled(rect, 4.0, bg);
    }
    if chevron {
        icons::paint(p, icon, Pos2::new(rect.center().x - 4.0, rect.center().y), size, fg);
        icons::paint(p, "chevronDown", Pos2::new(rect.center().x + 7.0, rect.center().y), 10.0, fg);
    } else {
        icons::paint(p, icon, rect.center(), size, fg);
    }
    let resp = if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    let resp = resp.on_hover_text(tip);
    enabled && resp.clicked()
}

fn zoom_slider(ui: &mut egui::Ui, app: &mut CupApp) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(92.0, 26.0), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(88.0, 3.0));
    let px = (PX_BASE * app.zoom as f32).clamp(PX_MIN, PX_MAX);
    let frac = (px - PX_MIN) / (PX_MAX - PX_MIN);
    let split = track.left() + track.width() * frac;
    let p = ui.painter();
    p.rect_filled(track, 3.0, c("#4a4a4a"));
    p.rect_filled(Rect::from_min_max(track.min, Pos2::new(split, track.max.y)), 3.0, c("#b9b9b9"));
    p.circle_filled(Pos2::new(split, track.center().y), 4.5, c("#e2e2e2"));
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let npx = PX_MIN + f * (PX_MAX - PX_MIN);
            app.zoom = (npx / PX_BASE) as f64;
        }
    }
    resp.on_hover_text(crate::i18n::tr("Zoom de la línea de tiempo"));
}

fn set_px(app: &mut CupApp, px: f32) {
    app.zoom = (px.clamp(PX_MIN, PX_MAX) / PX_BASE) as f64;
}

fn selected_id(app: &CupApp) -> Option<String> {
    app.editor.selection().map(|s| s.to_string())
}

fn need_selection(app: &mut CupApp) -> Option<String> {
    let id = selected_id(app);
    if id.is_none() {
        app.toast("Selecciona un clip primero");
    }
    id
}

fn toolbar(ui: &mut egui::Ui, app: &mut CupApp, st: &mut TlState, project: &Project, view_w: f32) {
    let has_sel = app.editor.selection().is_some() || !st.multi.is_empty();
    ui.spacing_mut().item_spacing.x = 1.0;
    ui.add_space(8.0);

    if tl_btn(ui, "plus", 17.0, "Añadir medios", true, false, BtnStyle::Plain) {
        app.import_media_dialog();
    }
    tl_sep(ui);
    let split_mode = app.tool_mode == ToolMode::Split;
    if tl_btn_w(ui, if split_mode { "scissors" } else { "cursor" }, 15.0, 36.0, "Seleccionar (V)", true, false, BtnStyle::Plain, true) {
        if split_mode {
            app.tool_mode = ToolMode::Select;
        }
        app.toast("Arrastra sobre la línea de tiempo para seleccionar varios clips (Supr los borra)");
    }
    if tl_btn(ui, "split", 16.0, "Dividir en el cursor (B / Ctrl+B)", true, false, BtnStyle::Plain) {
        let ph = app.playhead;
        let sel = selected_id(app);
        let n = app.editor.mutate(|p| p.split_at(ph, sel.as_deref())).unwrap_or(0);
        if n > 0 {
            app.save_current();
            app.toast("Clip dividido");
        } else {
            app.toast("Coloca el cursor sobre un clip para dividirlo");
        }
    }
    if tl_btn(ui, "trimLeft", 16.0, "Recortar inicio al cursor (Q)", true, false, BtnStyle::Plain) {
        let ph = app.playhead;
        let sel = selected_id(app);
        if app.editor.mutate(|p| p.trim_left_at(ph, sel.as_deref())).unwrap_or(false) {
            app.save_current();
            app.toast("Inicio recortado");
        } else {
            app.toast("Coloca el cursor sobre un clip");
        }
    }
    if tl_btn(ui, "trimRight", 16.0, "Recortar final al cursor (W)", true, false, BtnStyle::Plain) {
        let ph = app.playhead;
        let sel = selected_id(app);
        if app.editor.mutate(|p| p.trim_right_at(ph, sel.as_deref())).unwrap_or(false) {
            app.save_current();
            app.toast("Final recortado");
        } else {
            app.toast("Coloca el cursor sobre un clip");
        }
    }
    tl_sep(ui);
    if tl_btn(ui, "undo", 16.0, "Deshacer (Ctrl+Z)", app.editor.can_undo(), false, BtnStyle::Plain) && app.editor.undo() {
        app.save_current();
        app.toast("Deshecho");
    }
    if tl_btn(ui, "redo", 16.0, "Rehacer (Ctrl+Y)", app.editor.can_redo(), false, BtnStyle::Plain) && app.editor.redo() {
        app.save_current();
        app.toast("Rehecho");
    }
    if tl_btn(ui, "copy", 15.0, "Duplicar (Ctrl+D)", has_sel, false, BtnStyle::Plain) {
        if let Some(id) = selected_id(app) {
            if let Some(Some(did)) = app.editor.mutate(|p| p.duplicate_clip(&id)) {
                app.editor.select(Some(did));
                app.save_current();
                app.toast("Clip duplicado");
            }
        }
    }
    if tl_btn(ui, "trash", 15.0, "Eliminar (Supr)", has_sel, false, BtnStyle::Plain) {
        let mut ids = std::mem::take(&mut st.multi);
        if let Some(id) = selected_id(app) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        if !ids.is_empty() {
            let n = ids.len();
            app.editor.mutate(move |p| {
                for id in &ids {
                    p.remove_clip(id);
                }
            });
            app.editor.select(None);
            app.save_current();
            app.toast(if n == 1 { "Clip eliminado".to_string() } else { format!("{n} clips eliminados") });
        }
    }
    tl_sep(ui);
    if tl_btn(ui, "rotate", 16.0, "Girar 90°", true, false, BtnStyle::Plain) {
        if let Some(id) = need_selection(app) {
            app.editor.mutate(|p| p.rotate_clip_90(&id));
            app.save_current();
            app.toast("Clip girado 90°");
        }
    }
    if tl_btn(ui, "flip", 16.0, "Espejo horizontal", true, false, BtnStyle::Plain) {
        if let Some(id) = need_selection(app) {
            app.editor.mutate(|p| p.flip_clip_horizontal(&id));
            app.save_current();
            app.toast("Espejo aplicado");
        }
    }
    if tl_btn(ui, "reverse", 16.0, "Invertir reproducción", true, false, BtnStyle::Plain) {
        if let Some(id) = need_selection(app) {
            let ok = app.editor.mutate(|p| p.reverse_clip(&id)).unwrap_or(false);
            app.save_current();
            app.toast(if ok { "Reproducción invertida" } else { "Este clip no se puede invertir" });
        }
    }
    if tl_btn(ui, "freeze", 16.0, "Congelar fotograma", true, false, BtnStyle::Plain) {
        match selected_id(app) {
            None => app.toast("Selecciona un clip para congelar fotograma"),
            Some(id) => {
                let ph = app.playhead;
                let inside = project.find_clip(&id).map(|(_, cl)| ph >= cl.start && ph <= cl.end()).unwrap_or(false);
                if !inside {
                    app.toast("El cabezal debe estar dentro del clip");
                } else if let Some(Some(fid)) = app.editor.mutate(|p| p.freeze_frame(&id, ph)) {
                    app.editor.select(Some(fid));
                    app.save_current();
                    app.toast("Fotograma congelado (3 s)");
                }
            }
        }
    }
    if tl_btn(ui, "extractAudio", 16.0, "Separar audio a nueva pista", true, false, BtnStyle::Plain) {
        if let Some(id) = need_selection(app) {
            match app.editor.mutate(|p| p.separate_audio(&id)).flatten() {
                Some(_) => {
                    app.save_current();
                    app.toast("Audio separado a una nueva pista");
                }
                None => app.toast("Selecciona un clip de vídeo con audio"),
            }
        }
    }
    if tl_btn(ui, "marker", 15.0, "Añadir/quitar marcador (M)", true, false, BtnStyle::Plain) {
        let ph = app.playhead;
        let added = app.editor.mutate(|p| p.toggle_marker(ph)).unwrap_or(false);
        app.save_current();
        app.toast(if added { "Marcador añadido" } else { "Marcador eliminado" });
    }
    if tl_btn(ui, "shield", 15.0, "Guardado automático activado", true, false, BtnStyle::Plain) {
        app.save_current();
        app.toast("El proyecto se guarda automáticamente en este equipo");
    }

    // Lado derecho (se coloca de derecha a izquierda).
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.add_space(6.0);
        let px = PX_BASE * app.zoom as f32;
        if tl_btn(ui, "zoomIn", 16.0, "Acercar", true, false, BtnStyle::Plain) {
            set_px(app, px * 1.3);
        }
        zoom_slider(ui, app);
        if tl_btn(ui, "zoomOut", 16.0, "Alejar", true, false, BtnStyle::Plain) {
            set_px(app, px / 1.3);
        }
        if tl_btn(ui, "fit", 15.0, "Ajustar a la ventana", true, false, BtnStyle::Plain) {
            let len = project.timeline_length().max(1.0) as f32;
            set_px(app, (view_w - 26.0) / len);
            st.scroll_x = 0.0;
        }
        tl_sep(ui);
        if tl_btn(ui, "sparkles", 15.0, "Ajuste automático", true, false, BtnStyle::Cyan) {
            app.toast("La edición automática con IA no está incluida en esta versión local");
        }
        if tl_btn(ui, "link", 15.0, "Sincronizar pistas", true, false, BtnStyle::Cyan) {
            app.toast("Sincronización de pistas disponible al mover clips");
        }
        if tl_btn(ui, "magnet", 15.0, "Imán de ajuste", true, app.snap, BtnStyle::Cyan) {
            app.snap = !app.snap;
            app.toast(if app.snap { "Imán activado" } else { "Imán desactivado" });
        }
        tl_sep(ui);
        if tl_btn(ui, "mic", 15.0, "Grabar voz", true, false, BtnStyle::Plain) {
            app.panel_tab = PanelTab::Audio;
        }
    });
}

// ───────────────────────────── utilidades de dibujo ─────────────────────────────

fn ruler_step(px: f32) -> f64 {
    const STEPS: [f64; 12] = [0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
    STEPS.iter().copied().find(|s| *s as f32 * px >= 84.0).unwrap_or(600.0)
}

fn diamond(p: &egui::Painter, center: Pos2, half: f32, color: Color32) {
    p.add(Shape::convex_polygon(
        vec![
            Pos2::new(center.x, center.y - half),
            Pos2::new(center.x + half, center.y),
            Pos2::new(center.x, center.y + half),
            Pos2::new(center.x - half, center.y),
        ],
        color,
        Stroke::NONE,
    ));
}

fn xbtn(p: &egui::Painter, rect: Rect, hovered: bool, color: Color32) {
    p.rect_filled(rect, 3.0, Color32::from_black_alpha(if hovered { 153 } else { 89 }));
    p.text(rect.center(), Align2::CENTER_CENTER, "×", FontId::proportional(12.0), color);
}

#[allow(clippy::too_many_arguments)]
fn draw_clip(
    p: &egui::Painter,
    app: &mut CupApp,
    ctx: &egui::Context,
    project: &Project,
    clip: &Clip,
    rect: Rect,
    px: f32,
    selected: bool,
    marquee_hit: bool,
    dragging: bool,
    op: f32,
    hover: Option<Pos2>,
) {
    let (top, bot, border) = paint::clip_colors(clip.kind);
    let op = if dragging { op * 0.78 } else { op };
    paint::rounded_gradient(p, rect, 3, top.gamma_multiply(op), bot.gamma_multiply(op));
    let cp = p.with_clip_rect(rect.intersect(p.clip_rect()));

    // Miniatura en mosaico horizontal (opacidad .52).
    if let Some((tex, size)) = clip_thumb(app, ctx, project, clip) {
        let h = rect.height();
        let tw = (h * size.x / size.y.max(1.0)).max(8.0);
        let tint = Color32::WHITE.gamma_multiply(0.52 * op);
        // Solo los mosaicos visibles (con zoom un clip largo tenía miles).
        let vis = cp.clip_rect();
        let skip = ((vis.left() - rect.left()) / tw).floor().max(0.0);
        let mut x = rect.left() + skip * tw;
        let end = rect.right().min(vis.right());
        while x < end {
            let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(tw, h));
            cp.image(tex, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
            x += tw;
        }
    }
    // Sombreado (.clip-shade).
    paint::rounded_gradient(&cp, rect, 3, Color32::from_black_alpha((10.0 * op) as u8), Color32::from_black_alpha((122.0 * op) as u8));

    // Rombos de fotograma clave.
    if let Some(Value::Object(kfs)) = clip.extra.get("keyframes") {
        let mut times: Vec<f64> = kfs
            .values()
            .filter_map(Value::as_array)
            .flat_map(|l| l.iter().filter_map(|pt| pt.get("t").and_then(Value::as_f64)))
            .collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        times.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for t in times {
            let x = rect.left() + ((t - clip.start) as f32) * px;
            diamond(&cp, Pos2::new(x, rect.bottom() - 6.5), 3.5, c("#ffd166").gamma_multiply(op));
        }
    }

    // Onda de audio.
    if clip.kind == ClipKind::Audio {
        if let Some(wave) = clip.media_id.as_deref().and_then(|mid| wave_for(app, mid)) {
            let area = Rect::from_min_max(Pos2::new(rect.left() + 4.0, rect.top() + 8.0), Pos2::new(rect.right() - 4.0, rect.bottom() - 4.0));
            let col = Color32::from_rgba_unmultiplied(196, 202, 255, (0.62 * 0.55 * 255.0 * op) as u8);
            let mid_y = area.center().y;
            let vis = cp.clip_rect();
            let mut x = area.left().max(vis.left());
            let speed = clip.speed.max(0.01);
            while x < area.right().min(vis.right()) {
                let t = clip.in_point + ((x - rect.left()) / px) as f64 * speed;
                let i = (t * WAVE_PPS as f64) as usize;
                let j = ((t + 2.0 / px as f64 * speed) * WAVE_PPS as f64) as usize;
                let amp = wave.get(i..j.max(i + 1).min(wave.len())).map(|s| s.iter().cloned().fold(0.0f32, f32::max)).unwrap_or(0.0);
                let hh = (amp * area.height() * 0.5).max(0.5);
                cp.rect_filled(Rect::from_min_max(Pos2::new(x, mid_y - hh), Pos2::new(x + 1.2, mid_y + hh)), 0.0, col);
                x += 2.0;
            }
        }
    }

    let white = |a: f32| Color32::WHITE.gamma_multiply(a * op);
    match clip.kind {
        ClipKind::Text => {
            let content = clip.text.as_ref().and_then(|t| t.get("content")).and_then(Value::as_str).unwrap_or("");
            let font = FontId::proportional(9.5);
            let s = paint::ellipsize(&cp, content, &font, rect.width() - 16.0);
            cp.text(Pos2::new(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, s, font, white(0.85));
        }
        ClipKind::Sticker => {
            let content = clip
                .sticker
                .as_ref()
                .and_then(|s| s.get("content").or_else(|| s.get("emoji")))
                .and_then(Value::as_str)
                .unwrap_or(&clip.name)
                .to_string();
            let preview = serde_json::json!({ "content": content, "size": 15, "weight": 400 });
            paint::paint_text(&cp, rect, &preview, 1.0);
        }
        _ => {
            let font = theme::font_for(600.0, 9.0);
            let label = if clip.name.is_empty() { "Clip" } else { clip.name.as_str() };
            let s = paint::ellipsize(&cp, label, &font, rect.width() - 27.0);
            cp.text(Pos2::new(rect.left() + 8.0, rect.top() + 6.0), Align2::LEFT_TOP, s.clone(), font.clone(), Color32::from_black_alpha((200.0 * op) as u8));
            cp.text(Pos2::new(rect.left() + 7.0, rect.top() + 5.0), Align2::LEFT_TOP, s, font, white(1.0));
            cp.text(Pos2::new(rect.left() + 7.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, format_time(clip.duration), FontId::proportional(8.0), white(0.6));
        }
    }

    // Insignia fx.
    let has_fx = clip.extra.get("effect").and_then(Value::as_str).is_some_and(|e| !e.is_empty() && e != "none")
        || clip.extra.get("glEffect").and_then(Value::as_str).is_some_and(|e| !e.is_empty());
    if has_fx && rect.width() > 34.0 {
        let font = theme::font_for(700.0, 8.0);
        let r = Rect::from_min_size(Pos2::new(rect.right() - 10.0 - 16.0, rect.top() + 4.0), Vec2::new(16.0, 12.0));
        cp.rect_filled(r, 3.0, Color32::from_rgba_unmultiplied(25, 245, 228, 41));
        cp.rect_stroke(r, 3.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(25, 245, 228, 115)), StrokeKind::Inside);
        cp.text(r.center(), Align2::CENTER_CENTER, "fx", font, c("#8ef7ee"));
    }

    // Tiradores de recorte (resaltados al pasar el ratón).
    if let Some(h) = hover {
        let lh = Rect::from_min_max(rect.min, Pos2::new(rect.left() + HANDLE_W, rect.bottom()));
        let rh = Rect::from_min_max(Pos2::new(rect.right() - HANDLE_W, rect.top()), rect.max);
        let hc = Color32::from_rgba_unmultiplied(0, 229, 240, 166);
        if lh.contains(h) {
            cp.rect_filled(lh, 0.0, hc);
        } else if rh.contains(h) {
            cp.rect_filled(rh, 0.0, hc);
        }
    }

    // Borde (seleccionado: accent-2 + sombra de 1 px).
    if selected || marquee_hit {
        p.rect_stroke(rect, 3.0, Stroke::new(1.0, theme::ACCENT2), StrokeKind::Inside);
        p.rect_stroke(rect, 3.0, Stroke::new(1.0, theme::ACCENT2), StrokeKind::Outside);
    } else {
        p.rect_stroke(rect, 3.0, Stroke::new(1.0, border.gamma_multiply(op)), StrokeKind::Inside);
    }
}

// ───────────────────────────── principal ─────────────────────────────

enum Hit {
    Clip { id: String, edge: Option<TrimEdge>, track_locked: bool },
    FxRemove(String),
    Fx(String),
    FilterRemove(String),
    Filter(String, Rect),
    /// Borde derecho de la banda de transición (alargar/acortar).
    TransEdge(String),
    /// Cuerpo del bloque de transición entre dos clips: abre el panel.
    TransBody(String),
    /// «+» en un corte sin transición: abre el panel para ese corte.
    TransAdd(String),
    TransRemove(String),
}

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let Some(project) = app.editor.project().cloned() else { return };
    let ctx = ui.ctx().clone();
    let mut st = load_state(&ctx);

    let full = ui.available_rect_before_wrap();

    // ── barra de herramientas ──
    let tb_rect = Rect::from_min_size(full.min, Vec2::new(full.width(), TOOLBAR_H));
    let is_empty = project.tracks.iter().all(|t| t.clips.is_empty());
    let label_w = if is_empty { LABEL_W_EMPTY } else { LABEL_W };
    let view_w = full.width() - label_w;
    {
        let mut tb = ui.new_child(egui::UiBuilder::new().max_rect(tb_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        toolbar(&mut tb, app, &mut st, &project, view_w);
    }
    ui.painter().hline(full.x_range(), tb_rect.bottom() - 0.5, Stroke::new(1.0, c("#1e1e1f")));

    // Puede haber cambiado el proyecto o el zoom por la barra.
    let Some(project) = app.editor.project().cloned() else { return };
    let px = (PX_BASE * app.zoom as f32).clamp(PX_MIN, PX_MAX);
    let is_empty = project.tracks.iter().all(|t| t.clips.is_empty());

    let main = Rect::from_min_max(Pos2::new(full.left(), tb_rect.bottom()), full.max);
    ui.allocate_rect(full, Sense::hover());
    let ruler_h = if is_empty { RULER_H_EMPTY } else { RULER_H };
    let labels_rect = Rect::from_min_max(main.min, Pos2::new(main.left() + label_w, main.bottom()));
    let content_x = labels_rect.right();
    let ruler_rect = Rect::from_min_max(Pos2::new(content_x, main.top()), Pos2::new(main.right(), main.top() + ruler_h));
    let tracks_rect = Rect::from_min_max(Pos2::new(content_x, ruler_rect.bottom()), Pos2::new(main.right(), main.bottom() - SCROLLBAR));

    let painter = ui.painter().with_clip_rect(main);
    painter.rect_filled(main, CornerRadius::ZERO, theme::PANEL);

    // Filas
    let filter_entries: Vec<(Clip, (&'static str, String, f64))> = project
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter())
        .filter_map(|cl| active_filter(cl).map(|f| (cl.clone(), f)))
        .collect();
    let trans_entries: Vec<(String, String, f64, f64, bool)> = project
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter().map(move |cl| (cl, t.locked)))
        .filter_map(|(cl, locked)| {
            let tid = cl.extra.get("transition").and_then(Value::as_str).filter(|t| !t.is_empty() && *t != "none")?;
            let raw = cl.extra.get("transitionDuration").and_then(Value::as_f64).unwrap_or(0.6);
            let d = raw.min(cl.duration * 0.62).max(0.05);
            let lead = cl.extra.get("transitionLead").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, d);
            Some((cl.id.clone(), tid.to_string(), cl.start - lead, d, locked))
        })
        .collect();
    let rows = build_rows(&project, !filter_entries.is_empty(), !trans_entries.is_empty());
    let content_h = rows.len() as f32 * ROW_H + ADD_ROW_H;
    let len = project.timeline_length();
    let content_w = (len as f32 * px + 160.0).max(tracks_rect.width());

    // ── rueda: zoom (como el original); Mayús = desplazamiento horizontal ──
    let hover_pos = ctx.pointer_hover_pos();
    if hover_pos.is_some_and(|h| main.contains(h)) {
        let (delta, shift, ctrl) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift, i.modifiers.ctrl));
        if shift {
            let d = if delta.x.abs() > delta.y.abs() { delta.x } else { delta.y };
            st.scroll_x -= d;
        } else if ctrl || hover_pos.is_some_and(|h| labels_rect.contains(h)) {
            st.scroll_y -= delta.y;
        } else if delta.y.abs() > 0.5 {
            let factor = if delta.y > 0.0 { 1.1f32 } else { 0.9 }.powf((delta.y.abs() / 50.0).clamp(0.2, 2.0));
            let npx = (px * factor).clamp(PX_MIN, PX_MAX);
            if let Some(h) = hover_pos {
                let t = (h.x - content_x + st.scroll_x) / px;
                st.scroll_x = t * npx - (h.x - content_x);
            }
            set_px(app, npx);
        }
    }
    // Seguir el cabezal durante la reproducción.
    let ph_x_content = app.playhead as f32 * px;
    if app.playing && (ph_x_content < st.scroll_x || ph_x_content > st.scroll_x + tracks_rect.width() - 40.0) {
        st.scroll_x = ph_x_content - 40.0;
    }
    st.scroll_x = st.scroll_x.clamp(0.0, (content_w - tracks_rect.width()).max(0.0));
    st.scroll_y = st.scroll_y.clamp(0.0, (content_h - tracks_rect.height()).max(0.0));
    let sx = st.scroll_x;
    let sy = st.scroll_y;
    let time_x = |t: f64| content_x + t as f32 * px - sx;
    let x_time = |x: f32| ((x - content_x + sx) / px).max(0.0) as f64;
    let row_y = |i: usize| tracks_rect.top() + i as f32 * ROW_H - sy;

    let row_at = |y: f32| -> Option<usize> {
        if y < tracks_rect.top() {
            return None;
        }
        let i = ((y - tracks_rect.top() + sy) / ROW_H).floor() as usize;
        (i < rows.len()).then_some(i)
    };

    // ── zona de pistas ──
    let tp = painter.with_clip_rect(tracks_rect);
    let step = ruler_step(px);
    for (ri, row) in rows.iter().enumerate() {
        let y = row_y(ri);
        let r = Rect::from_min_size(Pos2::new(tracks_rect.left(), y), Vec2::new(tracks_rect.width(), ROW_H));
        if r.bottom() < tracks_rect.top() || r.top() > tracks_rect.bottom() {
            continue;
        }
        let (bg, grid) = match row {
            Row::Fx => (c("#1a151f"), !is_empty),
            Row::Filters => (c("#15121c"), false),
            Row::Transitions => (c("#17130f"), false),
            Row::Track(i) => {
                let over = st.drag_over.as_deref() == Some(project.tracks[*i].id.as_str());
                (if over { c("#1a2f30") } else { theme::PANEL }, !is_empty)
            }
        };
        tp.rect_filled(r, 0.0, bg);
        if grid {
            let mut t = (sx / px).floor() as f64;
            while time_x(t) < r.right() {
                tp.vline(time_x(t).round() + 0.5, r.y_range(), Stroke::new(1.0, c("#2d2d2d")));
                t += 1.0;
            }
        }
        if !is_empty {
            tp.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, c("#232323")));
        }
        if let Row::Track(i) = row {
            let t = &project.tracks[*i];
            if t.clips.is_empty() && !is_empty {
                tp.text(Pos2::new(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, track_meta(t.kind).0, FontId::proportional(9.0), c("#4f4f4f"));
            }
        }
    }

    // Recogemos rectángulos de impacto mientras pintamos.
    let mut hits: Vec<(Rect, Hit)> = Vec::new();
    let hover_in_tracks = hover_pos.filter(|h| tracks_rect.contains(*h));
    let marquee_rect = st.marquee.map(|(a, b)| Rect::from_two_pos(a, b));
    let sel = app.editor.selection().map(str::to_string);
    let dragging_id = st.drag.as_ref().filter(|d| d.moved).map(|d| d.clip_id.clone());
    let mut marquee_hits: Vec<String> = Vec::new();

    for (ri, row) in rows.iter().enumerate() {
        let y = row_y(ri);
        let lane = Rect::from_min_size(Pos2::new(tracks_rect.left(), y), Vec2::new(tracks_rect.width(), ROW_H));
        if lane.bottom() < tracks_rect.top() || lane.top() > tracks_rect.bottom() {
            continue;
        }
        match row {
            Row::Fx => {
                for (id, name, start, dur) in effect_clips(&project) {
                    let r = Rect::from_min_size(Pos2::new(time_x(start), y + 5.0), Vec2::new((dur as f32 * px).max(24.0), ROW_H - 12.0));
                    if r.right() < lane.left() || r.left() > lane.right() {
                        continue;
                    }
                    let hov = hover_in_tracks.is_some_and(|h| r.contains(h));
                    paint::rounded_gradient(&tp, r, 4, c("#7b46d6"), c("#5a2fa8"));
                    tp.rect_stroke(r, 4.0, Stroke::new(1.0, c("#a678ff")), StrokeKind::Inside);
                    if hov {
                        tp.rect_filled(r, 4.0, Color32::from_white_alpha(18));
                    }
                    let font = theme::font_for(600.0, 9.0);
                    let s = paint::ellipsize(&tp, &name, &font, r.width() - 30.0);
                    tp.with_clip_rect(r).text(Pos2::new(r.left() + 7.0, r.center().y), Align2::LEFT_CENTER, s, font, c("#f3ecff"));
                    let xr = Rect::from_center_size(Pos2::new(r.right() - 10.0, r.center().y), Vec2::splat(16.0));
                    xbtn(&tp, xr, hover_in_tracks.is_some_and(|h| xr.contains(h)), c("#f3ecff"));
                    hits.push((xr, Hit::FxRemove(id.clone())));
                    hits.push((r, Hit::Fx(id)));
                }
            }
            Row::Transitions => {
                for (id, tid, start, d, locked) in &trans_entries {
                    let label = crate::panels::TRANSITIONS.iter().find(|(i, _, _)| i == tid).map(|(_, l, _)| *l).unwrap_or(tid.as_str());
                    let r = Rect::from_min_size(Pos2::new(time_x(*start), y + 5.0), Vec2::new((*d as f32 * px).max(46.0), ROW_H - 12.0));
                    if r.right() < lane.left() || r.left() > lane.right() {
                        continue;
                    }
                    let dragging_this = st.trans_drag.as_ref().is_some_and(|(i, _, _)| i == id);
                    if let Some((mid, mx)) = st.trans_move.as_ref().filter(|(m, _)| m == id) {
                        let _ = mid;
                        let ghost = Rect::from_min_size(Pos2::new(*mx - r.width() / 2.0, r.top()), r.size());
                        tp.rect_filled(ghost, 4.0, c("#ffb703").gamma_multiply(0.85));
                        tp.rect_stroke(ghost, 4.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Inside);
                        if let Some((_, cs)) = nearest_cut(&project, x_time(*mx)) {
                            tp.vline(time_x(cs), tracks_rect.y_range(), Stroke::new(2.0, c("#ffb703")));
                        }
                    }
                    let active = app.playhead >= *start && app.playhead <= *start + *d;
                    let hov = hover_in_tracks.is_some_and(|h| r.contains(h));
                    let base = c("#ffb703");
                    tp.rect_filled(r, 4.0, if active || dragging_this || hov { base } else { base.gamma_multiply(0.75) });
                    let stripes = tp.with_clip_rect(r);
                    let mut sx0 = r.left() - r.height();
                    while sx0 < r.right() {
                        stripes.line_segment([Pos2::new(sx0, r.bottom()), Pos2::new(sx0 + r.height(), r.top())], Stroke::new(1.0, Color32::from_black_alpha(26)));
                        sx0 += 7.0;
                    }
                    let inner = tp.with_clip_rect(r.shrink2(Vec2::new(2.0, 0.0)));
                    icons::paint(&inner, "transition", Pos2::new(r.left() + 11.0, r.center().y), 12.0, c("#1a1020"));
                    let font = theme::font_for(600.0, 9.0);
                    let s = paint::ellipsize(&inner, &format!("{label} · {:.1}s", d), &font, r.width() - 56.0);
                    inner.text(Pos2::new(r.left() + 22.0, r.center().y), Align2::LEFT_CENTER, s, font, c("#1a1020"));
                    let grip = Rect::from_center_size(Pos2::new(r.right(), r.center().y), Vec2::new(10.0, r.height()));
                    let grip_hov = hover_in_tracks.is_some_and(|h| grip.contains(h));
                    let gcol = if grip_hov || dragging_this { Color32::WHITE } else { c("#1a1020").gamma_multiply(0.8) };
                    tp.rect_filled(Rect::from_center_size(Pos2::new(r.right() - 2.5, r.center().y), Vec2::new(3.0, r.height() - 6.0)), 1.5, gcol);
                    if dragging_this {
                        let gal = tp.layout_no_wrap(format!("{:.2} s", d), FontId::proportional(10.0), Color32::WHITE);
                        let bg = Rect::from_min_size(Pos2::new(r.right() - gal.size().x / 2.0 - 5.0, r.top() - gal.size().y - 6.0), gal.size() + Vec2::new(10.0, 4.0));
                        painter.rect_filled(bg, 3.0, c("#111111"));
                        painter.galley(bg.min + Vec2::new(5.0, 2.0), gal, Color32::WHITE);
                    }
                    if !*locked {
                        let xr = Rect::from_center_size(Pos2::new(r.right() - 16.0, r.center().y), Vec2::splat(14.0));
                        if hov {
                            xbtn(&tp, xr, hover_in_tracks.is_some_and(|h| xr.contains(h)), c("#1a1020"));
                            hits.push((xr, Hit::TransRemove(id.clone())));
                        }
                        hits.push((grip, Hit::TransEdge(id.clone())));
                        hits.push((r, Hit::TransBody(id.clone())));
                    }
                }
            }
            Row::Filters => {
                for (cl, (kind, label, amount)) in &filter_entries {
                    let r = Rect::from_min_size(Pos2::new(time_x(cl.start), y + 5.0), Vec2::new((cl.duration as f32 * px).max(24.0), ROW_H - 12.0));
                    if r.right() < lane.left() || r.left() > lane.right() {
                        continue;
                    }
                    let hov = hover_in_tracks.is_some_and(|h| r.contains(h));
                    paint::rounded_gradient(&tp, r, 4, c("#7b46d6"), c("#5a2fa8"));
                    tp.rect_stroke(r, 4.0, Stroke::new(1.0, c("#a678ff")), StrokeKind::Inside);
                    if hov {
                        tp.rect_filled(r, 4.0, Color32::from_white_alpha(20));
                    }
                    let inner = tp.with_clip_rect(r.shrink2(Vec2::new(2.0, 0.0)));
                    inner.circle_filled(Pos2::new(r.left() + 10.0, r.center().y), 2.0, c("#d9c6ff"));
                    let font = theme::font_for(600.0, 9.0);
                    let amount_txt = if *kind == "css" { String::new() } else { format!("{}%", (amount * 100.0).round() as i64) };
                    let aw = if amount_txt.is_empty() { 0.0 } else { 30.0 };
                    let s = paint::ellipsize(&inner, label, &font, r.width() - 42.0 - aw);
                    inner.text(Pos2::new(r.left() + 18.0, r.center().y), Align2::LEFT_CENTER, s, font, c("#f3ecff"));
                    if !amount_txt.is_empty() {
                        inner.text(Pos2::new(r.right() - 22.0, r.center().y), Align2::RIGHT_CENTER, amount_txt, FontId::proportional(9.0), c("#f3ecff").gamma_multiply(0.9));
                    }
                    let xr = Rect::from_center_size(Pos2::new(r.right() - 10.0, r.center().y), Vec2::splat(16.0));
                    xbtn(&tp, xr, hover_in_tracks.is_some_and(|h| xr.contains(h)), c("#f3ecff"));
                    hits.push((xr, Hit::FilterRemove(cl.id.clone())));
                    hits.push((r, Hit::Filter(cl.id.clone(), r)));
                }
            }
            Row::Track(i) => {
                let track = &project.tracks[*i];
                let op = if track.hidden { 0.38 } else { 1.0 };
                for clip in &track.clips {
                    let r = Rect::from_min_max(
                        Pos2::new(time_x(clip.start), y + 4.0),
                        Pos2::new(time_x(clip.start) + (clip.duration as f32 * px).max(8.0), y + ROW_H - 5.0),
                    );
                    if r.right() < lane.left() - 2.0 || r.left() > lane.right() + 2.0 {
                        continue;
                    }
                    let in_marquee = marquee_rect.is_some_and(|m| m.intersects(r)) && !track.locked;
                    if in_marquee {
                        marquee_hits.push(clip.id.clone());
                    }
                    let is_sel = sel.as_deref() == Some(clip.id.as_str()) || st.multi.contains(&clip.id);
                    let hov = if track.locked { None } else { hover_in_tracks.filter(|h| r.contains(*h)) };
                    draw_clip(&tp, app, &ctx, &project, clip, r, px, is_sel, in_marquee, dragging_id.as_deref() == Some(clip.id.as_str()), op, hov);
                    if clip.extra.get("transition").and_then(Value::as_str).is_some_and(|t| !t.is_empty() && t != "none") {
                    } else if !track.locked && matches!(clip.kind, ClipKind::Video | ClipKind::Image) {
                        // Corte entre dos clips pegados sin transición: «+» al pasar el cursor.
                        let has_prev = track.clips.iter().any(|o| o.id != clip.id && ((o.start + o.duration) - clip.start).abs() < 0.05);
                        if has_prev {
                            let plus = Rect::from_center_size(Pos2::new(r.left(), r.center().y), Vec2::splat(18.0));
                            let show = hover_in_tracks.is_some_and(|h| plus.expand(8.0).contains(h));
                            if show {
                                tp.circle_filled(plus.center(), 9.0, c("#ffb703"));
                                tp.text(plus.center(), Align2::CENTER_CENTER, "+", FontId::proportional(14.0), c("#1a1020"));
                            }
                            hits.push((plus, Hit::TransAdd(clip.id.clone())));
                        }
                    }
                    let edge = |h: Pos2| -> Option<TrimEdge> {
                        if h.x < r.left() + HANDLE_W {
                            Some(TrimEdge::Left)
                        } else if h.x > r.right() - HANDLE_W {
                            Some(TrimEdge::Right)
                        } else {
                            None
                        }
                    };
                    // El tirador se decide al pulsar; guardamos ambos lados como zonas.
                    let lh = Rect::from_min_max(r.min, Pos2::new(r.left() + HANDLE_W, r.bottom()));
                    let rh = Rect::from_min_max(Pos2::new(r.right() - HANDLE_W, r.top()), r.max);
                    let _ = edge;
                    hits.push((lh, Hit::Clip { id: clip.id.clone(), edge: Some(TrimEdge::Left), track_locked: track.locked }));
                    hits.push((rh, Hit::Clip { id: clip.id.clone(), edge: Some(TrimEdge::Right), track_locked: track.locked }));
                    hits.push((r, Hit::Clip { id: clip.id.clone(), edge: None, track_locked: track.locked }));
                }
            }
        }
    }

    let hit_at = |pos: Pos2| -> Option<&Hit> { hits.iter().find(|(r, _)| r.contains(pos)).map(|(_, h)| h) };

    // Cursor según la zona.
    if let Some(h) = hover_in_tracks {
        match hit_at(h) {
            Some(Hit::Clip { edge: Some(_), track_locked: false, .. }) | Some(Hit::TransEdge(_)) => ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
            Some(Hit::Clip { track_locked: true, .. }) => ctx.set_cursor_icon(egui::CursorIcon::NotAllowed),
            Some(Hit::Clip { .. }) => ctx.set_cursor_icon(if st.drag.is_some() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab }),
            Some(_) => ctx.set_cursor_icon(egui::CursorIcon::PointingHand),
            None => {}
        }
    }

    // ── interacción en la zona de pistas ──
    let tresp = ui.interact(tracks_rect, egui::Id::new("tl-tracks"), Sense::click_and_drag());
    let (alt, additive) = ui.input(|i| (i.modifiers.alt, i.modifiers.shift || i.modifiers.command));

    if tresp.double_clicked() {
        if let Some(pos) = tresp.interact_pointer_pos() {
            if let Some(Hit::Fx(id)) = hit_at(pos) {
                let id = id.clone();
                remove_effect_clip(app, &id);
            }
        }
    } else if tresp.clicked() {
        if let Some(pos) = tresp.interact_pointer_pos() {
            match hit_at(pos) {
                Some(Hit::FxRemove(id)) => {
                    let id = id.clone();
                    remove_effect_clip(app, &id);
                }
                Some(Hit::FilterRemove(id)) => {
                    let id = id.clone();
                    clear_clip_filter(app, &id);
                    st.filter_editor = None;
                }
                Some(Hit::Filter(id, r)) => {
                    let id = id.clone();
                    app.editor.select(Some(id.clone()));
                    st.filter_editor = Some((id, r.left_top()));
                    st.filter_snapshot = false;
                }
                Some(Hit::Fx(_)) => {}
                Some(Hit::TransRemove(id)) => {
                    let id = id.clone();
                    app.mutate_project(|p| {
                        let patch = p.find_clip(&id).map(|(_, c)| {
                            let d = c.extra.get("transitionDuration").and_then(Value::as_f64).unwrap_or(0.6);
                            crate::panels::transition_patch(c, "none", d)
                        });
                        if let Some(patch) = patch {
                            p.update_clip(&id, patch);
                        }
                    });
                    app.save_current();
                    app.toast("Transición eliminada");
                }
                Some(Hit::TransEdge(id)) | Some(Hit::TransBody(id)) | Some(Hit::TransAdd(id)) => {
                    let id = id.clone();
                    app.editor.select(Some(id));
                    app.left_view = crate::app::LeftView::Media;
                    app.panel_tab = PanelTab::Transitions;
                }
                Some(Hit::Clip { id, .. }) => {
                    let id = id.clone();
                    if app.tool_mode == ToolMode::Split {
                        let t = x_time(pos.x);
                        app.editor.mutate(|p| p.split_at(t, Some(&id)));
                        app.save_current();
                        app.toast("Clip dividido con cuchilla");
                    } else if additive {
                        if let Some(s) = selected_id(app) {
                            if !st.multi.contains(&s) {
                                st.multi.push(s);
                            }
                        }
                        if let Some(p) = st.multi.iter().position(|m| *m == id) {
                            st.multi.remove(p);
                        } else {
                            st.multi.push(id.clone());
                        }
                        app.editor.select(Some(id));
                    } else {
                        st.multi.clear();
                        app.editor.select(Some(id));
                    }
                }
                None => {
                    // Clic en vacío: mueve el cursor y limpia la selección.
                    let t = x_time(pos.x);
                    app.playhead = t;
                    app.audio_seek(t);
                    app.editor.select(None);
                    st.multi.clear();
                }
            }
        }
    }

    if tresp.secondary_clicked() {
        if let Some(pos) = tresp.interact_pointer_pos() {
            st.ctx_clip = match hit_at(pos) {
                Some(Hit::Clip { id, .. }) => Some(id.clone()),
                _ => None,
            };
            if let Some(id) = st.ctx_clip.clone() {
                if !st.multi.contains(&id) {
                    st.multi.clear();
                    app.editor.select(Some(id));
                }
            }
        }
    }
    let mut menu_act: Option<&'static str> = None;
    if st.ctx_clip.is_some() || st.clipboard.is_some() {
        let has_clip = st.ctx_clip.is_some();
        let can_paste = st.clipboard.is_some();
        let is_video = st
            .ctx_clip
            .as_deref()
            .and_then(|id| project.find_clip(id))
            .is_some_and(|(_, c)| c.kind == ClipKind::Video);
        tresp.context_menu(|ui| {
            ui.set_min_width(200.0);
            let mut item = |ui: &mut egui::Ui, label: &str, key: &str, enabled: bool, act: &'static str| {
                let b = ui.add_enabled(enabled, egui::Button::new(label).shortcut_text(key).min_size(Vec2::new(190.0, 0.0)));
                if b.clicked() {
                    menu_act = Some(act);
                    ui.close();
                }
            };
            item(ui, "Copiar", "Ctrl+C", has_clip, "copy");
            item(ui, "Cortar", "Ctrl+X", has_clip, "cut");
            item(ui, "Pegar", "Ctrl+V", can_paste, "paste");
            item(ui, "Duplicar", "Ctrl+D", has_clip, "dup");
            item(ui, "Eliminar", "Supr", has_clip, "del");
            ui.separator();
            item(ui, "Dividir en el cursor", "B", has_clip, "split");
            item(ui, "Extraer audio", "Ctrl+Mayús+S", is_video, "extract");
        });
    }
    if let Some(act) = menu_act {
        let id = st.ctx_clip.clone();
        match (act, id) {
            ("copy", Some(id)) | ("cut", Some(id)) => {
                st.clipboard = project.find_clip(&id).map(|(_, c)| c.clone());
                if act == "cut" {
                    app.editor.mutate(|p| p.remove_clip(&id));
                    app.editor.select(None);
                    app.save_current();
                }
                app.toast(if act == "cut" { "Clip cortado" } else { "Clip copiado" });
            }
            ("paste", _) => {
                if let Some(mut clip) = st.clipboard.clone() {
                    let at = app.playhead;
                    clip.id = cupcraft_core::new_id("clip");
                    let target = st.ctx_clip.as_deref().and_then(|i| project.find_clip(i)).map(|(t, _)| t.id.clone());
                    let new_id = clip.id.clone();
                    let move_id = new_id.clone();
                    app.editor.mutate(move |p| {
                        let kind = clip.kind;
                        let tid = target
                            .filter(|t| p.track(t).is_some_and(|tr| !tr.locked))
                            .or_else(|| p.tracks.iter().find(|t| !t.locked && match (kind, t.kind) {
                                (ClipKind::Audio, TrackType::Audio) | (ClipKind::Text, TrackType::Text) => true,
                                (ClipKind::Audio, _) | (ClipKind::Text, _) => false,
                                (_, k) => matches!(k, TrackType::Video | TrackType::Overlay),
                            }).map(|t| t.id.clone()));
                        if let Some(tid) = tid {
                            if let Some(tr) = p.track_mut(&tid) {
                                clip.start = at.max(0.0);
                                tr.clips.push(clip);
                            }
                            p.move_clip(&move_id, Some(at.max(0.0)), None);
                        }
                    });
                    app.editor.select(Some(new_id));
                    app.save_current();
                    app.toast("Clip pegado");
                }
            }
            ("dup", Some(id)) => {
                if let Some(Some(did)) = app.editor.mutate(|p| p.duplicate_clip(&id)) {
                    app.editor.select(Some(did));
                    app.save_current();
                    app.toast("Clip duplicado");
                }
            }
            ("del", Some(id)) => {
                app.editor.mutate(|p| p.remove_clip(&id));
                app.editor.select(None);
                st.multi.clear();
                app.save_current();
                app.toast("Clip eliminado");
            }
            ("split", Some(id)) => {
                let ph = app.playhead;
                let n = app.editor.mutate(|p| p.split_at(ph, Some(&id))).unwrap_or(0);
                if n > 0 {
                    app.save_current();
                    app.toast("Clip dividido");
                } else {
                    app.toast("Coloca el cursor sobre el clip para dividirlo");
                }
            }
            ("extract", Some(id)) => match app.editor.mutate(|p| p.separate_audio(&id)).flatten() {
                Some(_) => {
                    app.save_current();
                    app.toast("Audio separado a una nueva pista");
                }
                None => app.toast("Selecciona un clip de vídeo con audio"),
            },
            _ => {}
        }
    }

    if tresp.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
        match hit_at(origin) {
            Some(Hit::Clip { id, edge, track_locked }) => {
                if *track_locked {
                    app.toast("La pista está bloqueada");
                } else if app.tool_mode == ToolMode::Select {
                    let id = id.clone();
                    let edge = *edge;
                    if let Some((track, clip)) = project.find_clip(&id) {
                        let media_dur = clip.media_id.as_deref().and_then(|m| project.media.iter().find(|x| x.id == m)).and_then(|m| m.duration);
                        st.drag = Some(ClipDrag {
                            clip_id: id.clone(),
                            mode: edge.map(DragMode::Trim).unwrap_or(DragMode::Move),
                            origin,
                            orig_start: clip.start,
                            orig_dur: clip.duration,
                            orig_in: clip.in_point,
                            kind: track.kind,
                            media_dur,
                            moved: false,
                        });
                        if !st.multi.contains(&id) {
                            st.multi.clear();
                        }
                        app.editor.select(Some(id));
                    }
                }
            }
            Some(Hit::TransBody(id)) => {
                st.trans_move = Some((id.clone(), origin.x));
                app.editor.select(Some(id.clone()));
            }
            Some(Hit::TransEdge(id)) => {
                if let Some((_, clip)) = project.find_clip(id) {
                    let max = (clip.duration * 0.62).max(0.1);
                    let lead = clip.extra.get("transitionLead").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
                    st.trans_drag = Some((id.clone(), clip.start - lead, max));
                    app.editor.mutate(|_| ()); // un único paso de deshacer
                    app.editor.select(Some(id.clone()));
                }
            }
            Some(_) => {}
            None => {
                st.marquee = Some((origin, origin));
                st.marquee_base = if additive { st.multi.clone() } else { Vec::new() };
                if let (true, Some(s)) = (additive, selected_id(app)) {
                    if !st.marquee_base.contains(&s) {
                        st.marquee_base.push(s);
                    }
                }
            }
        }
    }

    if tresp.dragged() {
        if let Some(pos) = ctx.pointer_interact_pos() {
            if let Some((a, _)) = st.marquee {
                st.marquee = Some((a, pos));
            }
            if let Some((_, x)) = st.trans_move.as_mut() {
                *x = pos.x;
            }
            if let Some((id, start, max)) = st.trans_drag.clone() {
                // Pasos de 0,05 s; Alt = libre.
                let mut d = (x_time(pos.x) - start).clamp(0.1, max);
                if !alt {
                    d = ((d / 0.05).round() * 0.05).clamp(0.1, max);
                }
                let cur = project.find_clip(&id).and_then(|(_, c)| c.extra.get("transitionDuration").and_then(Value::as_f64));
                if cur.is_some_and(|c| (c - d).abs() < 1e-6) {
                    // Sin cambios: nada que tocar este fotograma.
                } else {
                app.editor.mutate_silent(move |p| {
                    let fades = p
                        .find_clip(&id)
                        .and_then(|(_, c)| c.extra.get("transition").and_then(Value::as_str))
                        .map(crate::panels::transition_fades)
                        .unwrap_or((0.0, 0.0));
                    let mut patch = json!({ "transitionDuration": d });
                    if fades.0 > 0.0 {
                        patch["fadeIn"] = json!(d);
                    }
                    p.update_clip(&id, patch);
                });
                }
            }
            if let Some(mut d) = st.drag.clone() {
                if !d.moved && ((pos.x - d.origin.x).abs() > 2.0 || (pos.y - d.origin.y).abs() > 2.0) {
                    d.moved = true;
                    app.editor.mutate(|_| ()); // un único paso de deshacer para todo el arrastre
                }
                if d.moved {
                    let dt = ((pos.x - d.origin.x) / px) as f64;
                    match d.mode {
                        DragMode::Move => {
                            let raw = d.orig_start + dt;
                            let (value, point) = if alt { (raw.max(0.0), None) } else { snap_start(app, &project, raw, d.orig_dur, &d.clip_id, px) };
                            st.guide = point;
                            // Pista destino del mismo tipo y sin bloquear.
                            let mut target = project.find_clip(&d.clip_id).map(|(t, _)| t.id.clone());
                            st.drag_over = None;
                            if let Some(Row::Track(ti)) = row_at(pos.y).map(|i| rows[i].clone()) {
                                let cand = &project.tracks[ti];
                                if tracks_compatible(cand.kind, d.kind) && !cand.locked {
                                    target = Some(cand.id.clone());
                                    st.drag_over = Some(cand.id.clone());
                                }
                            }
                            // Arrastrar por encima de la pista más alta crea una capa nueva arriba.
                            let top_ti = project.tracks.iter().position(|t| tracks_compatible(t.kind, d.kind) && !t.locked);
                            let top_row_y = top_ti.and_then(|ti| rows.iter().position(|r| matches!(r, Row::Track(i) if *i == ti))).map(row_y);
                            let above_top = top_row_y.is_some_and(|y| pos.y < y + 6.0)
                                && !matches!(row_at(pos.y).map(|i| rows[i].clone()), Some(Row::Track(ti)) if tracks_compatible(project.tracks[ti].kind, d.kind) && Some(ti) != top_ti);
                            let new_layer = if above_top {
                                let cur = project.find_clip(&d.clip_id).map(|(t, _)| (t.id.clone(), t.clips.len()));
                                let alone_on_top = cur.is_some_and(|(tid, n)| n == 1 && top_ti.is_some_and(|ti| project.tracks[ti].id == tid));
                                !alone_on_top
                            } else {
                                false
                            };
                            let id = d.clip_id.clone();
                            if new_layer {
                                let (kind, at) = (d.kind, top_ti.unwrap_or(0));
                                st.drag_over = None;
                                app.editor.mutate_silent(move |p| {
                                    let tid = p.insert_track_at(kind, at);
                                    p.move_clip(&id, Some(value), Some(&tid));
                                });
                            } else {
                                app.editor.mutate_silent(move |p| p.move_clip(&id, Some(value), target.as_deref()));
                            }
                        }
                        DragMode::Trim(edge) => {
                            let delta = match edge {
                                TrimEdge::Left => dt.clamp(-d.orig_start, d.orig_dur - 0.1),
                                TrimEdge::Right => dt.min(f64::MAX),
                            };
                            let id = d.clip_id.clone();
                            let (os, odur, oin, md) = (d.orig_start, d.orig_dur, d.orig_in, d.media_dur);
                            app.editor.mutate_silent(move |p| {
                                if let Some(cl) = p.find_clip_mut(&id) {
                                    cl.start = os;
                                    cl.duration = odur;
                                    cl.in_point = oin;
                                }
                                p.trim_clip(&id, edge, delta, md);
                            });
                        }
                    }
                    app.sync_engine();
                }
                st.drag = Some(d);
            }
        }
    }

    if tresp.drag_stopped() {
        if let Some((id, x)) = st.trans_move.take() {
            // Suelta libre: el bloque queda centrado donde lo sueltas; el corte más
            // cercano lo recibe y `transitionLead` dice cuánto empieza antes de él.
            let center = x_time(x);
            if let Some((target, cut)) = nearest_cut(&project, center) {
                let src = project.find_clip(&id).map(|(_, c)| {
                    let tid = c.extra.get("transition").and_then(Value::as_str).unwrap_or("none").to_string();
                    let d = c.extra.get("transitionDuration").and_then(Value::as_f64).unwrap_or(0.6);
                    (tid, d.min(c.duration * 0.62).max(0.05))
                });
                if let Some((tid, d)) = src {
                    let lead = (cut - (center - d / 2.0)).clamp(0.0, d);
                    app.mutate_project(|p| {
                        if target != id {
                            let off = p.find_clip(&id).map(|(_, c)| crate::panels::transition_patch(c, "none", d));
                            if let Some(off) = off {
                                p.update_clip(&id, off);
                            }
                        }
                        let on = p.find_clip(&target).map(|(_, c)| crate::panels::transition_patch(c, &tid, d));
                        if let Some(mut on) = on {
                            on["transitionLead"] = json!(lead);
                            p.update_clip(&target, on);
                        }
                    });
                    app.save_current();
                    app.editor.select(Some(target));
                }
            }
        }
        if let Some((_, _, _)) = st.trans_drag.take() {
            app.save_current();
        }
        if let Some(d) = st.drag.take() {
            if d.moved {
                app.save_current();
            }
        }
        if st.marquee.take().is_some() {
            let mut ids = std::mem::take(&mut st.marquee_base);
            for id in &marquee_hits {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
            app.editor.select(ids.first().cloned());
            if ids.len() > 1 {
                app.toast(format!("{} clips seleccionados", ids.len()));
            }
            st.multi = if ids.len() > 1 { ids } else { Vec::new() };
        }
        st.guide = None;
        st.drag_over = None;
    }

    // Caja de selección.
    if let Some(m) = marquee_rect {
        let m = m.intersect(tracks_rect);
        tp.rect_filled(m, 2.0, Color32::from_rgba_unmultiplied(0, 193, 205, 30));
        tp.rect_stroke(m, 2.0, Stroke::new(1.0, theme::ACCENT2), StrokeKind::Inside);
    }

    // ── regla ──
    let rp = painter.with_clip_rect(ruler_rect);
    rp.rect_filled(ruler_rect, 0.0, theme::PANEL);
    if !is_empty {
        rp.hline(ruler_rect.x_range(), ruler_rect.bottom() - 0.5, Stroke::new(1.0, c("#1d1d1d")));
        let minor = step / 2.0;
        let first = ((sx / px) as f64 / minor).floor() * minor;
        let mut t = first.max(0.0);
        while time_x(t) <= ruler_rect.right() {
            let x = time_x(t).round() + 0.5;
            let is_major = ((t / step).round() * step - t).abs() < 1e-6;
            rp.vline(x, ruler_rect.y_range(), Stroke::new(1.0, if is_major { c("#3b3b3b") } else { c("#2c2c2c") }));
            if is_major {
                rp.text(Pos2::new(x + 4.0, ruler_rect.top() + 5.0), Align2::LEFT_TOP, format_time(t), FontId::proportional(8.5), c("#858585"));
            }
            t += minor;
        }
    }
    // Marcadores (#f1b332) a lo alto de regla y pistas.
    for &m in &project.markers {
        let x = time_x(m);
        if x < content_x || x > main.right() {
            continue;
        }
        painter.vline(x, Rect::from_min_max(ruler_rect.min, tracks_rect.max).y_range(), Stroke::new(1.0, c("#f1b332")));
        diamond(&painter, Pos2::new(x, ruler_rect.top() + 5.5), 4.5, c("#f1b332"));
    }
    let rresp = ui.interact(ruler_rect, egui::Id::new("tl-ruler"), Sense::click_and_drag());
    if rresp.clicked() || rresp.dragged() {
        if let Some(pos) = rresp.interact_pointer_pos() {
            let t = x_time(pos.x);
            app.playhead = t;
            app.audio_seek(t);
        }
    }
    rresp.on_hover_cursor(egui::CursorIcon::PointingHand);

    // Guía de imán.
    if let Some(g) = st.guide {
        let x = time_x(g);
        let yr = Rect::from_min_max(ruler_rect.min, tracks_rect.max).y_range();
        painter.vline(x, yr, Stroke::new(5.0, Color32::from_rgba_unmultiplied(255, 209, 102, 40)));
        painter.vline(x, yr, Stroke::new(1.0, theme::GUIDE));
    }

    // Cabezal.
    let phx = time_x(app.playhead);
    if phx >= content_x - 1.0 && phx <= main.right() {
        painter.vline(phx, Rect::from_min_max(ruler_rect.min, tracks_rect.max).y_range(), Stroke::new(1.0, theme::PLAYHEAD));
        let top = ruler_rect.top() + 2.0;
        let yellow = c("#ffdd00");
        painter.add(Shape::convex_polygon(
            vec![Pos2::new(phx - 5.0, top), Pos2::new(phx + 5.0, top), Pos2::new(phx, top + 12.0)],
            yellow,
            Stroke::NONE,
        ));
        painter.rect_filled(Rect::from_min_size(Pos2::new(phx - 1.0, top + 10.0), Vec2::new(2.0, 8.0)), 1.0, yellow);
    }

    // ── estado vacío ──
    if is_empty {
        let band = Rect::from_center_size(
            Pos2::new((content_x + main.right()) / 2.0, tracks_rect.center().y),
            Vec2::new(main.right() - content_x, 48.0),
        );
        painter.rect_filled(band, 0.0, c("#2a2a2a"));
        let dash = Stroke::new(1.0, c("#3f3f3f"));
        let mut x = band.left();
        while x < band.right() {
            painter.hline(x..=(x + 4.0).min(band.right()), band.top() + 0.5, dash);
            painter.hline(x..=(x + 4.0).min(band.right()), band.bottom() - 0.5, dash);
            x += 8.0;
        }
        icons::paint(&painter, "film", Pos2::new(band.left() + 14.0 + 8.0, band.center().y), 13.0, c("#d0d0d0"));
        painter.text(Pos2::new(band.left() + 14.0 + 16.0 + 9.0, band.center().y), Align2::LEFT_CENTER, "Arrastra el material aquí y empieza a crear", FontId::proportional(11.0), c("#d4d4d4"));
    }

    // ── etiquetas de pista ──
    labels(ui, app, &painter, &project, &rows, labels_rect, ruler_h, sy, is_empty);

    // ── barras de desplazamiento ──
    scrollbars(ui, &painter, &mut st, tracks_rect, main, content_w, content_h);

    // ── editor de intensidad del filtro ──
    filter_editor(&ctx, app, &mut st);

    store_state(&ctx, st);
}

fn snap_start(app: &CupApp, project: &Project, desired: f64, duration: f64, exclude: &str, px: f32) -> (f64, Option<f64>) {
    if !app.snap {
        return (desired.max(0.0), None);
    }
    let threshold = 8.0 / px as f64;
    let mut points = vec![app.playhead, 0.0];
    for t in &project.tracks {
        for cl in &t.clips {
            if cl.id != exclude {
                points.push(cl.start);
                points.push(cl.end());
            }
        }
    }
    points.extend(project.markers.iter().copied());
    let mut best: Option<(f64, f64, f64)> = None;
    for point in points {
        for off in [0.0, duration] {
            let cand = point - off;
            let dist = (cand - desired).abs();
            if cand >= -0.001 && dist < threshold && best.is_none_or(|b| dist < b.2) {
                best = Some((cand.max(0.0), point, dist));
            }
        }
    }
    match best {
        Some((v, p, _)) => (v, Some(p)),
        None => (desired.max(0.0), None),
    }
}

#[allow(clippy::too_many_arguments)]
fn labels(ui: &mut egui::Ui, app: &mut CupApp, painter: &egui::Painter, project: &Project, rows: &[Row], rect: Rect, ruler_h: f32, sy: f32, is_empty: bool) {
    let p = painter.with_clip_rect(rect);
    p.rect_filled(rect, 0.0, theme::PANEL);
    p.vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, if is_empty { c("#333333") } else { c("#1d1d1d") }));
    if is_empty {
        return; // `.tl-empty .track-label { visibility: hidden }`
    }
    p.hline(rect.x_range(), rect.top() + ruler_h - 0.5, Stroke::new(1.0, c("#1d1d1d")));
    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + ruler_h), Pos2::new(rect.right(), rect.bottom() - SCROLLBAR));
    let bp = p.with_clip_rect(body);
    let hover = ui.ctx().pointer_hover_pos();

    let icon_box = |pp: &egui::Painter, center: Pos2, icon: &str, bg: Color32, fg: Color32| {
        pp.rect_filled(Rect::from_center_size(center, Vec2::splat(20.0)), 3.0, bg);
        icons::paint(pp, icon, center, 13.0, fg);
    };

    for (ri, row) in rows.iter().enumerate() {
        let y = body.top() + ri as f32 * ROW_H - sy;
        let r = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width() - 1.0, ROW_H));
        if r.bottom() < body.top() || r.top() > body.bottom() {
            continue;
        }
        bp.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, c("#232323")));
        let ic = Pos2::new(r.left() + 18.0, r.center().y);
        let name_x = r.left() + 34.0;
        match row {
            Row::Fx => {
                icon_box(&bp, ic, "sparkles", c("#303030"), c("#a2a2a2"));
                bp.text(Pos2::new(name_x, r.center().y), Align2::LEFT_CENTER, "Efectos", FontId::proportional(10.0), c("#c9a9ff"));
            }
            Row::Filters => {
                icon_box(&bp, ic, "filter", c("#2a2138"), c("#c9a9ff"));
                bp.text(Pos2::new(name_x, r.center().y), Align2::LEFT_CENTER, "Filtros", FontId::proportional(10.0), c("#c9a9ff"));
            }
            Row::Transitions => {
                icon_box(&bp, ic, "transition", c("#3a2c10"), c("#ffb703"));
                bp.text(Pos2::new(name_x, r.center().y), Align2::LEFT_CENTER, "Transiciones", FontId::proportional(10.0), c("#ffcf5a"));
            }
            Row::Track(i) => {
                let t = &project.tracks[*i];
                let (_, icon) = track_meta(t.kind);
                icon_box(&bp, ic, icon, c("#303030"), c("#a2a2a2"));
                let row_hover = hover.is_some_and(|h| r.contains(h) && body.contains(h));
                let actions_w = 50.0;
                let font = FontId::proportional(10.0);
                let name = paint::ellipsize(&bp, &t.name, &font, r.right() - 8.0 - actions_w - name_x - 4.0);
                bp.text(Pos2::new(name_x, r.center().y), Align2::LEFT_CENTER, name, font, c("#a9a9a9"));
                // Acciones: ocultar/silenciar y bloquear (opacidad .55 salvo al pasar).
                let is_audio = t.kind == TrackType::Audio;
                let toggled = if is_audio { t.muted } else { t.hidden };
                let b1 = Rect::from_min_size(Pos2::new(r.right() - 8.0 - 49.0, r.center().y - 12.0), Vec2::splat(24.0));
                let b2 = Rect::from_min_size(Pos2::new(r.right() - 8.0 - 24.0, r.center().y - 12.0), Vec2::splat(24.0));
                if !body.contains(b1.center()) {
                    continue;
                }
                let tid = t.id.clone();
                let alpha = if row_hover { 1.0 } else { 0.55 };
                let icon1 = if is_audio { if t.muted { "mute" } else { "volume" } } else { "eye" };
                let tip1 = if is_audio { "Silenciar pista" } else { "Ocultar pista" };
                if label_btn(ui, &bp, b1, egui::Id::new(("tl-tg1", &tid)), icon1, 13.0, toggled, alpha, tip1) {
                    let field_audio = is_audio;
                    let id2 = tid.clone();
                    app.editor.mutate(move |p| {
                        if let Some(tr) = p.tracks.iter_mut().find(|x| x.id == id2) {
                            if field_audio {
                                tr.muted = !tr.muted;
                            } else {
                                tr.hidden = !tr.hidden;
                            }
                        }
                    });
                    app.save_current();
                }
                if label_btn(ui, &bp, b2, egui::Id::new(("tl-tg2", &tid)), "lock", 12.0, t.locked, alpha, "Bloquear pista") {
                    let id2 = tid.clone();
                    app.editor.mutate(move |p| {
                        if let Some(tr) = p.tracks.iter_mut().find(|x| x.id == id2) {
                            tr.locked = !tr.locked;
                        }
                    });
                    app.save_current();
                }
            }
        }
    }

    // Fila para añadir pistas.
    let y = body.top() + rows.len() as f32 * ROW_H - sy;
    let r = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width() - 1.0, ADD_ROW_H));
    if r.top() < body.bottom() && r.bottom() > body.top() {
        bp.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, c("#232323")));
        let ic = Pos2::new(r.left() + 18.0, r.center().y);
        bp.rect_filled(Rect::from_center_size(ic, Vec2::splat(20.0)), 3.0, c("#303030"));
        icons::paint(&bp, "plus", ic, 13.0, theme::ACCENT);
        let kinds = [
            (TrackType::Video, "film", "Añadir pista de vídeo"),
            (TrackType::Text, "text", "Añadir pista de texto"),
            (TrackType::Audio, "music", "Añadir pista de audio"),
            (TrackType::Overlay, "layers", "Añadir pista de superposición"),
        ];
        for (k, (kind, icon, tip)) in kinds.into_iter().enumerate() {
            let b = Rect::from_min_size(Pos2::new(r.left() + 32.0 + k as f32 * 26.0, r.center().y - 11.0), Vec2::splat(22.0));
            if !body.contains(b.center()) {
                continue;
            }
            let resp = ui.interact(b, egui::Id::new(("tl-add", k)), Sense::click()).on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
            let hov = resp.hovered();
            bp.rect_filled(b, 4.0, if hov { c("#1f2a2b") } else { c("#232323") });
            bp.rect_stroke(b, 4.0, Stroke::new(1.0, if hov { theme::ACCENT } else { c("#333333") }), StrokeKind::Inside);
            icons::paint(&bp, icon, b.center(), 12.0, if hov { theme::ACCENT } else { c("#a9a9a9") });
            if resp.clicked() {
                app.editor.mutate(move |p| {
                    p.add_track(kind);
                });
                app.save_current();
                app.toast("Pista añadida");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn label_btn(ui: &mut egui::Ui, p: &egui::Painter, rect: Rect, id: egui::Id, icon: &str, size: f32, active: bool, alpha: f32, tip: &str) -> bool {
    let resp = ui.interact(rect, id, Sense::click()).on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
    if active {
        p.rect_filled(rect, 4.0, theme::ACCENT_SOFT.gamma_multiply(alpha));
    } else if resp.hovered() {
        p.rect_filled(rect, 4.0, c("#2f2f2f"));
    }
    let col = if active { theme::ACCENT } else if resp.hovered() { Color32::WHITE } else { c("#b5b5b5") };
    icons::paint(p, icon, rect.center(), size, col.gamma_multiply(alpha));
    resp.clicked()
}

fn scrollbars(ui: &mut egui::Ui, painter: &egui::Painter, st: &mut TlState, tracks: Rect, main: Rect, content_w: f32, content_h: f32) {
    // Horizontal (debajo de las pistas).
    let hbar = Rect::from_min_max(Pos2::new(tracks.left(), main.bottom() - SCROLLBAR), Pos2::new(main.right(), main.bottom()));
    if content_w > tracks.width() + 1.0 {
        let frac = tracks.width() / content_w;
        let tw = (hbar.width() * frac).max(24.0);
        let maxs = content_w - tracks.width();
        let tx = hbar.left() + (hbar.width() - tw) * (st.scroll_x / maxs).clamp(0.0, 1.0);
        let thumb = Rect::from_min_size(Pos2::new(tx, hbar.top() + 2.0), Vec2::new(tw, SCROLLBAR - 4.0));
        let resp = ui.interact(hbar, egui::Id::new("tl-hbar"), Sense::click_and_drag());
        if resp.drag_started() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if !thumb.contains(pos) {
                    st.scroll_x = ((pos.x - hbar.left() - tw / 2.0) / (hbar.width() - tw)).clamp(0.0, 1.0) * maxs;
                }
                st.hscroll_drag = Some((pos.x, st.scroll_x));
            }
        }
        if let (true, Some((x0, s0)), Some(pos)) = (resp.dragged(), st.hscroll_drag, resp.interact_pointer_pos()) {
            st.scroll_x = (s0 + (pos.x - x0) / (hbar.width() - tw).max(1.0) * maxs).clamp(0.0, maxs);
        }
        if resp.drag_stopped() {
            st.hscroll_drag = None;
        }
        let col = if resp.hovered() || resp.dragged() { c("#6a6a6a") } else { c("#4a4a4a") };
        painter.rect_filled(thumb, 3.0, col);
    }
    // Vertical (borde derecho de las pistas).
    if content_h > tracks.height() + 1.0 {
        let vbar = Rect::from_min_max(Pos2::new(tracks.right() - SCROLLBAR, tracks.top()), tracks.max);
        let frac = tracks.height() / content_h;
        let th = (vbar.height() * frac).max(24.0);
        let maxs = content_h - tracks.height();
        let ty = vbar.top() + (vbar.height() - th) * (st.scroll_y / maxs).clamp(0.0, 1.0);
        let thumb = Rect::from_min_size(Pos2::new(vbar.left() + 2.0, ty), Vec2::new(SCROLLBAR - 4.0, th));
        let resp = ui.interact(vbar, egui::Id::new("tl-vbar"), Sense::click_and_drag());
        if resp.drag_started() {
            if let Some(pos) = resp.interact_pointer_pos() {
                st.vscroll_drag = Some((pos.y, st.scroll_y));
            }
        }
        if let (true, Some((y0, s0)), Some(pos)) = (resp.dragged(), st.vscroll_drag, resp.interact_pointer_pos()) {
            st.scroll_y = (s0 + (pos.y - y0) / (vbar.height() - th).max(1.0) * maxs).clamp(0.0, maxs);
        }
        if resp.drag_stopped() {
            st.vscroll_drag = None;
        }
        let col = if resp.hovered() || resp.dragged() { c("#6a6a6a") } else { c("#4a4a4a") };
        painter.rect_filled(thumb, 3.0, col);
    }
}

/// Barra flotante de intensidad del filtro (`[data-filter-editor]` del original).
fn filter_editor(ctx: &egui::Context, app: &mut CupApp, st: &mut TlState) {
    let Some((clip_id, anchor)) = st.filter_editor.clone() else { return };
    let Some(project) = app.editor.project() else { return };
    let Some((_, clip)) = project.find_clip(&clip_id) else {
        st.filter_editor = None;
        return;
    };
    let Some((kind, label, amount)) = active_filter(clip) else {
        st.filter_editor = None;
        return;
    };
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.filter_editor = None;
        return;
    }
    let pos = Pos2::new(anchor.x.max(6.0), (anchor.y - 78.0).max(40.0));
    let mut close = false;
    let mut clear = false;
    let mut new_amount: Option<f64> = None;
    let area = egui::Area::new(egui::Id::new("tl-filter-editor"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(c("#2b2b2b"))
                .stroke(Stroke::new(1.0, c("#3a3a3a")))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(184.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&label).size(11.0).color(c("#efefef")).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::icon_button_sized(ui, "x", 12.0, Vec2::splat(20.0), c("#b5b5b5"), "Cerrar", false).clicked() {
                                close = true;
                            }
                        });
                    });
                    if kind != "css" {
                        ui.horizontal(|ui| {
                            let mut v = (amount * 100.0).round() as i64;
                            ui.spacing_mut().slider_width = 118.0;
                            let r = ui.add(egui::Slider::new(&mut v, 0..=100).show_value(false));
                            ui.label(egui::RichText::new(format!("{v}%")).size(10.0).color(c("#cfcfcf")));
                            if r.changed() {
                                new_amount = Some(v as f64 / 100.0);
                            }
                        });
                    }
                    if ui.add(egui::Button::new(egui::RichText::new(crate::i18n::tr("Quitar filtro")).size(10.5)).fill(c("#3a3a3a"))).clicked() {
                        clear = true;
                    }
                });
        });
    if let Some(a) = new_amount {
        if !st.filter_snapshot {
            app.editor.mutate(|_| ());
            st.filter_snapshot = true;
        }
        let id = clip_id.clone();
        app.editor.mutate_silent(move |p| {
            if let Some(cl) = p.find_clip_mut(&id) {
                if cl.gl_filter.is_some() {
                    cl.gl_filter_amount = Some(a);
                } else {
                    cl.lut_amount = Some(a);
                }
            }
        });
        app.save_current();
    }
    if clear {
        clear_clip_filter(app, &clip_id);
        st.filter_editor = None;
    } else if close {
        st.filter_editor = None;
    } else if ctx.input(|i| i.pointer.any_pressed()) {
        // Clic fuera del panel → se cierra (como `onDocumentClick`).
        if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
            if !area.response.rect.contains(p) {
                st.filter_editor = None;
            }
        }
    }
}
