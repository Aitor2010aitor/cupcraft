//! Pantalla de inicio nativa (equivalente 1:1 de `src/editor/home.js`):
//! barra lateral, barra de herramientas con controles de ventana, botón
//! «Crear proyecto», tarjetas de IA con EditPilot y cuadrícula de proyectos.

use crate::app::{CupApp, HomeSort, Modal, PanelTab};
use crate::format::{format_time, time_ago};
use crate::icons;
use crate::paint;
use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use serde_json::json;

/// Paleta específica de la pantalla de inicio (de `styles.css`).
mod colors {
    use eframe::egui::Color32;
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x04, 0x11, 0x13);
    pub const SIDEBAR_USER: Color32 = Color32::from_rgb(0x0a, 0x21, 0x24);
    pub const SIDEBAR_BORDER: Color32 = Color32::from_rgb(0x13, 0x32, 0x36);
    pub const SIDEBAR_ACTIVE: Color32 = Color32::from_rgb(0x0e, 0x2b, 0x2f);
    pub const MAIN_BG: Color32 = Color32::from_rgb(0x05, 0x0f, 0x11);
    pub const CARD_BG: Color32 = Color32::from_rgb(0x0e, 0x1c, 0x1e);
    pub const CARD_BORDER: Color32 = Color32::from_rgb(0x18, 0x28, 0x2a);
    pub const CARD_HOVER: Color32 = Color32::from_rgb(0x12, 0x23, 0x26);
    pub const HERO_CYAN_1: Color32 = Color32::from_rgb(0x79, 0xe7, 0xf2);
    pub const HERO_CYAN_2: Color32 = Color32::from_rgb(0x1a, 0xb1, 0xcf);
    pub const EDITPILOT_1: Color32 = Color32::from_rgb(0x05, 0x62, 0x70);
    pub const EDITPILOT_2: Color32 = Color32::from_rgb(0x03, 0x3c, 0x47);
    pub const PROMO_BLUE_1: Color32 = Color32::from_rgb(0x0a, 0x6a, 0xa8);
    pub const PROMO_BLUE_2: Color32 = Color32::from_rgb(0x07, 0x1e, 0x3d);
    pub const SIDE_TEXT: Color32 = Color32::from_rgb(0xc8, 0xd5, 0xd6);
}

const AI_MSG: &str = "La generación y el procesamiento con IA necesitan un servicio en la nube. Esta versión local incluye EditPilot como borrador de texto y todas las herramientas de edición manual.";
const GITHUB_URL: &str = "https://github.com/Aitor2010aitor/cupcraft";
const DISCORD_URL: &str = "https://discord.gg/N2kfVGjeZ7";

fn new_project_modal() -> Modal {
    Modal::NewProject { cat: 0, tpl: 0, quality: 2, fps: 1, name: "Proyecto sin título".into() }
}

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let ctx = ui.ctx().clone();
    let screen = ui.max_rect();
    let mut scroll_to: Option<f32> = None; // 0 = arriba, 1 = proyectos
    let wr = crate::editor::window_radius(&ctx);

    egui::Panel::left("home-sidebar")
        .exact_size(216.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(colors::SIDEBAR_BG).stroke(Stroke::new(1.0, colors::SIDEBAR_BORDER)).corner_radius(CornerRadius { nw: wr, sw: wr, ne: 0, se: 0 }).inner_margin(egui::Margin::symmetric(12, 12)))
        .show(ui, |ui| sidebar(ui, app, &mut scroll_to));

    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(colors::MAIN_BG).corner_radius(CornerRadius { nw: 0, sw: 0, ne: wr, se: wr })).show(ui, |ui| {
        // Barra de herramientas superior (arrastre de ventana + iconos).
        let bar = Rect::from_min_size(ui.max_rect().min, Vec2::new(ui.max_rect().width(), 44.0));
        crate::editor::drag_region(ui, bar);
        let mut bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::right_to_left(egui::Align::Center)));
        toolbar(&mut bar_ui, app);
        ui.add_space(44.0);

        let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("home-scroll");
        if scroll_to == Some(0.0) {
            area = area.vertical_scroll_offset(0.0);
        }
        area.show(ui, |ui| {
            let side = ((ui.available_width() - 1100.0) / 2.0).max(28.0);
            egui::Frame::NONE.inner_margin(egui::Margin { left: side as i8, right: side.min(120.0) as i8, top: 6, bottom: 40 }).show(ui, |ui| {
                ui.set_max_width(1100.0);
                hero(ui, app);
                ui.add_space(18.0);
                ai_grid(ui, app);
                ui.add_space(30.0);
                let head = projects_head(ui, app);
                if scroll_to == Some(1.0) {
                    head.scroll_to_me(Some(egui::Align::TOP));
                }
                ui.add_space(12.0);
                projects_grid(ui, app);
            });
        });
    });

    crate::editor::resize_edges(&ctx, screen);
}

