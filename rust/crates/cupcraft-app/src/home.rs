//! Pantalla de inicio nativa de cupcraft: diseño 1:1 estilo CapCut.
//!
//! Barra lateral de navegación, banner hero cian "+ Crear proyecto",
//! tarjetas de creación con IA (EditPilot), catálogo y cuadrícula de proyectos.

use crate::app::CupApp;
use crate::format::time_ago;
use crate::theme;
use eframe::egui;

/// Paleta específica de la pantalla de inicio estilo CapCut
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
}

pub fn show(ui: &mut egui::Ui, app: &mut CupApp) {
    let ctx = ui.ctx().clone();

    // ──────────────── Barra lateral izquierda (210 px) ──────────────────────
    egui::Panel::left("home_sidebar")
        .exact_size(210.0)
        .frame(egui::Frame::NONE.fill(colors::SIDEBAR_BG))
        .show(ui, |ui| {
            ui.add_space(14.0);
            // Logotipo + Marca
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new("✦").color(theme::ACCENT).size(18.0).strong());
                ui.label(egui::RichText::new("cupcraft").color(theme::TEXT).size(15.0).strong());
            });
            ui.add_space(14.0);

            // Tarjeta de usuario / Iniciar sesión
            egui::Frame::NONE
                .fill(colors::SIDEBAR_USER)
                .stroke(egui::Stroke::new(1.0, colors::SIDEBAR_BORDER))
                .corner_radius(egui::CornerRadius::from(8))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Avatar placeholder
                        let (avatar_rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                        ui.painter().rect_filled(avatar_rect, egui::CornerRadius::from(6), egui::Color32::from_rgb(0x22, 0x2f, 0x32));
                        ui.painter().text(avatar_rect.center(), egui::Align2::CENTER_CENTER, "👤", egui::FontId::proportional(13.0), theme::MUTED);

                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Iniciar sesión").color(theme::TEXT).size(11.5).strong());
                            ui.label(egui::RichText::new("Local · sin cuenta").color(theme::MUTED).size(9.0));
                        });
                    });
                    ui.add_space(6.0);
                    // Botón gradiente Unirse a Pro
                    let (btn_rect, btn_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
                    paint_horizontal_gradient(ui.painter(), btn_rect, 4.0, egui::Color32::from_rgb(0xfb, 0xd3, 0xf2), egui::Color32::from_rgb(0x9d, 0xf6, 0xdd));
                    ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "Unirse a Pro", egui::FontId::proportional(11.0), egui::Color32::from_rgb(0x14, 0x20, 0x1f));
                    if btn_resp.clicked() {
                        app.toast("cupcraft Pro: Todas las funciones están desbloqueadas en esta versión");
                    }
                });

            ui.add_space(10.0);

            // Elementos de navegación
            nav_button(ui, "🏠 Inicio", true, || {});
            nav_button(ui, "📑 Plantillas", false, || {});

            ui.add_space(6.0);
            ui.label(egui::RichText::new("Crear con IA").color(theme::MUTED2).size(10.0));
            ui.add_space(2.0);
            nav_button(ui, "🎬 Estudio de video", false, || {});
            nav_button(ui, "🎨 Estudio de diseño", false, || {});

            ui.add_space(6.0);
            ui.label(egui::RichText::new("Administración").color(theme::MUTED2).size(10.0));
            ui.add_space(2.0);
            nav_button(ui, "📁 Proyectos de CapCut", false, || {});
            nav_button_with_badge(ui, "🔗 Puente IA", "ACTIVO", egui::Color32::from_rgb(0x25, 0xd4, 0x90));
            nav_button(ui, "📦 Biblioteca", false, || {});

            // Pie de barra lateral
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(12.0);

                // Tarjeta promo ¿Quieres alargarlo?
                let (promo_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 70.0), egui::Sense::click());
                paint_horizontal_gradient(ui.painter(), promo_rect, 6.0, colors::PROMO_BLUE_1, colors::PROMO_BLUE_2);
                let p_p = ui.painter().with_clip_rect(promo_rect);
                p_p.text(egui::pos2(promo_rect.min.x + 10.0, promo_rect.min.y + 16.0), egui::Align2::LEFT_TOP, "¿Quieres alargarlo?", egui::FontId::proportional(11.5), egui::Color32::from_rgb(0x5a, 0xdc, 0xf2));
                p_p.text(egui::pos2(promo_rect.min.x + 10.0, promo_rect.min.y + 36.0), egui::Align2::LEFT_TOP, "Amplía tu video con IA", egui::FontId::proportional(9.5), egui::Color32::from_rgb(0xc8, 0xe8, 0xf8));

                ui.add_space(8.0);

                // Invitar a amigos
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_premultiplied(8, 23, 25, 128))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x1d, 0x3a, 0x3d)))
                    .corner_radius(egui::CornerRadius::from(6))
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("👥 Invitar a amigos").color(theme::TEXT).size(10.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let badge_text = egui::RichText::new("NUEVO").color(egui::Color32::from_rgb(0x22, 0x1b, 0x00)).size(8.0).strong();
                                ui.label(badge_text);
                            });
                        });
                    });
            });
        });

    // ──────────────── Contenido Principal ────────────────────────────────────
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(colors::MAIN_BG))
        .show(ui, |ui| {
            // Barra superior de utilidades (derecha)
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(16.0);
                    if ui.small_button("⚙ Ajustes").clicked() {
                        app.toast("cupcraft 0.4.5 (Nativo Rust)");
                    }
                    if ui.small_button("🔗 Puente IA").clicked() {
                        app.toast("Puente IA en 127.0.0.1:3010 con token activo");
                    }
                    if ui.small_button("📁 CapCut").clicked() {
                        app.toast("Importador de CapCut listo");
                    }
                    if ui.small_button("ℹ Ayuda").clicked() {
                        app.toast("cupcraft: Editor de vídeo nativo en Rust + wgpu");
                    }
                });
            });

            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);

                // ═════════════════════════════════════════════════════════════
                // BANNER HERO GIGANTE CIAN: "+ Crear proyecto" (Estilo CapCut)
                // ═════════════════════════════════════════════════════════════
                let avail_w = (ui.available_width() - 32.0).max(400.0);
                let (hero_rect, hero_resp) = ui.allocate_exact_size(egui::vec2(avail_w, 136.0), egui::Sense::click());

                // Dibujar gradiente cian vibrante
                paint_horizontal_gradient(
                    ui.painter(),
                    hero_rect,
                    10.0,
                    if hero_resp.hovered() { egui::Color32::from_rgb(0x8a, 0xee, 0xf7) } else { colors::HERO_CYAN_1 },
                    if hero_resp.hovered() { egui::Color32::from_rgb(0x26, 0xbe, 0xdb) } else { colors::HERO_CYAN_2 },
                );

                // Botón oscuro interior con el signo '+'
                let plus_rect = egui::Rect::from_center_size(
                    egui::pos2(hero_rect.center().x - 84.0, hero_rect.center().y),
                    egui::vec2(30.0, 30.0),
                );
                ui.painter().rect_filled(plus_rect, egui::CornerRadius::from(6), egui::Color32::from_rgb(0x0c, 0x24, 0x27));
                ui.painter().text(plus_rect.center(), egui::Align2::CENTER_CENTER, "＋", egui::FontId::proportional(17.0), egui::Color32::WHITE);

                // Texto "+ Crear proyecto"
                ui.painter().text(
                    egui::pos2(hero_rect.center().x - 56.0, hero_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "Crear proyecto",
                    egui::FontId::proportional(22.0),
                    egui::Color32::from_rgb(0x0b, 0x24, 0x27),
                );

                if hero_resp.clicked() {
                    app.new_project();
                }

                ui.add_space(20.0);

                // ═════════════════════════════════════════════════════════════
                // FILA DE 4 TARJETAS CON IA (home-grid)
                // ═════════════════════════════════════════════════════════════
                ui.horizontal(|ui| {
                    let total_w = hero_rect.width();
                    let card_gap = 14.0;
                    // Proporción: la primera es más ancha (1.6x), las otras 3 iguales
                    let c1_w = (total_w - (card_gap * 3.0)) * 0.36;
                    let c_other_w = (total_w - (card_gap * 3.0) - c1_w) / 3.0;

                    // ── Tarjeta 1: EditPilot ──────────────────────────────────
                    let (card1_rect, _) = ui.allocate_exact_size(egui::vec2(c1_w, 136.0), egui::Sense::hover());
                    paint_horizontal_gradient(ui.painter(), card1_rect, 8.0, colors::EDITPILOT_1, colors::EDITPILOT_2);
                    ui.painter().rect_stroke(card1_rect, egui::CornerRadius::from(8), egui::Stroke::new(1.0, egui::Color32::from_rgb(0x0a, 0x4b, 0x56)), egui::StrokeKind::Inside);

                    let p1 = ui.painter().with_clip_rect(card1_rect);
                    p1.text(egui::pos2(card1_rect.min.x + 14.0, card1_rect.min.y + 14.0), egui::Align2::LEFT_TOP, "✨ EditPilot", egui::FontId::proportional(11.0), egui::Color32::from_rgb(0x49, 0xe1, 0xea));
                    p1.text(egui::pos2(card1_rect.min.x + 14.0, card1_rect.min.y + 34.0), egui::Align2::LEFT_TOP, "¿Qué puedo ayudarte a editar hoy?", egui::FontId::proportional(15.0), egui::Color32::WHITE);

                    // Input simulado dentro de la tarjeta
                    let input_rect = egui::Rect::from_min_size(
                        egui::pos2(card1_rect.min.x + 14.0, card1_rect.min.y + 86.0),
                        egui::vec2(c1_w - 28.0, 32.0),
                    );
                    p1.rect_filled(input_rect, egui::CornerRadius::from(6), egui::Color32::from_rgb(0x15, 0x5e, 0x6a));
                    p1.rect_stroke(input_rect, egui::CornerRadius::from(6), egui::Stroke::new(1.0, egui::Color32::from_rgb(0x1d, 0x6c, 0x77)), egui::StrokeKind::Inside);
                    p1.text(egui::pos2(input_rect.min.x + 10.0, input_rect.center().y), egui::Align2::LEFT_CENTER, "Describe lo que quieres editar…", egui::FontId::proportional(11.0), egui::Color32::from_rgb(0xa4, 0xc8, 0xcb));
                    let send_rect = egui::Rect::from_center_size(egui::pos2(input_rect.max.x - 16.0, input_rect.center().y), egui::vec2(22.0, 22.0));
                    p1.rect_filled(send_rect, egui::CornerRadius::from(11), egui::Color32::from_rgba_premultiplied(255, 255, 255, 45));
                    p1.text(send_rect.center(), egui::Align2::CENTER_CENTER, "➔", egui::FontId::proportional(11.0), egui::Color32::WHITE);

                    ui.add_space(card_gap);

                    // ── Tarjeta 2: Imagen de IA ──────────────────────────────
                    ai_card(ui, c_other_w, "🖼", "Imagen de IA", "Convierte ideas en imágenes");

                    ui.add_space(card_gap);

                    // ── Tarjeta 3: Ampliar video ─────────────────────────────
                    ai_card(ui, c_other_w, "⛶", "Ampliar video", "Amplía cualquier clip");

                    ui.add_space(card_gap);

                    // ── Tarjeta 4: Editar con IA ─────────────────────────────
                    ai_card(ui, c_other_w, "✂", "Editar con IA", "Edita clips por texto");
                });

                ui.add_space(32.0);

                // ═════════════════════════════════════════════════════════════
                // SECCIÓN: PROYECTOS (Encabezado + Toolbar)
                // ═════════════════════════════════════════════════════════════
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Proyectos").color(theme::TEXT).size(16.0).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if ui.button("📁 CapCut").clicked() {
                            app.toast("Explorando borradores de CapCut…");
                        }
                        if ui.button("📥 Abrir archivo").clicked() {
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

                ui.add_space(14.0);

                // ═════════════════════════════════════════════════════════════
                // CUADRÍCULA DE PROYECTOS REALES
                // ═════════════════════════════════════════════════════════════
                let projects = app.store.as_ref().map(|s| s.list_projects().unwrap_or_default()).unwrap_or_default();

                ui.horizontal_wrapped(|ui| {
                    for p in &projects {
                        project_card(ui, app, p);
                    }
                    if projects.is_empty() {
                        egui::Frame::NONE
                            .fill(colors::CARD_BG)
                            .stroke(egui::Stroke::new(1.0, colors::CARD_BORDER))
                            .corner_radius(egui::CornerRadius::from(8))
                            .inner_margin(egui::Margin::symmetric(24, 20))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new("Aún no hay proyectos").color(theme::TEXT).strong());
                                    ui.label(egui::RichText::new("Haz clic en \"Crear proyecto\" arriba para empezar.").color(theme::MUTED).size(11.0));
                                });
                            });
                    }
                });

                ui.add_space(40.0);
            });
        });

    crate::app::show_toast(&ctx, app);
}

