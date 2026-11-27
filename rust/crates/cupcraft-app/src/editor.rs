//! Editor principal (equivalente nativo de `editor/index.js` + `preview.js`):
//! barra de título con menús, carril izquierdo + paneles de recursos,
//! reproductor con composición en vivo, inspector y línea de tiempo.

use crate::app::{CupApp, LeftView, Modal, PanelTab, ToolMode};
use crate::format::{format_bytes, format_timecode};
use crate::icons;
use crate::paint;
use crate::panels;
use crate::services::compose::{active_layers, clip_alpha};
use crate::services::{media, raster, video};
use crate::theme;
use crate::timeline as tl;
use cupcraft_core::keyframes as kf;
use cupcraft_core::ClipKind;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const TITLEBAR_H: f32 = 42.0;
const RAIL_W: f32 = 140.0;

// ----------------------------------------------------------------------
// Ventana sin marco: arrastre, controles y redimensionado por los bordes
// (sustituye a la barra de título personalizada de la versión web).
// ----------------------------------------------------------------------

/// Zona de arrastre de la ventana (doble clic = maximizar/restaurar).
pub fn drag_region(ui: &mut egui::Ui, rect: Rect) {
    let resp = ui.interact(rect, ui.scope_id().with("drag-region"), Sense::click_and_drag());
    if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if resp.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
}

/// Botones minimizar / maximizar / cerrar (estilo Windows 11).
pub fn window_controls(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let btn = |ui: &mut egui::Ui, kind: u8, tip: &str| -> bool {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(44.0, TITLEBAR_H), Sense::click());
        let hovered = resp.hovered();
        if hovered {
            let fill = if kind == 2 { Color32::from_rgb(0xc4, 0x2b, 0x1c) } else { Color32::from_white_alpha(18) };
            ui.painter().rect_filled(r, 0.0, fill);
        }
        let c = if hovered { Color32::WHITE } else { theme::MUTED };
        let p = ui.painter();
        let s = Stroke::new(1.1, c);
        let m = r.center();
        match kind {
            0 => {
                p.line_segment([m + Vec2::new(-5.0, 0.0), m + Vec2::new(5.0, 0.0)], s);
            }
            1 => {
                if max {
                    p.rect_stroke(Rect::from_center_size(m + Vec2::new(-1.0, 1.0), Vec2::splat(8.0)), 1.0, s, StrokeKind::Middle);
                    p.line_segment([m + Vec2::new(-3.0, -5.0), m + Vec2::new(5.0, -5.0)], s);
                    p.line_segment([m + Vec2::new(5.0, -5.0), m + Vec2::new(5.0, 3.0)], s);
                } else {
                    p.rect_stroke(Rect::from_center_size(m, Vec2::splat(10.0)), 1.0, s, StrokeKind::Middle);
                }
            }
            _ => {
                p.line_segment([m + Vec2::new(-5.0, -5.0), m + Vec2::new(5.0, 5.0)], s);
                p.line_segment([m + Vec2::new(5.0, -5.0), m + Vec2::new(-5.0, 5.0)], s);
            }
        }
        resp.on_hover_text(tip).clicked()
    };
    ui.spacing_mut().item_spacing.x = 0.0;
    // Se añaden de derecha a izquierda.
    if btn(ui, 2, "Cerrar") {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    if btn(ui, 1, if max { "Restaurar" } else { "Maximizar" }) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
    if btn(ui, 0, "Minimizar") {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
}

/// Redimensionado desde los bordes de la ventana sin marco, más un borde
/// fino visible (la ventana no tiene marco del sistema).
pub fn resize_edges(ctx: &egui::Context, screen: Rect) {
    use egui::viewport::ResizeDirection as D;
    let (max, full) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if max || full {
        return;
    }
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("window-border"))).rect_stroke(
        screen.shrink(0.5),
        window_radius(ctx),
        Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4d, 0x55)),
        StrokeKind::Inside,
    );
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    const E: f32 = 5.0;
    let l = pos.x <= screen.left() + E;
    let r = pos.x >= screen.right() - E;
    let t = pos.y <= screen.top() + E;
    let b = pos.y >= screen.bottom() - E;
    let dir = match (l, r, t, b) {
        (true, _, true, _) => Some((D::NorthWest, egui::CursorIcon::ResizeNorthWest)),
        (_, true, true, _) => Some((D::NorthEast, egui::CursorIcon::ResizeNorthEast)),
        (true, _, _, true) => Some((D::SouthWest, egui::CursorIcon::ResizeSouthWest)),
        (_, true, _, true) => Some((D::SouthEast, egui::CursorIcon::ResizeSouthEast)),
        (true, ..) => Some((D::West, egui::CursorIcon::ResizeWest)),
        (_, true, ..) => Some((D::East, egui::CursorIcon::ResizeEast)),
        (_, _, true, _) => Some((D::North, egui::CursorIcon::ResizeNorth)),
        (_, _, _, true) => Some((D::South, egui::CursorIcon::ResizeSouth)),
        _ => None,
    };
    if let Some((d, cursor)) = dir {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
        }
    }
}

/// Radio de las esquinas de la ventana (0 si está maximizada o a pantalla completa).
pub fn window_radius(ctx: &egui::Context) -> u8 {
    let (max, full) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if max || full {
        0
    } else {
        10
    }
}

/// Tarjeta redondeada con hueco alrededor (estilo CapCut).
fn card_frame(fill: Color32, inner: egui::Margin) -> egui::Frame {
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::from(8))
        .inner_margin(inner)
        .outer_margin(egui::Margin::same(4))
}

// ----------------------------------------------------------------------
// Entrada
// ----------------------------------------------------------------------

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let ctx = ui.ctx().clone();
    let screen = ui.max_rect();
    // Fondo oscuro tras las tarjetas redondeadas (los huecos entre paneles).
    ui.painter().rect_filled(screen, window_radius(&ctx), theme::BG2);

    egui::Panel::top("editor-titlebar")
        .exact_size(TITLEBAR_H)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| titlebar(ui, app));

    egui::Panel::bottom("editor-timeline")
        .resizable(true)
        .default_size(320.0)
        .min_size(120.0)
        .max_size((screen.height() - 220.0).max(240.0))
        .frame(card_frame(theme::BG2, egui::Margin::ZERO))
        .show(ui, |ui| tl::show(ui, app));

    egui::Panel::left("editor-rail")
        .exact_size(RAIL_W)
        .resizable(false)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin { left: 6, right: 6, top: 14, bottom: 8 }))
        .show(ui, |ui| rail(ui, app));

    egui::Panel::left("editor-resources")
        .default_size(400.0)
        .min_size(200.0)
        .max_size((screen.width() * 0.55).max(300.0))
        .resizable(true)
        .frame(card_frame(theme::PANEL, egui::Margin::symmetric(10, 6)))
        .show(ui, |ui| match app.left_view {
            LeftView::Media => panels::show(ui, app),
            LeftView::Timeline => timeline_view(ui, app),
            LeftView::Secondary => secondary_view(ui, app),
        });

    egui::Panel::right("editor-inspector")
        .default_size(300.0)
        .min_size(200.0)
        .max_size((screen.width() * 0.55).max(300.0))
        .resizable(true)
        .frame(card_frame(theme::PANEL, egui::Margin::symmetric(12, 10)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| crate::inspector::show(ui, app));
        });

    egui::CentralPanel::default()
        .frame(card_frame(theme::BG, egui::Margin::ZERO))
        .show(ui, |ui| player(ui, app));

    custom_ratio_dialog(&ctx, app);
    if app.modal.is_none() {
        handle_shortcuts(&ctx, app);
    }
    resize_edges(&ctx, screen);
}

// ----------------------------------------------------------------------
// Barra de título
// ----------------------------------------------------------------------