// ----------------------------------------------------------------------
// Barra lateral
// ----------------------------------------------------------------------

fn side_item(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
    if active {
        ui.painter().rect_filled(r, 7.0, colors::SIDEBAR_ACTIVE);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 7.0, Color32::from_white_alpha(10));
    }
    let c = if active || resp.hovered() { Color32::WHITE } else { colors::SIDE_TEXT };
    icons::paint(ui.painter(), icon, r.left_center() + Vec2::new(18.0, 0.0), 16.0, c);
    ui.painter().text(r.left_center() + Vec2::new(36.0, 0.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.5), c);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn side_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(crate::i18n::tr(&text)).size(10.5).color(theme::MUTED2));
    ui.add_space(2.0);
}

fn side_sep(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let y = ui.cursor().top();
    ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, colors::SIDEBAR_BORDER));
    ui.add_space(7.0);
}

/// Copia un enlace externo al portapapeles (la app no abre navegadores).
fn copy_link(ui: &egui::Ui, app: &mut CupApp, what: &str, url: &str) {
    ui.ctx().copy_text(url.to_string());
    app.toast(format!("Enlace de {what} copiado al portapapeles: {url}"));
}

fn sidebar(ui: &mut egui::Ui, app: &mut CupApp, scroll_to: &mut Option<f32>) {
    ui.spacing_mut().item_spacing.y = 2.0;
    // Marca
    let (br, bresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
    icons::paint_logo(ui.painter(), Rect::from_center_size(br.left_center() + Vec2::new(14.0, 0.0), Vec2::splat(24.0)), Color32::WHITE);
    ui.painter().text(br.left_center() + Vec2::new(34.0, 0.0), egui::Align2::LEFT_CENTER, "cupcraft", egui::FontId::proportional(15.5), theme::TEXT);
    if bresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *scroll_to = Some(0.0);
    }
    ui.add_space(10.0);

    // Tarjeta de usuario
    egui::Frame::NONE
        .fill(colors::SIDEBAR_USER)
        .stroke(Stroke::new(1.0, colors::SIDEBAR_BORDER))
        .corner_radius(CornerRadius::from(9))
        .inner_margin(egui::Margin::symmetric(10, 9))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (ar, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(ar, 14.0, Color32::from_rgb(0x22, 0x2f, 0x32));
                icons::paint_logo(ui.painter(), ar.shrink(5.0), Color32::WHITE);
                ui.label(RichText::new(crate::i18n::tr("Iniciar sesión")).size(12.0).strong().color(theme::TEXT));
            });
        });
    ui.add_space(10.0);

    if side_item(ui, "home", "Inicio", true).clicked() {
        *scroll_to = Some(0.0);
    }
    if side_item(ui, "templates", "Plantillas", false).clicked() {
        app.modal = Some(Modal::Templates);
    }
    side_sep(ui);
    side_label(ui, "Crear con IA");
    if side_item(ui, "studio", "Estudio de video", false).clicked() {
        app.modal = Some(new_project_modal());
    }
    if side_item(ui, "design", "Estudio de diseño", false).clicked() {
        app.modal = Some(Modal::Templates);
    }
    let more = side_item(ui, "grid", "Más herramientas", false);
    icons::paint(ui.painter(), if app.more_tools_open { "chevronUp" } else { "chevronDown" }, more.rect.right_center() - Vec2::new(14.0, 0.0), 12.0, theme::MUTED);
    if more.clicked() {
        app.more_tools_open = !app.more_tools_open;
    }
    if app.more_tools_open {
        ui.indent("more-tools", |ui| {
            if side_item(ui, "sparkles", "Crear con IA", false).clicked() {
                *scroll_to = Some(0.0);
                app.editpilot_focus = true;
            }
            if side_item(ui, "wand", "Herramientas de edición", false).clicked() {
                app.unavailable("Funciones de IA", AI_MSG);
            }
        });
    }
    side_sep(ui);
    side_label(ui, "Administración");
    if side_item(ui, "library", "Biblioteca", false).clicked() {
        app.unavailable("Biblioteca", "Abre un proyecto para ver su biblioteca de medios local. No hay contenido en la nube.");
    }
    if side_item(ui, "folderOpen", "Proyectos de CapCut", false).clicked() {
        app.open_capcut();
    }
    let bridge = side_item(ui, "link", "Puente IA", false).on_hover_text(crate::i18n::tr("Puerto local para IA"));
    ui.painter().circle_filled(bridge.rect.right_center() - Vec2::new(14.0, 0.0), 3.5, Color32::from_rgb(0x25, 0xd4, 0x90));
    if bridge.clicked() {
        app.modal = Some(Modal::Bridge);
    }
    if side_item(ui, "github", "GitHub", false).on_hover_text(crate::i18n::tr("Repositorio en GitHub (copia el enlace)")).clicked() {
        copy_link(ui, app, "GitHub", GITHUB_URL);
    }
    if side_item(ui, "discord", "Discord", false).on_hover_text(crate::i18n::tr("Servidor de Discord de cupcraft (copia el enlace)")).clicked() {
        copy_link(ui, app, "Discord", DISCORD_URL);
    }

    // Pie
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        let (pr, presp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 66.0), Sense::click());
        gradient_h(ui.painter(), pr, 9, colors::PROMO_BLUE_1, colors::PROMO_BLUE_2);
        ui.painter().text(pr.left_top() + Vec2::new(12.0, 15.0), egui::Align2::LEFT_TOP, "¿Quieres alargarlo?", egui::FontId::proportional(12.5), Color32::from_rgb(0x5a, 0xdc, 0xf2));
        ui.painter().text(pr.left_top() + Vec2::new(12.0, 36.0), egui::Align2::LEFT_TOP, "Amplía tu video con IA", egui::FontId::proportional(11.0), Color32::from_rgb(0xc8, 0xe8, 0xf8));
        if presp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            app.modal = Some(Modal::Pro);
        }
        let (ir, iresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
        ui.painter().rect(
            ir,
            7.0,
            if iresp.hovered() { Color32::from_rgb(0x0c, 0x23, 0x26) } else { Color32::from_rgb(0x08, 0x17, 0x19) },
            Stroke::new(1.0, Color32::from_rgb(0x1d, 0x3a, 0x3d)),
            StrokeKind::Inside,
        );
        icons::paint(ui.painter(), "user", ir.left_center() + Vec2::new(16.0, 0.0), 15.0, theme::TEXT);
        ui.painter().text(ir.left_center() + Vec2::new(32.0, 0.0), egui::Align2::LEFT_CENTER, "Invitar a amigos", egui::FontId::proportional(12.0), theme::TEXT);
        let badge = Rect::from_center_size(ir.right_center() - Vec2::new(26.0, 0.0), Vec2::new(38.0, 16.0));
        ui.painter().rect_filled(badge, 8.0, Color32::from_rgb(0xff, 0xd7, 0x4a));
        ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, "NUEVO", egui::FontId::proportional(8.5), Color32::from_rgb(0x22, 0x1b, 0x00));
        if iresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            app.unavailable("Invitar a amigos", "cupcraft funciona sin cuentas. Comparte el proyecto exportado como archivo de video para colaborar.");
        }
    });
}