// ────────────────────── Componentes auxiliares ───────────────────────────────

fn nav_button(ui: &mut egui::Ui, text: &str, active: bool, _on_click: impl FnOnce()) {
    let (bg, fg) = if active {
        (colors::SIDEBAR_ACTIVE, egui::Color32::WHITE)
    } else {
        (egui::Color32::TRANSPARENT, egui::Color32::from_rgb(0xc8, 0xd5, 0xd6))
    };

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), egui::Sense::click());
    if active || resp.hovered() {
        ui.painter().rect_filled(rect, egui::CornerRadius::from(6), bg);
    }
    ui.painter().text(
        egui::pos2(rect.min.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(12.0),
        if resp.hovered() { egui::Color32::WHITE } else { fg },
    );
}

fn nav_button_with_badge(ui: &mut egui::Ui, text: &str, badge: &str, badge_color: egui::Color32) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, egui::CornerRadius::from(6), egui::Color32::from_rgba_premultiplied(255, 255, 255, 12));
    }
    ui.painter().text(
        egui::pos2(rect.min.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(0xc8, 0xd5, 0xd6),
    );

    // Glowing dot / badge on the right
    let badge_rect = egui::Rect::from_center_size(egui::pos2(rect.max.x - 28.0, rect.center().y), egui::vec2(36.0, 16.0));
    ui.painter().rect_filled(badge_rect, egui::CornerRadius::from(8), egui::Color32::from_rgba_premultiplied(37, 212, 144, 40));
    ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, badge, egui::FontId::proportional(8.5), badge_color);
}