fn titlebar(ui: &mut egui::Ui, app: &mut CupApp) {
    let rect = ui.max_rect();
    // La zona de arrastre se registra primero: los widgets añadidos después
    // quedan por encima y reciben sus propios clics.
    drag_region(ui, rect);
    ui.horizontal_centered(|ui| {
        ui.add_space(10.0);
        // Logotipo → inicio
        let (lr, lresp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
        icons::paint_logo(ui.painter(), lr, Color32::WHITE);
        if lresp.on_hover_text("Volver al inicio").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            app.back_home();
            return;
        }
        ui.add_space(4.0);
        main_menu(ui, app);
        if text_button(ui, "Registro", false).on_hover_text("Ver el registro de actividad").clicked() {
            app.modal = Some(Modal::Log { filter: String::new(), only_errors: false });
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            window_controls(ui);
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.x = 8.0;
            let export = ui.add(
                egui::Button::new(RichText::new("Exportar").color(theme::ACCENT_INK).strong().size(12.5))
                    .fill(theme::ACCENT)
                    .corner_radius(CornerRadius::from(6))
                    .min_size(Vec2::new(84.0, 28.0)),
            );
            if export.on_hover_text("Exportar video (MP4 / WebM)").clicked() {
                app.export_handle = app.export_handle.take().filter(|h| h.is_running());
                app.modal = Some(Modal::Export);
            }
            if text_button(ui, "Compartir", true).clicked() {
                app.modal = Some(Modal::Share);
            }
            if icons::icon_button(ui, "cropFree", 16.0, theme::MUTED, "Capturar fotograma").clicked() {
                app.capture_frame();
            }
            settings_menu(ui, app);
            // Indicador de guardado
            let busy = app.pending_jobs > 0;
            let (dr, dresp) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
            ui.painter().circle_filled(dr.center(), 3.5, if busy { theme::WARN } else { theme::ACCENT });
            dresp.on_hover_text(if busy { "Procesando…" } else { "Guardado en este equipo" });

            // Título editable centrado en el espacio restante.
            let remaining = ui.available_width();
            ui.allocate_ui_with_layout(Vec2::new(remaining, TITLEBAR_H), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                title_field(ui, app);
            });
        });
    });
}

fn text_button(ui: &mut egui::Ui, label: &str, framed: bool) -> egui::Response {
    let b = egui::Button::new(RichText::new(label).color(theme::TEXT).size(12.5)).corner_radius(CornerRadius::from(6)).min_size(Vec2::new(0.0, 28.0));
    let b = if framed { b.fill(theme::PANEL3) } else { b.frame(false) };
    ui.add(b).on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn title_field(ui: &mut egui::Ui, app: &mut CupApp) {
    let Some((pid, pname)) = app.editor.project().map(|p| (p.id.clone(), p.name.clone())) else { return };
    let id = egui::Id::new("project-title");
    let focused = ui.memory(|m| m.has_focus(id));
    if !focused {
        app.title_buf = pname.clone();
    }
    let w = (ui.fonts_mut(|f| f.layout_no_wrap(app.title_buf.clone(), egui::FontId::proportional(13.0), theme::TEXT).size().x) + 24.0).clamp(80.0, 360.0);
    let resp = ui.add(
        egui::TextEdit::singleline(&mut app.title_buf)
            .id(id)
            .char_limit(80)
            .frame(if focused { egui::Frame::NONE.fill(theme::FIELD).stroke(egui::Stroke::new(1.0, theme::ACCENT)).corner_radius(4).inner_margin(egui::Margin::symmetric(6, 3)) } else { egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 3)) })
            .align(egui::Align2::CENTER_CENTER)
            .font(egui::FontId::proportional(13.0))
            .text_color(theme::TEXT)
            .desired_width(w),
    );
    let resp = resp.on_hover_text("Cambiar el nombre del proyecto");
    if resp.lost_focus() {
        let name = app.title_buf.trim().to_string();
        if !name.is_empty() && name != pname {
            app.rename_project(&pid, &name);
        } else {
            app.title_buf = pname;
        }
    }
}

fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str, shortcut: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(212.0), 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, theme::PANEL3);
    }
    icons::paint(ui.painter(), icon, r.left_center() + Vec2::new(16.0, 0.0), 15.0, theme::MUTED);
    ui.painter().text(r.left_center() + Vec2::new(32.0, 0.0), egui::Align2::LEFT_CENTER, crate::i18n::tr(label), egui::FontId::proportional(12.5), theme::TEXT);
    if !shortcut.is_empty() {
        ui.painter().text(r.right_center() - Vec2::new(10.0, 0.0), egui::Align2::RIGHT_CENTER, shortcut, egui::FontId::proportional(11.0), theme::MUTED2);
    }
    let clicked = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn style_menu(ui: &mut egui::Ui) {
    ui.set_min_width(228.0);
    ui.spacing_mut().item_spacing.y = 0.0;
}

fn main_menu(ui: &mut egui::Ui, app: &mut CupApp) {
    let btn = egui::Button::new(RichText::new("Menú ▾").color(theme::TEXT).size(12.5)).frame(false).min_size(Vec2::new(0.0, 28.0));
    egui::containers::menu::MenuButton::from_button(btn).ui(ui, |ui| {
        style_menu(ui);
        if menu_item(ui, "plus", "Nuevo proyecto", "") {
            app.save_current();
            app.modal = Some(Modal::NewProject { cat: 0, tpl: 0, quality: 2, fps: 1, name: "Proyecto sin título".into() });
        }
        if menu_item(ui, "save", "Guardar ahora", "Ctrl+S") {
            app.save_current();
            app.toast("Proyecto guardado");
        }
        if menu_item(ui, "download", "Guardar proyecto como archivo", "") {
            if let Some(id) = app.editor.project().map(|p| p.id.clone()) {
                app.save_current();
                app.save_project_file(&id);
            }
        }
        if menu_item(ui, "folderOpen", "Abrir proyecto desde archivo", "") {
            app.open_project_file();
        }
        if menu_item(ui, "import", "Importar medios", "Ctrl+I") {
            app.import_media_dialog();
        }
        if menu_item(ui, "folderOpen", "Abrir proyecto de CapCut", "") {
            app.open_capcut();
        }
        if menu_item(ui, "link", "Puente IA", "") {
            app.modal = Some(Modal::Bridge);
        }
        if menu_item(ui, "export", "Exportar video", "") {
            app.modal = Some(Modal::Export);
        }
        ui.separator();
        if menu_item(ui, "home", "Ir al inicio", "") {
            app.back_home();
        }
        if menu_item(ui, "keyboard", "Atajos de teclado", "") {
            app.modal = Some(Modal::Shortcuts);
        }
        if menu_item(ui, "info", "Acerca de cupcraft", "") {
            app.modal = Some(Modal::About);
        }
    });
}

fn settings_menu(ui: &mut egui::Ui, app: &mut CupApp) {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, Color32::from_rgb(0x2f, 0x2f, 0x2f));
    }
    icons::paint(ui.painter(), "settings", r.center(), 16.0, if resp.hovered() { Color32::WHITE } else { theme::MUTED });
    let resp = resp.on_hover_text("Ajustes");
    egui::Popup::menu(&resp).show(|ui| {
        style_menu(ui);
        if menu_item(ui, "settings", "Preferencias", "") {
            app.modal = Some(Modal::Settings);
        }
        if menu_item(ui, "keyboard", "Atajos", "") {
            app.modal = Some(Modal::Shortcuts);
        }
        if menu_item(ui, "folder", "Espacio de almacenamiento", "") {
            let used = app.storage_bytes();
            app.toast(format!("Almacenamiento local usado: {}", format_bytes(used)));
        }
        if menu_item(ui, "info", "Acerca de", "") {
            app.modal = Some(Modal::About);
        }
    });
}

// ----------------------------------------------------------------------
// Carril izquierdo
// ----------------------------------------------------------------------

// Colores de `.left-rail` en styles.css.
const RAIL_BOX: Color32 = Color32::from_rgb(0x3b, 0x3b, 0x3b);
const RAIL_BOX_HOVER: Color32 = Color32::from_rgb(0x45, 0x45, 0x45);
const RAIL_HOVER: Color32 = Color32::from_rgb(0x30, 0x30, 0x30);