// ----------------------------------------------------------------------
// Barra de herramientas
// ----------------------------------------------------------------------

fn toolbar(ui: &mut egui::Ui, app: &mut CupApp) {
    crate::editor::window_controls(ui);
    ui.add_space(4.0);
    let (sr, _) = ui.allocate_exact_size(Vec2::new(10.0, 14.0), Sense::hover());
    ui.painter().vline(sr.center().x, sr.y_range(), Stroke::new(1.0, Color32::from_rgb(0x33, 0x33, 0x33)));
    ui.spacing_mut().item_spacing.x = 2.0;
    if icons::icon_button(ui, "settings", 16.0, theme::MUTED, "Ajustes").clicked() {
        app.modal = Some(Modal::Settings);
    }
    if icons::icon_button(ui, "bell", 16.0, theme::MUTED, "Novedades").clicked() {
        app.modal = Some(Modal::Notifications);
    }
    if icons::icon_button(ui, "folderOpen", 16.0, theme::MUTED, "Abrir proyecto de CapCut").clicked() {
        app.open_capcut();
    }
    if icons::icon_button(ui, "link", 16.0, theme::MUTED, "Puente IA").clicked() {
        app.modal = Some(Modal::Bridge);
    }
    if icons::icon_button(ui, "info", 16.0, theme::MUTED, "Ayuda y atajos").clicked() {
        app.modal = Some(Modal::Shortcuts);
    }
}