fn ai_card(ui: &mut egui::Ui, width: f32, icon: &str, title: &str, subtitle: &str) {
    let (card_rect, resp) = ui.allocate_exact_size(egui::vec2(width, 136.0), egui::Sense::click());
    let bg = if resp.hovered() { colors::CARD_HOVER } else { colors::CARD_BG };
    ui.painter().rect_filled(card_rect, egui::CornerRadius::from(8), bg);
    ui.painter().rect_stroke(card_rect, egui::CornerRadius::from(8), egui::Stroke::new(1.0, colors::CARD_BORDER), egui::StrokeKind::Inside);

    let p = ui.painter().with_clip_rect(card_rect);

    // Icon container
    let icon_rect = egui::Rect::from_min_size(egui::pos2(card_rect.min.x + 14.0, card_rect.min.y + 14.0), egui::vec2(30.0, 30.0));
    p.rect_filled(icon_rect, egui::CornerRadius::from(6), egui::Color32::from_rgb(0x12, 0x20, 0x23));
    p.rect_stroke(icon_rect, egui::CornerRadius::from(6), egui::Stroke::new(1.0, egui::Color32::from_rgb(0x33, 0x43, 0x45)), egui::StrokeKind::Inside);
    p.text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(15.0), egui::Color32::WHITE);

    // Title
    p.text(egui::pos2(card_rect.min.x + 14.0, card_rect.min.y + 60.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(13.0), theme::TEXT);

    // Subtitle
    p.text(egui::pos2(card_rect.min.x + 14.0, card_rect.min.y + 82.0), egui::Align2::LEFT_TOP, subtitle, egui::FontId::proportional(10.5), theme::MUTED);
}