/// `.rail-item`: 27 px, 10.5 px, recorte con "…"; activo = fondo #3b3b3b y texto cian.
fn rail_item(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 27.0), Sense::click());
    if active {
        ui.painter().rect_filled(r, 4.0, RAIL_BOX);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, RAIL_HOVER);
    }
    let font = egui::FontId::proportional(10.5);
    let text = paint::ellipsize(ui.painter(), label, &font, r.width() - 14.0);
    let color = if active {
        theme::ACCENT2
    } else if resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xd4, 0xd4, 0xd4)
    };
    ui.painter().text(r.left_center() + Vec2::new(7.0, 0.0), egui::Align2::LEFT_CENTER, text, font, color);
    resp.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// `.rail-drop`: caja #3b3b3b de 28 px con chevron y etiqueta "AI" arriba a la derecha.
fn rail_group(ui: &mut egui::Ui, open: &mut Option<&'static str>, key: &'static str, label: &str, ai: bool) -> bool {
    let is_open = *open == Some(key);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(r, 4.0, if resp.hovered() { RAIL_BOX_HOVER } else { RAIL_BOX });
    let font = egui::FontId::proportional(11.0);
    let text = paint::ellipsize(p, label, &font, r.width() - 30.0);
    let color = if is_open { Color32::WHITE } else { Color32::from_rgb(0xde, 0xde, 0xde) };
    p.text(r.left_center() + Vec2::new(7.0, 0.0), egui::Align2::LEFT_CENTER, text, font, color);
    icons::paint(p, if is_open { "chevronUp" } else { "chevronDown" }, r.right_center() - Vec2::new(if ai { 18.0 } else { 12.0 }, 0.0), 10.0, color);
    if ai {
        let tf = egui::FontId::proportional(7.5);
        let g = p.layout_no_wrap("AI".into(), tf.clone(), Color32::from_rgb(0x07, 0x3b, 0x3e));
        let tr = Rect::from_min_size(egui::pos2(r.right() - g.size().x - 6.0, r.top() - 8.0), Vec2::new(g.size().x + 6.0, 13.0));
        p.rect_filled(tr, 2.0, theme::ACCENT);
        p.text(tr.center(), egui::Align2::CENTER_CENTER, "AI", tf, Color32::from_rgb(0x07, 0x3b, 0x3e));
        p.circle_filled(egui::pos2(r.right() - 5.0, r.center().y), 3.0, theme::ACCENT);
    }
    if resp.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *open = if is_open { None } else { Some(key) };
    }
    *open == Some(key)
}

/// `.rail-sub button`: 10.5 px, #a8a8a8, recorte con "…".
fn rail_sub(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 3.0, RAIL_HOVER);
    }
    let font = egui::FontId::proportional(10.5);
    let t = paint::ellipsize(ui.painter(), label, &font, r.width() - 16.0);
    let color = if active {
        theme::ACCENT2
    } else if resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xa8, 0xa8, 0xa8)
    };
    ui.painter().text(r.left_center() + Vec2::new(8.0, 0.0), egui::Align2::LEFT_CENTER, t, font, color);
    resp.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn rail(ui: &mut egui::Ui, app: &mut CupApp) {
    ui.spacing_mut().item_spacing.y = 2.0;
    // `.rail-import`: 28 px, fondo #3b3b3b, texto e icono cian.
    let (ir, imp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    {
        let p = ui.painter();
        p.rect_filled(ir, 4.0, if imp.hovered() { RAIL_BOX_HOVER } else { RAIL_BOX });
        p.text(ir.left_center() + Vec2::new(7.0, 0.0), egui::Align2::LEFT_CENTER, "Importar", egui::FontId::proportional(11.0), theme::ACCENT2);
        icons::paint(p, "chevronUp", ir.right_center() - Vec2::new(12.0, 0.0), 12.0, theme::ACCENT2);
    }
    let imp = imp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if imp.on_hover_text("Importar videos, imágenes o audio (Ctrl+I)").clicked() {
        app.import_media_dialog();
    }
    ui.add_space(8.0);
    for (view, label) in [
        (LeftView::Media, "Archivos multimedia"),
        (LeftView::Timeline, "Línea de tiempo"),
        (LeftView::Secondary, "Proyectos secundarios"),
    ] {
        if rail_item(ui, label, app.left_view == view) {
            app.left_view = view;
        }
    }
    ui.add_space(14.0);
    ui.spacing_mut().item_spacing.y = 7.0;

    let gid = egui::Id::new("rail-open-group");
    let mut open: Option<&'static str> = ui.data(|d| d.get_temp::<Option<&'static str>>(gid)).flatten();

    if rail_group(ui, &mut open, "yours", "Tuyos", false) {
        ui.add_space(-5.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        for (i, l) in ["Todos los medios", "Videos", "Imágenes", "Audio"].iter().enumerate() {
            let active = app.left_view == LeftView::Media && app.panel_tab == PanelTab::Media && app.media_filter == i as u8;
            if rail_sub(ui, l, active) {
                app.left_view = LeftView::Media;
                app.panel_tab = PanelTab::Media;
                app.media_filter = i as u8;
            }
        }
        ui.add_space(3.0);
        ui.spacing_mut().item_spacing.y = 7.0;
    }
    if rail_group(ui, &mut open, "generate", "Generar", true) {
        ui.add_space(-5.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        for l in ["Imagen de IA", "Video de IA", "Audio de IA"] {
            if rail_sub(ui, l, false) {
                app.ai_unavailable();
            }
        }
        ui.add_space(3.0);
        ui.spacing_mut().item_spacing.y = 7.0;
    }
    if rail_group(ui, &mut open, "spaces", "Espacios", false) {
        ui.add_space(-5.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        if rail_sub(ui, "Crear espacio", false) {
            app.unavailable(
                "Crear espacio de trabajo",
                "Los espacios compartidos requieren una cuenta en la nube. En esta versión local, cada proyecto se guarda únicamente en este equipo.",
            );
        }
        if rail_sub(ui, "¿Qué son los espacios?", false) {
            app.unavailable(
                "Espacios",
                "Un espacio es una carpeta compartida para trabajar con otras personas. cupcraft no tiene cuentas ni colaboración: todo se queda en este equipo.",
            );
        }
        ui.add_space(3.0);
        ui.spacing_mut().item_spacing.y = 7.0;
    }
    if rail_group(ui, &mut open, "library", "Biblioteca", false) {
        ui.add_space(-5.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        if rail_sub(ui, "Medios guardados", false) {
            app.left_view = LeftView::Media;
            app.panel_tab = PanelTab::Media;
            app.media_filter = 0;
        }
        if rail_sub(ui, "Plantillas", false) {
            app.modal = Some(Modal::Templates);
        }
        ui.add_space(3.0);
        ui.spacing_mut().item_spacing.y = 7.0;
    }
    ui.data_mut(|d| d.insert_temp(gid, open));
}

fn view_head(ui: &mut egui::Ui, app: &mut CupApp, title: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::icon_button(ui, "info", 15.0, theme::MUTED, "Ayuda").clicked() {
                app.modal = Some(Modal::Shortcuts);
            }
        });
    });
    ui.add_space(6.0);
}

fn section_head(ui: &mut egui::Ui, label: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(crate::i18n::tr(label)).size(12.0).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(count.to_string()).size(12.0).strong().color(theme::TEXT));
        });
    });
    ui.add_space(4.0);
}