// ----------------------------------------------------------------------
// Hero y tarjetas de IA
// ----------------------------------------------------------------------

fn gradient_h(p: &egui::Painter, rect: Rect, radius: u8, a: Color32, b: Color32) {
    // Degradado horizontal con esquinas redondeadas: malla en franjas
    // recortada por un rectángulo redondeado (se pinta por columnas).
    let r = (radius as f32).min(rect.height() / 2.0).min(rect.width() / 2.0);
    // Muestras densas dentro del radio de cada esquina y unas pocas en el centro.
    let corner = 10;
    let mut xs: Vec<f32> = Vec::new();
    for i in 0..=corner {
        xs.push(rect.left() + r * (i as f32 / corner as f32));
    }
    for i in 1..8 {
        xs.push(rect.left() + r + (rect.width() - 2.0 * r) * (i as f32 / 8.0));
    }
    for i in 0..=corner {
        xs.push(rect.right() - r + r * (i as f32 / corner as f32));
    }
    let mut mesh = egui::Mesh::default();
    for (i, &x) in xs.iter().enumerate() {
        let t = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        // Inset vertical en las esquinas redondeadas.
        let dx = (x - rect.left()).min(rect.right() - x);
        let inset = if dx < r { r - (r * r - (r - dx) * (r - dx)).max(0.0).sqrt() } else { 0.0 };
        let c = paint::lerp(a, b, t);
        mesh.colored_vertex(Pos2::new(x, rect.top() + inset), c);
        mesh.colored_vertex(Pos2::new(x, rect.bottom() - inset), c);
        if i > 0 {
            let k = (i as u32) * 2;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

fn hero(ui: &mut egui::Ui, app: &mut CupApp) {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 128.0), Sense::click());
    let hov = resp.hovered();
    gradient_h(
        ui.painter(),
        r,
        12,
        if hov { Color32::from_rgb(0x8a, 0xee, 0xf7) } else { colors::HERO_CYAN_1 },
        if hov { Color32::from_rgb(0x26, 0xbe, 0xdb) } else { colors::HERO_CYAN_2 },
    );
    let font = egui::FontId::proportional(22.0);
    let g = ui.painter().layout_no_wrap("Crear proyecto".into(), font, Color32::from_rgb(0x0b, 0x24, 0x27));
    let total = 34.0 + 12.0 + g.size().x;
    let start = r.center().x - total / 2.0;
    let plus = Rect::from_min_size(Pos2::new(start, r.center().y - 17.0), Vec2::splat(34.0));
    ui.painter().rect_filled(plus, 8.0, Color32::from_rgb(0x0c, 0x24, 0x27));
    icons::paint(ui.painter(), "plus", plus.center(), 18.0, Color32::WHITE);
    ui.painter().galley(Pos2::new(plus.right() + 12.0, r.center().y - g.size().y / 2.0), g, Color32::BLACK);
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        app.modal = Some(new_project_modal());
    }
}