fn project_card(ui: &mut egui::Ui, app: &mut CupApp, p: &cupcraft_core::Project) {
    let id = p.id.clone();
    let renaming = app.rename_id.as_deref() == Some(id.as_str());

    egui::Frame::NONE
        .fill(colors::CARD_BG)
        .stroke(egui::Stroke::new(1.0, colors::CARD_BORDER))
        .corner_radius(egui::CornerRadius::from(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(220.0);
            ui.set_max_width(220.0);

            // Thumbnail estilizada de vídeo (16:9)
            let (thumb_rect, thumb_resp) = ui.allocate_exact_size(egui::vec2(200.0, 114.0), egui::Sense::click());
            paint_horizontal_gradient(
                ui.painter(),
                thumb_rect,
                6.0,
                egui::Color32::from_rgb(0x17, 0x26, 0x28),
                egui::Color32::from_rgb(0x0e, 0x18, 0x1a),
            );

            // Icono de play / película centrado
            ui.painter().text(thumb_rect.center(), egui::Align2::CENTER_CENTER, "▶", egui::FontId::proportional(22.0), egui::Color32::from_rgba_premultiplied(255, 255, 255, 50));

            // Badge de duración en la esquina inferior derecha
            let dur_str = format_duration(p.timeline_length());
            let badge_w = 42.0;
            let badge_rect = egui::Rect::from_min_size(
                egui::pos2(thumb_rect.max.x - badge_w - 6.0, thumb_rect.max.y - 20.0),
                egui::vec2(badge_w, 16.0),
            );
            ui.painter().rect_filled(badge_rect, egui::CornerRadius::from(3), egui::Color32::from_rgba_premultiplied(0, 0, 0, 180));
            ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, &dur_str, egui::FontId::proportional(9.0), egui::Color32::WHITE);

            if thumb_resp.clicked() {
                app.open_project(&id);
            }

            ui.add_space(8.0);

            // Título del proyecto
            if renaming {
                let resp = ui.text_edit_singleline(&mut app.rename_buf);
                if resp.lost_focus() {
                    let name = app.rename_buf.clone();
                    app.rename_project(&id, &name);
                    app.rename_id = None;
                    app.toast("Renombrado");
                }
            } else {
                ui.label(egui::RichText::new(&p.name).color(theme::TEXT).size(13.0).strong());
            }

            // Metadatos: Ratio, FPS, fecha
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&p.ratio).color(theme::MUTED2).size(10.0));
                ui.label(egui::RichText::new("·").color(theme::MUTED2).size(10.0));
                ui.label(egui::RichText::new(time_ago(p.updated_at)).color(theme::MUTED).size(10.5));
            });

            ui.add_space(6.0);

            // Botones de acción
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("Abrir").color(theme::ACCENT).size(11.0)).clicked() {
                    app.open_project(&id);
                }
                if ui.small_button("✏").clicked() {
                    app.rename_id = Some(id.clone());
                    app.rename_buf = p.name.clone();
                }
                let deleting = app.pending_delete.as_ref().map(|(x, _)| x == &id).unwrap_or(false);
                let del_label = if deleting { "Confirmar" } else { "🗑" };
                if ui.small_button(del_label).clicked() {
                    if deleting {
                        app.delete_project(&id);
                        app.pending_delete = None;
                    } else {
                        app.pending_delete = Some((id.clone(), now_secs() + 4.0));
                    }
                }
            });
        });
}

fn format_duration(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let m = s / 60;
    let rem = s % 60;
    format!("{:02}:{:02}", m, rem)
}

fn now_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Pinta un rectángulo con gradiente horizontal de color_a a color_b usando egui::Mesh.
fn paint_horizontal_gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: f32,
    color_a: egui::Color32,
    color_b: egui::Color32,
) {
    let p = painter.with_clip_rect(rect);
    let mut mesh = egui::Mesh::default();

    // 4 vértices
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_top(), uv: egui::epaint::WHITE_UV, color: color_a });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_bottom(), uv: egui::epaint::WHITE_UV, color: color_a });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_bottom(), uv: egui::epaint::WHITE_UV, color: color_b });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_top(), uv: egui::epaint::WHITE_UV, color: color_b });

    // 2 triángulos
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);

    p.add(mesh);

    if corner_radius > 0.0 {
        // Redondear esquinas con stroke sutil
        p.rect_stroke(rect, egui::CornerRadius::from(corner_radius as u8), egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 60)), egui::StrokeKind::Inside);
    }
}