fn rail_action(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
    ui.painter().rect_filled(r, 6.0, if resp.hovered() { theme::PANEL3 } else { theme::PANEL2 });
    icons::paint(ui.painter(), icon, r.left_center() + Vec2::new(16.0, 0.0), 13.0, theme::TEXT);
    ui.painter().text(r.left_center() + Vec2::new(30.0, 0.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.5), theme::TEXT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn timeline_view(ui: &mut egui::Ui, app: &mut CupApp) {
    view_head(ui, app, "Línea de tiempo");
    let tracks: Vec<(String, String, usize, bool)> = app
        .editor
        .project()
        .map(|p| {
            p.tracks
                .iter()
                .map(|t| {
                    let ty = format!("{:?}", t.kind).to_lowercase();
                    (t.name.clone(), ty, t.clips.len(), t.hidden || t.muted)
                })
                .collect()
        })
        .unwrap_or_default();
    section_head(ui, "Pistas", tracks.len());
    egui::ScrollArea::vertical().max_height(ui.available_height() - 110.0).auto_shrink([false, true]).show(ui, |ui| {
        for (name, ty, n, hidden) in &tracks {
            egui::Frame::NONE.fill(theme::PANEL2).corner_radius(CornerRadius::from(6)).inner_margin(8).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let ic = match ty.as_str() {
                        "audio" => "music",
                        "text" => "text",
                        _ => "film",
                    };
                    icons::icon(ui, ic, 14.0, theme::MUTED);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(name).size(12.5).strong().color(theme::TEXT));
                        ui.label(RichText::new(format!("{n} clip{}", if *n == 1 { "" } else { "s" })).size(11.0).color(theme::MUTED));
                    });
                    if *hidden {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Oculta").size(11.0).color(theme::MUTED2));
                        });
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
    ui.add_space(6.0);
    if rail_action(ui, "plus", "Añadir texto en el cursor") {
        let t = app.playhead;
        let id = app.mutate_project(|p| p.add_text_clip("Texto", Some(t), 3.0, None)).flatten();
        if let Some(id) = id {
            app.editor.select(Some(id));
            app.toast("Texto añadido en el cursor");
        }
    }
    ui.add_space(6.0);
    panels::hint(ui, "Arrastra los clips para moverlos, usa los tiradores de los extremos para recortar y pulsa B para dividir en el cursor.");
}

fn secondary_view(ui: &mut egui::Ui, app: &mut CupApp) {
    view_head(ui, app, "Proyectos secundarios");
    let current = app.editor.project().map(|p| p.id.clone()).unwrap_or_default();
    let others: Vec<_> = app.list_projects().into_iter().filter(|p| p.id != current).collect();
    section_head(ui, "Proyectos secundarios", others.len());
    let mut open = None;
    egui::ScrollArea::vertical().max_height(ui.available_height() - 50.0).auto_shrink([false, true]).show(ui, |ui| {
        if others.is_empty() {
            panels::hint(ui, "No hay otros proyectos todavía. Crea uno nuevo desde el menú o vuelve al inicio.");
        }
        for p in &others {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::click());
            ui.painter().rect_filled(r, 6.0, if resp.hovered() { theme::PANEL3 } else { theme::PANEL2 });
            let thumb = Rect::from_min_size(r.min + Vec2::new(6.0, 6.0), Vec2::new(64.0, 40.0));
            ui.painter().rect_filled(thumb, 4.0, theme::BG);
            icons::paint(ui.painter(), "film", thumb.center(), 18.0, theme::MUTED2);
            let font = egui::FontId::proportional(12.5);
            let name = paint::ellipsize(ui.painter(), &p.name, &font, r.width() - 90.0);
            ui.painter().text(Pos2::new(thumb.right() + 10.0, r.top() + 10.0), egui::Align2::LEFT_TOP, name, font, theme::TEXT);
            ui.painter().text(
                Pos2::new(thumb.right() + 10.0, r.top() + 29.0),
                egui::Align2::LEFT_TOP,
                format!("{} · {}", p.ratio, p.resolution),
                egui::FontId::proportional(11.0),
                theme::MUTED,
            );
            if resp.on_hover_text("Abrir este proyecto").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                open = Some(p.id.clone());
            }
            ui.add_space(4.0);
        }
    });
    if let Some(id) = open {
        app.save_current();
        app.open_project(&id);
        return;
    }
    ui.add_space(6.0);
    if rail_action(ui, "home", "Ir al inicio") {
        app.back_home();
    }
}

// ----------------------------------------------------------------------
// Reproductor
// ----------------------------------------------------------------------

/// Capa preparada para pintar (copia de los datos del clip).
enum Layer {
    Media { key: String, path: Option<std::path::PathBuf>, image: bool, src_t: f64, dims: Option<(u32, u32)>, clip: cupcraft_core::Clip, alpha: f32, local: f64 },
    Graphic { val: serde_json::Value, text: bool, clip: cupcraft_core::Clip, alpha: f32, local: f64 },
}

type GfxCache = Mutex<HashMap<String, Option<(egui::TextureHandle, Vec2)>>>;

fn gfx_cache() -> &'static GfxCache {
    static C: OnceLock<GfxCache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Rasteriza texto o sticker con el mismo motor que la exportación.
fn graphic_texture(ctx: &egui::Context, val: &serde_json::Value, text: bool, scale: f32) -> Option<(egui::TextureHandle, Vec2)> {
    let key = format!("{text}:{:.3}:{val}", scale);
    let mut map = gfx_cache().lock().ok()?;
    if let Some(t) = map.get(&key) {
        return t.clone();
    }
    if map.len() > 256 {
        map.clear();
    }
    let pm = if text { raster::render_text(val, scale) } else { raster::render_sticker(val, scale) };
    let out = pm.map(|pm| {
        let img = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data());
        let size = Vec2::new(pm.width() as f32, pm.height() as f32);
        (ctx.load_texture("preview-gfx", img, egui::TextureOptions::LINEAR), size)
    });
    map.insert(key, out.clone());
    out
}

/// Pinta una textura como cuadrilátero rotado (tamaño con signo = volteo).
fn draw_quad(p: &egui::Painter, tex: egui::TextureId, center: Pos2, size: Vec2, rot_deg: f32, tint: Color32) {
    let (s, c) = rot_deg.to_radians().sin_cos();
    let rot = |v: Vec2| Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c);
    let hx = size.x / 2.0;
    let hy = size.y / 2.0;
    let corners = [Vec2::new(-hx, -hy), Vec2::new(hx, -hy), Vec2::new(hx, hy), Vec2::new(-hx, hy)];
    let uvs = [Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0)];
    let mut mesh = egui::Mesh::with_texture(tex);
    for (k, uv) in corners.iter().zip(uvs) {
        mesh.vertices.push(egui::epaint::Vertex { pos: center + rot(*k), uv, color: tint });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    p.add(egui::Shape::mesh(mesh));
}

/// Caja de una capa visual en pantalla (centro, medio tamaño, giro en grados).
struct Hit {
    id: String,
    center: Pos2,
    half: Vec2,
    rot: f32,
}

#[derive(Clone, Copy)]
enum DragKind {
    Move { mouse0: Pos2, x0: f64, y0: f64 },
    Scale { d0: f32, s0: f64 },
    Rotate { a0: f32, r0: f64 },
}

#[derive(Clone)]
struct DragState {
    id: String,
    kind: DragKind,
}

impl Default for DragState {
    fn default() -> Self {
        Self { id: String::new(), kind: DragKind::Scale { d0: 1.0, s0: 1.0 } }
    }
}