fn ai_card(ui: &mut egui::Ui, w: f32, icon: &str, title: &str, sub: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 140.0), Sense::click());
    let p = ui.painter();
    p.rect(r, 10.0, if resp.hovered() { colors::CARD_HOVER } else { colors::CARD_BG }, Stroke::new(1.0, colors::CARD_BORDER), StrokeKind::Inside);
    let ir = Rect::from_min_size(r.min + Vec2::new(16.0, 16.0), Vec2::splat(34.0));
    p.rect(ir, 8.0, Color32::from_rgb(0x12, 0x20, 0x23), Stroke::new(1.0, Color32::from_rgb(0x33, 0x43, 0x45)), StrokeKind::Inside);
    icons::paint(p, icon, ir.center(), 17.0, Color32::WHITE);
    p.text(r.min + Vec2::new(16.0, 70.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(14.0), theme::TEXT);
    p.text(r.min + Vec2::new(16.0, 93.0), egui::Align2::LEFT_TOP, sub, egui::FontId::proportional(11.5), theme::MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn submit_editpilot(app: &mut CupApp) {
    let prompt = app.editpilot.trim().to_string();
    if prompt.is_empty() {
        app.editpilot_focus = true;
        return;
    }
    let name: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(42).collect();
    app.editpilot.clear();
    app.new_project_named(&name);
    app.panel_tab = PanelTab::Text;
    let preset = json!({ "size": 84, "weight": 800, "color": "#ffffff" });
    app.mutate_project(|p| p.add_text_clip(&prompt, Some(0.4), 5.0, Some(&preset)));
    app.save_current();
    app.toast("Borrador creado con EditPilot");
}

fn ai_grid(ui: &mut egui::Ui, app: &mut CupApp) {
    let gap = 14.0;
    let total = ui.available_width();
    let w1 = (total - gap * 3.0) * 0.36;
    let wo = (total - gap * 3.0 - w1) / 3.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        // EditPilot
        let (r, _) = ui.allocate_exact_size(Vec2::new(w1, 140.0), Sense::hover());
        gradient_h(ui.painter(), r, 10, colors::EDITPILOT_1, colors::EDITPILOT_2);
        ui.painter().rect_stroke(r, 10.0, Stroke::new(1.0, Color32::from_rgb(0x0a, 0x4b, 0x56)), StrokeKind::Inside);
        icons::paint(ui.painter(), "sparkles", r.min + Vec2::new(23.0, 23.0), 14.0, Color32::from_rgb(0x49, 0xe1, 0xea));
        ui.painter().text(r.min + Vec2::new(36.0, 23.0), egui::Align2::LEFT_CENTER, "EditPilot", egui::FontId::proportional(12.0), Color32::from_rgb(0x49, 0xe1, 0xea));
        ui.painter().text(r.min + Vec2::new(16.0, 40.0), egui::Align2::LEFT_TOP, "¿Qué puedo ayudarte a editar hoy?", egui::FontId::proportional(16.0), Color32::WHITE);
        let input = Rect::from_min_size(r.min + Vec2::new(16.0, 88.0), Vec2::new(w1 - 32.0, 36.0));
        ui.painter().rect(input, 8.0, Color32::from_rgb(0x15, 0x5e, 0x6a), Stroke::new(1.0, Color32::from_rgb(0x1d, 0x6c, 0x77)), StrokeKind::Inside);
        let field = Rect::from_min_max(input.min + Vec2::new(10.0, 4.0), input.max - Vec2::new(40.0, 4.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let te = child.add(
            egui::TextEdit::singleline(&mut app.editpilot)
                .id(egui::Id::new("editpilot-input"))
                .frame(egui::Frame::NONE)
                .char_limit(180)
                .hint_text(RichText::new(crate::i18n::tr("Describe lo que quieres editar")).color(Color32::from_rgb(0xa4, 0xc8, 0xcb)))
                .text_color(Color32::WHITE)
                .desired_width(field.width()),
        );
        if app.editpilot_focus {
            te.request_focus();
            app.editpilot_focus = false;
        }
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let send = Rect::from_center_size(Pos2::new(input.right() - 20.0, input.center().y), Vec2::splat(26.0));
        let sresp = ui.interact(send, ui.scope_id().with("editpilot-send"), Sense::click());
        ui.painter().circle_filled(send.center(), 13.0, Color32::from_white_alpha(if sresp.hovered() { 70 } else { 45 }));
        icons::paint(ui.painter(), "chevronRight", send.center(), 14.0, Color32::WHITE);
        if sresp.on_hover_text(crate::i18n::tr("Crear borrador")).clicked() || enter {
            submit_editpilot(app);
        }

        if ai_card(ui, wo, "image", "Imagen de IA", "Convierte ideas en imágenes") {
            app.unavailable("Funciones de IA", AI_MSG);
        }
        if ai_card(ui, wo, "fullscreen", "Ampliar video", "Amplía fácilmente cualquier clip") {
            app.unavailable("Funciones de IA", AI_MSG);
        }
        if ai_card(ui, wo, "scissors", "Editar con IA", "Edita clips describiéndolos") {
            app.editpilot_focus = true;
        }
    });
}

// ----------------------------------------------------------------------
// Proyectos
// ----------------------------------------------------------------------

fn pill(ui: &mut egui::Ui, icon: &str, label: &str, tip: &str) -> bool {
    let font = egui::FontId::proportional(12.0);
    let tw = ui.painter().layout_no_wrap(label.to_string(), font.clone(), theme::TEXT).size().x;
    let (r, resp) = ui.allocate_exact_size(Vec2::new(tw + 42.0, 30.0), Sense::click());
    ui.painter().rect(r, 15.0, if resp.hovered() { colors::CARD_HOVER } else { colors::CARD_BG }, Stroke::new(1.0, colors::CARD_BORDER), StrokeKind::Inside);
    icons::paint(ui.painter(), icon, r.left_center() + Vec2::new(18.0, 0.0), 14.0, theme::TEXT);
    ui.painter().text(r.left_center() + Vec2::new(30.0, 0.0), egui::Align2::LEFT_CENTER, label, font, theme::TEXT);
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn projects_head(ui: &mut egui::Ui, app: &mut CupApp) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(RichText::new(crate::i18n::tr("Proyectos")).size(18.0).strong().color(theme::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if icons::icon_button(ui, "filter", 15.0, theme::MUTED, "Ordenar proyectos").clicked() {
                app.home_sort = match app.home_sort {
                    HomeSort::Recent => HomeSort::Name,
                    HomeSort::Name => HomeSort::Oldest,
                    HomeSort::Oldest => HomeSort::Recent,
                };
                let label = match app.home_sort {
                    HomeSort::Name => "Nombre",
                    HomeSort::Oldest => "Más antiguos",
                    HomeSort::Recent => "Más recientes",
                };
                app.toast(format!("Proyectos ordenados por: {label}"));
            }
            if icons::icon_button(ui, "trash", 15.0, theme::MUTED, "Papelera").clicked() {
                app.unavailable("Papelera", "La papelera está vacía. Los proyectos que elimines se borran definitivamente de este equipo.");
            }
            if icons::icon_button(ui, "grid", 15.0, theme::MUTED, "Ver todos los proyectos").clicked() {
                ui.scroll_to_cursor(Some(egui::Align::TOP));
            }
            if icons::icon_button_sized(ui, "search", 15.0, Vec2::splat(28.0), theme::MUTED, "Buscar", app.home_search_open).clicked() {
                app.home_search_open = !app.home_search_open;
                if !app.home_search_open {
                    app.home_search.clear();
                }
            }
            if pill(ui, "download", "Abrir archivo", "Abrir un proyecto guardado como archivo") {
                app.open_project_file();
            }
            if pill(ui, "folderOpen", "CapCut", "Importar un proyecto de CapCut") {
                app.open_capcut();
            }
            if app.home_search_open {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut app.home_search)
                        .id(egui::Id::new("home-search"))
                        .hint_text(crate::i18n::tr("Buscar proyectos"))
                        .desired_width(200.0),
                );
                if resp.gained_focus() || app.home_search.is_empty() && !resp.has_focus() && !resp.lost_focus() {
                    // Enfoca la búsqueda nada más abrirla.
                    if ui.memory(|m| m.focused().is_none()) {
                        resp.request_focus();
                    }
                }
            }
        });
    })
    .response
}