/// Lleva un punto de pantalla al sistema local de la caja (sin giro, centrado).
fn to_local(h: &Hit, pt: Pos2) -> Vec2 {
    let (s, c) = (-h.rot).to_radians().sin_cos();
    let v = pt - h.center;
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn from_local(h: &Hit, v: Vec2) -> Pos2 {
    let (s, c) = h.rot.to_radians().sin_cos();
    h.center + Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn contains(h: &Hit, pt: Pos2) -> bool {
    let l = to_local(h, pt);
    l.x.abs() <= h.half.x.max(6.0) && l.y.abs() <= h.half.y.max(6.0)
}

fn angle_of(center: Pos2, pt: Pos2) -> f32 {
    let v = pt - center;
    v.y.atan2(v.x).to_degrees()
}

/// Edición directa en el visor: clic selecciona, arrastrar mueve, esquina escala
/// y el círculo superior gira (como CapCut). Un clic en vacío reproduce/pausa.
fn stage_interaction(ui: &mut egui::Ui, app: &mut CupApp, p: &egui::Painter, stage: Rect, hits: &[Hit], (sw, sh): (f32, f32)) {
    let accent = theme::ACCENT;
    let sel_id = app.editor.selection().map(str::to_string);
    let sel_hit = sel_id.as_ref().and_then(|s| hits.iter().find(|h| &h.id == s));

    // Posiciones de los tiradores del seleccionado.
    const ROT_GAP: f32 = 26.0;
    let handles = sel_hit.map(|h| {
        let corner = from_local(h, h.half);
        let rotp = from_local(h, Vec2::new(0.0, -h.half.y - ROT_GAP));
        (corner, rotp)
    });

    let resp = ui.interact(stage, ui.scope_id().with("stage"), Sense::click_and_drag());
    let drag_id = ui.scope_id().with("stage-drag");
    let hover = resp.hover_pos();

    // Cursor según lo que haya bajo el puntero.
    if let Some(pt) = hover {
        if let Some((corner, rotp)) = handles {
            if pt.distance(rotp) < 10.0 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Alias);
            } else if pt.distance(corner) < 10.0 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            } else if hits.iter().any(|h| contains(h, pt)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        } else if hits.iter().any(|h| contains(h, pt)) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }

    // Inicio del arrastre: elegir qué se manipula.
    if resp.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin()).or(hover);
        if let Some(pt) = origin {
            let mut state: Option<DragState> = None;
            if let (Some(h), Some((corner, rotp))) = (sel_hit, handles) {
                let ph_t = app.playhead;
                let (cx, cy) = app.editor.project().and_then(|pj| pj.find_clip(&h.id).map(|(_, c)| (kf::value_at(c, "scale", ph_t), kf::value_at(c, "rotate", ph_t)))).unwrap_or((1.0, 0.0));
                if pt.distance(rotp) < 12.0 {
                    state = Some(DragState { id: h.id.clone(), kind: DragKind::Rotate { a0: angle_of(h.center, pt), r0: cy } });
                } else if pt.distance(corner) < 12.0 {
                    state = Some(DragState { id: h.id.clone(), kind: DragKind::Scale { d0: pt.distance(h.center).max(4.0), s0: cx } });
                }
            }
            if state.is_none() {
                // Objeto superior bajo el puntero (el último pintado está encima).
                let top = hits.iter().rev().find(|h| contains(h, pt));
                if let Some(h) = top {
                    let ph_t = app.playhead;
                    let (x0, y0) = app.editor.project().and_then(|pj| pj.find_clip(&h.id).map(|(_, c)| (kf::value_at(c, "x", ph_t), kf::value_at(c, "y", ph_t)))).unwrap_or((0.0, 0.0));
                    state = Some(DragState { id: h.id.clone(), kind: DragKind::Move { mouse0: pt, x0, y0 } });
                }
            }
            if let Some(st) = state {
                app.editor.select(Some(st.id.clone()));
                app.editor.mutate(|_| {}); // punto de deshacer único para todo el gesto
                ui.ctx().data_mut(|d| d.insert_temp(drag_id, st));
            }
        }
    }

    // Arrastre en curso.
    let active: Option<DragState> = ui.ctx().data(|d| d.get_temp(drag_id));
    if let Some(st) = active {
        if resp.dragged() {
            if let (Some(pt), Some(h)) = (resp.interact_pointer_pos(), hits.iter().find(|h| h.id == st.id)) {
                let shift = ui.input(|i| i.modifiers.shift);
                let center = h.center;
                let id = st.id.clone();
                let ph_t = app.playhead;
                match st.kind {
                    DragKind::Move { mouse0, x0, y0 } => {
                        let d = pt - mouse0;
                        let (mut nx, mut ny) = (x0 + (d.x / sw * 100.0) as f64, y0 + (d.y / sh * 100.0) as f64);
                        // Imán al centro del lienzo.
                        if !shift {
                            if nx.abs() < 1.5 {
                                nx = 0.0;
                            }
                            if ny.abs() < 1.5 {
                                ny = 0.0;
                            }
                        }
                        let (nx, ny) = (nx.clamp(-150.0, 150.0), ny.clamp(-150.0, 150.0));
                        app.editor.mutate_silent(|pj| {
                            if let Some(c) = pj.find_clip_mut(&id) {
                                // Con fotogramas clave, arrastrar crea/actualiza el del cursor.
                                kf::write_value(c, "x", ph_t, nx);
                                kf::write_value(c, "y", ph_t, ny);
                            }
                        });
                    }
                    DragKind::Scale { d0, s0 } => {
                        let ns = (s0 * (pt.distance(center) / d0) as f64).clamp(0.05, 8.0);
                        app.editor.mutate_silent(|pj| {
                            if let Some(c) = pj.find_clip_mut(&id) {
                                kf::write_value(c, "scale", ph_t, ns);
                            }
                        });
                    }
                    DragKind::Rotate { a0, r0 } => {
                        let mut nr = r0 + (angle_of(center, pt) - a0) as f64;
                        nr = (nr + 180.0).rem_euclid(360.0) - 180.0;
                        if !shift {
                            for snap in [-180.0, -90.0, 0.0, 90.0, 180.0] {
                                if (nr - snap).abs() < 3.0 {
                                    nr = snap;
                                }
                            }
                        }
                        app.editor.mutate_silent(|pj| {
                            if let Some(c) = pj.find_clip_mut(&id) {
                                kf::write_value(c, "rotate", ph_t, nr);
                            }
                        });
                    }
                }
                ui.ctx().request_repaint();
            }
        }
        if resp.drag_stopped() {
            ui.ctx().data_mut(|d| d.remove_temp::<DragState>(drag_id));
            app.save_current();
        }
    }

    // Clic simple: seleccionar el objeto bajo el puntero; en vacío, reproducir/pausar.
    if resp.clicked() {
        let pt = resp.interact_pointer_pos();
        let handle_hit = pt.zip(handles).map(|(pt, (c, r))| pt.distance(c) < 12.0 || pt.distance(r) < 12.0).unwrap_or(false);
        if !handle_hit {
            match pt.and_then(|pt| hits.iter().rev().find(|h| contains(h, pt))) {
                Some(h) => app.editor.select(Some(h.id.clone())),
                None => app.toggle_play(),
            }
        }
    }

    // Recuadro y tiradores del objeto seleccionado.
    let sel_id = app.editor.selection().map(str::to_string);
    if let Some(h) = sel_id.as_ref().and_then(|s| hits.iter().find(|h| &h.id == s)) {
        let pp = p.with_clip_rect(stage.expand(40.0));
        let pts = [Vec2::new(-h.half.x, -h.half.y), Vec2::new(h.half.x, -h.half.y), Vec2::new(h.half.x, h.half.y), Vec2::new(-h.half.x, h.half.y)].map(|v| from_local(h, v));
        let mut closed = pts.to_vec();
        closed.push(pts[0]);
        pp.add(egui::Shape::line(closed, Stroke::new(1.5, accent)));
        let top_mid = from_local(h, Vec2::new(0.0, -h.half.y));
        let rotp = from_local(h, Vec2::new(0.0, -h.half.y - ROT_GAP));
        pp.line_segment([top_mid, rotp], Stroke::new(1.0, accent));
        pp.circle(rotp, 6.0, Color32::WHITE, Stroke::new(1.5, accent));
        for (i, c) in pts.iter().enumerate() {
            if i == 2 {
                pp.circle(*c, 6.5, accent, Stroke::new(1.5, Color32::WHITE));
            } else {
                pp.rect_filled(Rect::from_center_size(*c, Vec2::splat(7.0)), 1.0, Color32::WHITE);
            }
        }
    }
}

fn ratio_of(r: &str) -> f32 {
    r.split_once(':')
        .and_then(|(a, b)| Some(a.trim().parse::<f32>().ok()? / b.trim().parse::<f32>().ok()?.max(0.01)))
        .filter(|v| *v > 0.05)
        .unwrap_or(16.0 / 9.0)
}

fn flag(clip: &cupcraft_core::Clip, key: &str) -> bool {
    clip.extra.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn player(ui: &mut egui::Ui, app: &mut CupApp) {
    let full = ui.max_rect();
    // Cabecera "Reproductor"
    let head = Rect::from_min_size(full.min, Vec2::new(full.width(), 34.0));
    ui.painter().text(head.left_center() + Vec2::new(14.0, 0.0), egui::Align2::LEFT_CENTER, "Reproductor-Línea de tiempo 01", egui::FontId::proportional(12.0), theme::MUTED);
    let transport_h = 46.0;
    let area = Rect::from_min_max(Pos2::new(full.left(), head.bottom()), Pos2::new(full.right(), full.bottom() - transport_h)).shrink2(Vec2::new(16.0, 8.0));
    let transport = Rect::from_min_max(Pos2::new(full.left(), full.bottom() - transport_h), full.max);

    let Some(project) = app.editor.project() else { return };
    let ratio_s = project.ratio.clone();
    let ar = ratio_of(&ratio_s);
    let (pw, ph) = crate::services::compose::canvas_size(&ratio_s, 1080);
    let res = project.resolution.clone();
    let fps = project.fps.max(1);
    let dur = project.duration();
    let t = app.playhead;
    let playing = app.playing;
    let orig_dims = project.media.iter().find_map(|m| media::media_dims(m));

    let sw = area.width().min(area.height() * ar).max(10.0);
    let sh = sw / ar;
    let stage = Rect::from_center_size(area.center(), Vec2::new(sw, sh));

    // Copia de capas activas (libera el préstamo del proyecto).
    let mut layers = Vec::new();
    for (clip, local) in active_layers(project, t) {
        let alpha = clip_alpha(clip, local);
        if alpha <= 0.001 {
            continue;
        }
        match clip.kind {
            ClipKind::Video | ClipKind::Image => {
                let m = clip.media_id.as_deref().and_then(|mid| project.media.iter().find(|m| m.id == mid));
                let Some(m) = m else { continue };
                layers.push(Layer::Media {
                    key: format!("pv:{}", clip.id),
                    path: app.media_path(m),
                    image: clip.kind == ClipKind::Image,
                    src_t: (clip.in_point + paint::effect_local(clip, local) * clip.speed.max(0.01)).max(0.0),
                    dims: media::media_dims(m),
                    clip: clip.clone(),
                    alpha,
                    local,
                });
            }
            ClipKind::Text | ClipKind::Sticker => {
                let (val, text) = if clip.kind == ClipKind::Text { (clip.text.clone(), true) } else { (clip.sticker.clone(), false) };
                if let Some(val) = val {
                    layers.push(Layer::Graphic { val, text, clip: clip.clone(), alpha, local });
                }
            }
            _ => {}
        }
    }

    // Transición en curso bajo el cursor: se compone con la misma tubería que el
    // export (barridos, empujes, destellos, clip anterior debajo).
    let comp_project = if crate::services::compose::transition_active(project, t) || crate::services::compose::texture_filter_active(project, t) || crate::services::compose::shape_or_retouch_active(project, t) { Some(project.clone()) } else { None };

    let ctx = ui.ctx().clone();
    let ppp = ctx.pixels_per_point();
    let p = ui.painter_at(area.expand(8.0));
    p.rect_filled(stage, 0.0, Color32::BLACK);
    let mut composed = false;
    if let Some(pj) = comp_project {
        if let Some(id) = app.preview_composite(&ctx, &pj, t) {
            p.image(id, stage, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            composed = true;
        }
    }
    let k = sw / pw as f32; // puntos de pantalla por píxel del lienzo
    let qf = [1.0_f32, 0.75, 0.5, 0.33][app.preview_quality.min(3) as usize];
    let max_w = ((sw * ppp * qf) as u32).clamp(64, 1280);
    let max_h = ((sh * ppp * qf) as u32).clamp(64, 1280);

    let painter = p.with_clip_rect(stage);
    // Cajas de cada capa visual (de abajo a arriba) para seleccionar y editar en pantalla.
    let mut hits: Vec<Hit> = Vec::new();
    for layer in layers.iter().filter(|_| !composed) {
        match layer {
            Layer::Media { key, path, image, src_t, dims, clip, alpha, local } => {
                if let Some(path) = path {
                    if *image {
                        if let Some(anim) = crate::services::gif_anim::load(path) {
                            let (idx, rgba) = anim.frame_at(*src_t);
                            app.video.push_frame(key, anim.w, anim.h, rgba, idx as u64 + 1);
                            if playing {
                                ctx.request_repaint();
                            }
                        } else if app.video.texture(&ctx, key).is_none() {
                            app.video.request_image(key, path);
                        }
                    } else {
                        let (w, h) = video::fit_dims(*dims, max_w, max_h);
                        app.video.request_video(key, path, *src_t, w, h, playing);
                    }
                }
                // Cadena de color real del clip (filtro + ajuste + efecto + IA +
                // curvas). Si tiene alguna etapa visible, se hornea por píxel en
                // la textura (igual que el export); si no, textura cruda y tinte
                // blanco. Así el panel de detalles y los filtros se ven en vivo.
                let chain = paint::clip_chain(clip);
                // Deformación geométrica real del efecto activo (remolino,
                // pellizco, ondas…), compartida con el export.
                let warp = paint::clip_warp_at(clip, *local);
                let ai = crate::services::ai_local::AiParams::from_clip(clip).map(|p| crate::services::ai_local::AiCtx {
                    id: format!("p:{}", clip.id),
                    local: *local,
                    dur: clip.duration,
                    p,
                });
                let fx_active = warp.is_some() || ai.is_some() || chain.iter().any(|fx| !fx.is_identity());
                let got = if fx_active { app.video.texture_fx(&ctx, key, &chain, warp, ai) } else { app.video.texture(&ctx, key) };
                let Some((tex, size)) = got else { continue };
                // Dimensiones reales del medio (la textura puede venir reducida).
                let (mw, mh) = dims.map(|(w, h)| (w as f32, h as f32)).unwrap_or((size.x, size.y));
                let fit = (pw as f32 / mw).min(ph as f32 / mh);
                // Posición, escala, giro con fotogramas clave en este instante.
                let tr = crate::services::compose::transform_at(clip, clip.start + *local);
                let mut sz = Vec2::new(mw * fit * k * tr.scale_x as f32, mh * fit * k * tr.scale_y as f32);
                if flag(clip, "flipH") {
                    sz.x = -sz.x;
                }
                if flag(clip, "flipV") {
                    sz.y = -sz.y;
                }
                let center = stage.center() + Vec2::new(tr.x as f32 / 100.0 * sw, tr.y as f32 / 100.0 * sh);
                // El color ya va horneado en los píxeles cuando `fx_active`; solo
                // queda la opacidad del clip. Sin cadena, tinte blanco = idéntico.
                let ca = crate::clip_anim::eval(clip, *local);
                let center = center + Vec2::new(ca.dx * sw, ca.dy * sh);
                let sz = sz * ca.scale;
                let tint = Color32::from_white_alpha((alpha * ca.alpha * 255.0) as u8);
                let neg = Vec2::new(sz.x.signum(), sz.y.signum());
                hits.push(Hit { id: clip.id.clone(), center, half: sz.abs() / 2.0, rot: tr.rotate as f32 });
                draw_quad(&painter, tex, center, Vec2::new(sz.x.abs(), sz.y.abs()) * neg, tr.rotate as f32 + ca.rot, tint);
            }
            Layer::Graphic { val, text, clip, alpha, local } => {
                // Misma escala que el compositor: alto del lienzo / 1080,
                // convertida a píxeles físicos de pantalla.
                let scale = (ph as f32 / 1080.0) * k * ppp;
                let fxv = if *text { crate::fx_text::apply(val, clip, *local) } else { None };
                let val = fxv.as_ref().unwrap_or(val);
                // Karaoke: la palabra activa en su color (igual que la exportación).
                let kv = if *text { Some(crate::karaoke::apply(val, *local)) } else { None };
                let val = kv.as_ref().unwrap_or(val);
                let anim = if *text { raster::text_anim(val, *local, clip.duration) } else { None };
                let shown = match anim {
                    Some(a) if a.reveal < 0.999 => raster::reveal_text(val, a.reveal),
                    _ => val.clone(),
                };
                let alpha = *alpha * anim.map(|a| a.alpha).unwrap_or(1.0);
                if alpha <= 0.002 {
                    continue;
                }
                let Some((tex, size)) = graphic_texture(&ctx, &shown, *text, scale) else { continue };
                let vx = val.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let vy = val.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let vrot = if *text { 0.0 } else { val.get("rotate").and_then(|v| v.as_f64()).unwrap_or(0.0) };
                let tr = crate::services::compose::transform_at(clip, clip.start + *local);
                let mut center = stage.center() + Vec2::new((tr.x + vx) as f32 / 100.0 * sw, (tr.y + vy) as f32 / 100.0 * sh);
                let mut sz = size / ppp * tr.scale_x as f32;
                let mut rot = (tr.rotate + vrot) as f32;
                hits.push(Hit { id: clip.id.clone(), center, half: sz / 2.0, rot });
                if let Some(a) = anim {
                    // Desplazamiento en tamaños de fuente, girado con el clip (igual que el canvas).
                    let fpx = raster::text_font_px(val, scale) / ppp * tr.scale_x as f32 * a.scale;
                    let (sn, cs) = rot.to_radians().sin_cos();
                    let (ox, oy) = (a.dx * fpx, a.dy * fpx);
                    center += Vec2::new(ox * cs - oy * sn, ox * sn + oy * cs);
                    sz *= a.scale;
                    rot += a.rotate;
                }
                let ca = crate::clip_anim::eval(clip, *local);
                center += Vec2::new(ca.dx * sw, ca.dy * sh);
                sz *= ca.scale;
                rot += ca.rot;
                let tint = Color32::from_white_alpha((alpha * ca.alpha * 255.0) as u8);
                draw_quad(&painter, tex.id(), center, sz, rot, tint);
            }
        }
    }
    if layers.is_empty() {
        p.text(stage.center() - Vec2::new(0.0, 10.0), egui::Align2::CENTER_CENTER, format!("{pw} × {ph} · {ratio_s}"), egui::FontId::proportional(12.0), theme::MUTED2);
        p.text(stage.center() + Vec2::new(0.0, 10.0), egui::Align2::CENTER_CENTER, "Añade medios o texto para previsualizar", egui::FontId::proportional(11.0), theme::MUTED2);
    }
    stage_interaction(ui, app, &p, stage, &hits, (sw, sh));

    // ---------------- Transporte (`.preview-controls`) ----------------
    // Izquierda: código de tiempo + silenciar. Centro: reproducir.
    // Derecha: "Completa" (ajustar), "Relación" (cambia la proporción) y pantalla completa.
    let mut tui = ui.new_child(egui::UiBuilder::new().max_rect(transport).layout(egui::Layout::left_to_right(egui::Align::Center)));
    tui.painter().hline(transport.x_range(), transport.top(), Stroke::new(1.0, theme::LINE));
    tui.spacing_mut().item_spacing.x = 5.0;
    tui.add_space(12.0);
    let tc = egui::FontId::monospace(10.5);
    tui.label(RichText::new(format_timecode(t, fps)).font(tc.clone()).color(if playing { theme::ACCENT2 } else { Color32::from_rgb(0xd0, 0xd0, 0xd0) }));
    tui.label(RichText::new("/").font(tc.clone()).color(Color32::from_rgb(0x5f, 0x5f, 0x5f)));
    tui.label(RichText::new(format_timecode(dur, fps)).font(tc).color(Color32::from_rgb(0x8f, 0x8f, 0x8f)));
    let muted = app.muted;
    if icons::icon_button_sized(&mut tui, if muted { "mute" } else { "volume" }, 14.0, Vec2::splat(22.0), theme::MUTED, if muted { "Activar sonido" } else { "Silenciar" }, muted).clicked() {
        app.muted = !muted;
        if let Some(eng) = &app.audio_engine {
            eng.set_muted(app.muted);
        }
    }
    // Botón de reproducir centrado (`.pc-center`, círculo de 26 px).
    let pr = Rect::from_center_size(transport.center(), Vec2::splat(30.0));
    let presp = ui.interact(pr, ui.scope_id().with("play-btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if presp.hovered() {
        ui.painter().circle_filled(pr.center(), 15.0, Color32::from_white_alpha(26));
    }
    let pc = if playing { theme::ACCENT2 } else if presp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xbd, 0xbd, 0xbd) };
    icons::paint(ui.painter(), if playing { "pause" } else { "play" }, pr.center(), 16.0, pc);
    if presp.on_hover_text(if playing { "Pausar (Espacio)" } else { "Reproducir (Espacio)" }).clicked() {
        app.toggle_play();
    }
    tui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.add_space(10.0);
        let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if icons::icon_button_sized(ui, "fullscreen", 13.0, Vec2::splat(22.0), theme::MUTED, "Pantalla completa", fullscreen).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }
        let r1 = icons::icon_button_sized(ui, "aspect", 13.0, Vec2::splat(22.0), theme::MUTED, "Cambiar relación de aspecto", false);
        let r2 = text_ctrl(ui, "Relación", "Cambiar relación de aspecto");
        ratio_menu(app, &r1, &ratio_s, orig_dims);
        ratio_menu(app, &r2, &ratio_s, orig_dims);
        let q1 = icons::icon_button_sized(ui, "fit", 13.0, Vec2::splat(22.0), theme::MUTED, "Calidad de la vista previa", false);
        let q2 = text_ctrl(ui, QUALITIES[app.preview_quality.min(3) as usize].0, "Calidad de la vista previa");
        quality_menu(app, &q1);
        quality_menu(app, &q2);
        let _ = (&res, pw, ph);
    });
}