fn projects_grid(ui: &mut egui::Ui, app: &mut CupApp) {
    let q = app.home_search.trim().to_lowercase();
    let mut list: Vec<_> = app.list_projects().into_iter().filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q)).collect();
    match app.home_sort {
        HomeSort::Recent => list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        HomeSort::Oldest => list.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
        HomeSort::Name => list.sort_by_key(|p| p.name.to_lowercase()),
    }
    if list.is_empty() {
        ui.add_space(10.0);
        egui::Frame::NONE
            .fill(colors::CARD_BG)
            .stroke(Stroke::new(1.0, colors::CARD_BORDER))
            .corner_radius(CornerRadius::from(10))
            .inner_margin(egui::Margin::symmetric(24, 26))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(crate::i18n::tr("Aún no hay proyectos")).size(14.0).strong().color(theme::TEXT));
                    ui.label(RichText::new(crate::i18n::tr("Crea tu primer video o importa un archivo para empezar.")).size(12.0).color(theme::MUTED));
                    ui.add_space(10.0);
                    if ui
                        .add(egui::Button::new(RichText::new(crate::i18n::tr("+  Crear proyecto")).color(theme::ACCENT_INK).strong()).fill(theme::ACCENT).corner_radius(CornerRadius::from(6)).min_size(Vec2::new(150.0, 32.0)))
                        .clicked()
                    {
                        app.modal = Some(new_project_modal());
                    }
                });
            });
        return;
    }
    let gap = 16.0;
    let avail = ui.available_width();
    let cols = ((avail + gap) / (220.0 + gap)).floor().max(1.0) as usize;
    let w = (avail - gap * (cols as f32 - 1.0)) / cols as f32;
    let mut action: Option<(u8, String, String)> = None;
    for (row, chunk) in list.chunks(cols).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (i, p) in chunk.iter().enumerate() {
                if let Some(a) = project_card(ui, p, row * cols + i, w) {
                    action = Some((a, p.id.clone(), p.name.clone()));
                }
            }
        });
        ui.add_space(gap);
    }
    match action {
        Some((0, id, _)) => app.open_project(&id),
        Some((1, id, name)) => app.modal = Some(Modal::Rename { id, name }),
        Some((2, id, _)) => app.save_project_file(&id),
        Some((3, id, name)) => app.modal = Some(Modal::ConfirmDelete { id, name }),
        _ => {}
    }
}