/// `.text-ctrl`: botón de texto con borde fino (9 px).
const QUALITIES: [(&str, &str, &str); 4] = [
    ("Completa", "Máxima calidad", "Resolución del video original"),
    ("Alta", "Alta calidad", "Reproducción fluida sin que el video exportado se vea afectado"),
    ("Media", "Calidad media", "Reproducción más fluida sin que el video exportado se vea afectado"),
    ("Baja", "Baja calidad", "La reproducción es más fluida sin que el video exportado se vea afectado"),
];

/// Fila de menú con marca de selección, texto y subtítulo opcional.
fn pick_row(ui: &mut egui::Ui, checked: bool, label: &str, sub: &str, shape: Option<(f32, f32)>) -> bool {
    let h = if sub.is_empty() { 28.0 } else { 40.0 };
    let w = if sub.is_empty() { 190.0 } else { 400.0 };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, Color32::from_rgb(0x3a, 0x3a, 0x3a));
    }
    if checked {
        ui.painter().text(r.left_center() + Vec2::new(12.0, 0.0), egui::Align2::CENTER_CENTER, "✓", egui::FontId::proportional(13.0), theme::ACCENT2);
    }
    let ty = if sub.is_empty() { r.center().y } else { r.top() + 13.0 };
    ui.painter().text(Pos2::new(r.left() + 34.0, ty), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.0), Color32::WHITE);
    if !sub.is_empty() {
        ui.painter().text(Pos2::new(r.left() + 34.0, r.top() + 28.0), egui::Align2::LEFT_CENTER, sub, egui::FontId::proportional(10.0), theme::MUTED);
    }
    if let Some((a, b)) = shape {
        let k = 16.0 / a.max(b);
        let s = Vec2::new(a * k, b * k).max(Vec2::splat(4.0));
        let sr = Rect::from_center_size(Pos2::new(r.right() - 22.0, r.center().y), s);
        ui.painter().rect_stroke(sr, 1.5, Stroke::new(1.0, theme::MUTED), StrokeKind::Inside);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn ratio_menu(app: &mut CupApp, anchor: &egui::Response, current: &str, orig: Option<(u32, u32)>) {
    let cur = ratio_of(current);
    let same = |v: &str| (ratio_of(v) - cur).abs() < 0.005;
    let orig_s = orig.map(|(w, h)| format!("{w}:{h}"));
    let mut chosen: Option<String> = None;
    let mut custom = false;
    egui::Popup::menu(anchor).show(|ui| {
        style_menu(ui);
        let rows: [(&str, &str, bool); 9] = [
            ("16:9", "16:9", true),
            ("4:3", "4:3", true),
            ("2.35:1", "2.35:1", true),
            ("2:1", "2:1", true),
            ("1.85:1", "1.85:1", true),
            ("9:16", "9:16", true),
            ("3:4", "3:4", true),
            ("5.8 pulgadas", "9:19.5", true),
            ("1:1", "1:1", true),
        ];
        let is_orig = orig_s.as_deref().map(|o| same(o)).unwrap_or(false);
        let any = is_orig || rows.iter().any(|(_, v, _)| same(v));
        if pick_row(ui, is_orig, "Original", "", None) {
            chosen = Some(orig_s.clone().unwrap_or_else(|| "16:9".into()));
        }
        if pick_row(ui, !any, "Personalizado", "", None) {
            custom = true;
        }
        for (i, (label, v, _)) in rows.iter().enumerate() {
            if i == 0 || i == 5 {
                ui.separator();
            }
            let shape = {
                let (a, b) = v.split_once(':').map(|(a, b)| (a.parse().unwrap_or(16.0), b.parse().unwrap_or(9.0))).unwrap_or((16.0, 9.0));
                Some((a, b))
            };
            if pick_row(ui, !is_orig && same(v), label, "", shape) {
                chosen = Some(v.to_string());
            }
        }
    });
    if custom {
        app.custom_ratio_open = true;
    }
    if let Some(next) = chosen {
        apply_ratio(app, next);
    }
}

fn apply_ratio(app: &mut CupApp, next: String) {
    let n2 = next.clone();
    app.editor.mutate(move |p| p.ratio = n2);
    app.sync_engine();
    app.save_current();
    app.toast(&format!("Relación de aspecto: {next}"));
}

fn quality_menu(app: &mut CupApp, anchor: &egui::Response) {
    let mut chosen: Option<u8> = None;
    let cur = app.preview_quality;
    egui::Popup::menu(anchor).show(|ui| {
        style_menu(ui);
        for (i, (_, label, sub)) in QUALITIES.iter().enumerate() {
            if pick_row(ui, cur as usize == i, label, sub, None) {
                chosen = Some(i as u8);
            }
        }
    });
    if let Some(q) = chosen {
        app.preview_quality = q;
        app.toast(QUALITIES[q as usize].1);
    }
}

/// Diálogo para introducir una relación de aspecto personalizada.
pub fn custom_ratio_dialog(ctx: &egui::Context, app: &mut CupApp) {
    if !app.custom_ratio_open {
        return;
    }
    let mut open = true;
    let mut apply = false;
    let (mut w, mut h) = app.custom_ratio;
    egui::Window::new("Relación personalizada").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut w).range(0.1..=100.0).speed(0.1));
            ui.label(":");
            ui.add(egui::DragValue::new(&mut h).range(0.1..=100.0).speed(0.1));
        });
        if ui.button("Aplicar").clicked() {
            apply = true;
        }
    });
    app.custom_ratio = (w, h);
    if apply {
        apply_ratio(app, format!("{w}:{h}"));
        open = false;
    }
    app.custom_ratio_open = open;
}

fn text_ctrl(ui: &mut egui::Ui, label: &str, tip: &str) -> egui::Response {
    let font = egui::FontId::proportional(9.5);
    let g = ui.painter().layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 10.0, 16.0), Sense::click());
    let c = if resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xb5, 0xb5, 0xb5) };
    ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_white_alpha(56)), StrokeKind::Inside);
    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label, font, c);
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ----------------------------------------------------------------------
// Atajos de teclado (igual que `shortcuts.js`)
// ----------------------------------------------------------------------

fn handle_shortcuts(ctx: &egui::Context, app: &mut CupApp) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    use egui::Key;
    let (ctrl, shift) = ctx.input(|i| (i.modifiers.command, i.modifiers.shift));
    let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));

    if ctrl && (shift && pressed(Key::Z) || pressed(Key::Y)) {
        if app.editor.redo() {
            app.sync_engine();
            app.save_current();
            app.toast("Rehecho");
        }
        return;
    }
    if ctrl && pressed(Key::Z) {
        if app.editor.undo() {
            app.sync_engine();
            app.save_current();
            app.toast("Deshecho");
        }
        return;
    }
    if ctrl && pressed(Key::S) {
        app.save_current();
        app.toast("Proyecto guardado");
        return;
    }
    if ctrl && pressed(Key::I) {
        app.import_media_dialog();
        return;
    }
    if ctrl && pressed(Key::B) {
        let ph = app.playhead;
        let sel = app.editor.selection().map(str::to_string);
        app.mutate_project(|p| {
            p.split_at(ph, sel.as_deref());
        });
        app.toast("Dividido");
        return;
    }
    if ctrl && pressed(Key::D) {
        if let Some(id) = app.editor.selection().map(str::to_string) {
            if let Some(did) = app.mutate_project(|p| p.duplicate_clip(&id)).flatten() {
                app.editor.select(Some(did));
                app.toast("Clip duplicado");
            }
        }
        return;
    }
    if ctrl {
        return;
    }
    if pressed(Key::Space) {
        app.toggle_play();
    }
    if shift && pressed(Key::Z) {
        if let Some(p) = app.editor.project() {
            let dur = p.timeline_length().max(1.0);
            app.zoom = (900.0 / (dur * 80.0)).clamp(0.25, 8.0);
            app.toast("Zoom ajustado a ventana");
        }
        return;
    }
    if pressed(Key::V) {
        app.tool_mode = ToolMode::Select;
        app.toast("Herramienta: Selección (V)");
    }
    if pressed(Key::B) {
        app.tool_mode = ToolMode::Split;
        app.toast("Herramienta: Cuchilla (B)");
    }
    if pressed(Key::Q) {
        let ph = app.playhead;
        let sel = app.editor.selection().map(str::to_string);
        if app.mutate_project(|p| p.trim_left_at(ph, sel.as_deref())).unwrap_or(false) {
            app.toast("Recortado inicio (Q)");
        }
    }
    if pressed(Key::W) {
        let ph = app.playhead;
        let sel = app.editor.selection().map(str::to_string);
        if app.mutate_project(|p| p.trim_right_at(ph, sel.as_deref())).unwrap_or(false) {
            app.toast("Recortado final (W)");
        }
    }
    if pressed(Key::Delete) || pressed(Key::Backspace) {
        if let Some(id) = app.editor.selection().map(str::to_string) {
            app.mutate_project(|p| {
                p.remove_clip(&id);
            });
            app.editor.select(None);
            app.toast("Clip eliminado");
        }
    }
    if pressed(Key::M) {
        let ph = app.playhead;
        let added = app.mutate_project(|p| p.toggle_marker(ph)).unwrap_or(false);
        app.toast(if added { "Marcador añadido (M)" } else { "Marcador eliminado (M)" });
    }
    if pressed(Key::N) {
        app.snap = !app.snap;
        app.toast(if app.snap { "Imán activado" } else { "Imán desactivado" });
    }
    let step = 1.0 / app.editor.project().map(|p| p.fps.max(1)).unwrap_or(30) as f64;
    if pressed(Key::ArrowLeft) {
        app.seek(app.playhead - if shift { 1.0 } else { step });
    }
    if pressed(Key::ArrowRight) {
        app.seek(app.playhead + if shift { 1.0 } else { step });
    }
    if pressed(Key::Home) {
        app.seek(0.0);
    }
    if pressed(Key::End) {
        let d = app.editor.project().map(|p| p.duration()).unwrap_or(0.0);
        app.seek(d);
    }
    if pressed(Key::Escape) {
        if app.tool_mode == ToolMode::Split {
            app.tool_mode = ToolMode::Select;
        } else {
            app.editor.select(None);
        }
    }
}