const THUMB_GRADIENTS: [(Color32, Color32); 4] = [
    (Color32::from_rgb(0x1a, 0x1a, 0x1a), Color32::from_rgb(0x11, 0x11, 0x11)),
    (Color32::from_rgb(0x1b, 0x1d, 0x22), Color32::from_rgb(0x11, 0x12, 0x14)),
    (Color32::from_rgb(0x1f, 0x1b, 0x18), Color32::from_rgb(0x12, 0x11, 0x10)),
    (Color32::from_rgb(0x17, 0x1d, 0x24), Color32::from_rgb(0x0f, 0x12, 0x16)),
];

/// Devuelve 0 = abrir, 1 = renombrar, 2 = guardar como archivo, 3 = eliminar.
fn project_card(ui: &mut egui::Ui, p: &cupcraft_core::Project, index: usize, w: f32) -> Option<u8> {
    let th = w * 9.0 / 16.0;
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, th + 54.0), Sense::click());
    let thumb = Rect::from_min_size(r.min, Vec2::new(w, th));
    let (a, b) = THUMB_GRADIENTS[index % THUMB_GRADIENTS.len()];
    paint::rounded_gradient(ui.painter(), thumb, 8, a, b);
    if resp.hovered() {
        ui.painter().rect_stroke(thumb, 8.0, Stroke::new(1.5, theme::ACCENT), StrokeKind::Inside);
    }
    icons::paint(ui.painter(), "film", thumb.center(), 26.0, Color32::from_rgb(0x3a, 0x3a, 0x3a));
    let dur = p.duration();
    if dur > 0.0 {
        let t = format_time(dur);
        let br = Rect::from_min_size(thumb.right_bottom() - Vec2::new(52.0, 24.0), Vec2::new(46.0, 18.0));
        ui.painter().rect_filled(br, 4.0, Color32::from_black_alpha(180));
        ui.painter().text(br.center(), egui::Align2::CENTER_CENTER, &t, egui::FontId::proportional(10.0), Color32::WHITE);
    }
    let font = egui::FontId::proportional(13.0);
    let name = paint::ellipsize(ui.painter(), &p.name, &font, w - 4.0);
    ui.painter().text(Pos2::new(r.left() + 2.0, thumb.bottom() + 10.0), egui::Align2::LEFT_TOP, name, font, theme::TEXT);
    let meta = format!("{}    {}    {}", p.resolution, format_time(dur), time_ago(p.updated_at));
    ui.painter().text(Pos2::new(r.left() + 2.0, thumb.bottom() + 31.0), egui::Align2::LEFT_TOP, meta, egui::FontId::proportional(11.0), theme::MUTED);

    // Acciones al pasar el ratón (esquina superior derecha de la miniatura).
    let mut out = None;
    let hover_card = ui.rect_contains_pointer(r);
    if hover_card {
        let mut x = thumb.right() - 6.0;
        for (k, icon, tip) in [(3u8, "trash", "Eliminar"), (2, "download", "Guardar proyecto como archivo"), (1, "pencil", "Renombrar")] {
            let br = Rect::from_min_size(Pos2::new(x - 26.0, thumb.top() + 6.0), Vec2::splat(26.0));
            x -= 30.0;
            let bresp = ui.interact(br, ui.scope_id().with(("card-action", &p.id, k)), Sense::click());
            ui.painter().rect_filled(br, 5.0, if bresp.hovered() { Color32::from_black_alpha(230) } else { Color32::from_black_alpha(170) });
            icons::paint(ui.painter(), icon, br.center(), 13.0, Color32::WHITE);
            if bresp.on_hover_text(tip).clicked() {
                out = Some(k);
            }
        }
    }
    if out.is_none() && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        out = Some(0);
    }
    out
}
