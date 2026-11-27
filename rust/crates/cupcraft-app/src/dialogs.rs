//! Diálogos modales (equivalentes nativos de `openModal` / `confirmDialog`
//! de la versión web): nuevo proyecto, ajustes, Pro, CapCut, exportación,
//! registro, atajos, renombrar, eliminar, plantillas, etc.

use crate::app::{CupApp, LogKind, Modal};
use crate::format::{format_bytes, format_time, time_ago};
use crate::icons;
use crate::panels::{self, chip, hint, primary_button, secondary_button};
use crate::paint;
use crate::services::export::ExportStatus;
use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use serde_json::{json, Value};

// ----------------------------------------------------------------------
// Datos del diálogo "Nuevo proyecto" (DESIGN_CATEGORIES de la web)
// ----------------------------------------------------------------------

pub struct Design {
    pub id: &'static str,
    pub label: &'static str,
    pub ratio: &'static str,
    pub w: u32,
    pub h: u32,
}

pub const DESIGN_CATS: &[(&str, &[Design])] = &[
    (
        "YouTube",
        &[
            Design { id: "yt-video", label: "Video de YouTube", ratio: "16:9", w: 1920, h: 1080 },
            Design { id: "yt-short", label: "YouTube Shorts", ratio: "9:16", w: 1080, h: 1920 },
        ],
    ),
    (
        "Instagram",
        &[
            Design { id: "ig-reel", label: "Reel de Instagram", ratio: "9:16", w: 1080, h: 1920 },
            Design { id: "ig-story", label: "Historia de Instagram", ratio: "9:16", w: 1080, h: 1920 },
            Design { id: "ig-post", label: "Publicación cuadrada", ratio: "1:1", w: 1080, h: 1080 },
            Design { id: "ig-feed", label: "Publicación vertical", ratio: "4:5", w: 1080, h: 1350 },
        ],
    ),
    (
        "Facebook y TikTok",
        &[
            Design { id: "fb-reel", label: "Reel de Facebook", ratio: "9:16", w: 720, h: 1280 },
            Design { id: "fb-story", label: "Historia de Facebook", ratio: "9:16", w: 720, h: 1280 },
            Design { id: "tiktok", label: "TikTok", ratio: "9:16", w: 720, h: 1280 },
            Design { id: "fb-video", label: "Video de Facebook", ratio: "16:9", w: 1280, h: 720 },
        ],
    ),
    (
        "Más",
        &[
            Design { id: "snapchat", label: "Snapchat", ratio: "9:16", w: 720, h: 1280 },
            Design { id: "x", label: "X (Twitter)", ratio: "16:9", w: 1280, h: 720 },
            Design { id: "linkedin", label: "LinkedIn", ratio: "16:9", w: 1280, h: 720 },
            Design { id: "horizontal", label: "Horizontal", ratio: "16:9", w: 1280, h: 720 },
            Design { id: "square", label: "Cuadrado", ratio: "1:1", w: 1080, h: 1080 },
            Design { id: "vertical", label: "Vertical", ratio: "9:16", w: 720, h: 1280 },
            Design { id: "classic", label: "Clásico", ratio: "4:3", w: 1440, h: 1080 },
        ],
    ),
];

/// (etiqueta, lado corto)
pub const QUALITIES: &[(&str, u32)] = &[("4K", 2160), ("1440p", 1440), ("1080p", 1080), ("720p", 720)];
pub const FPS_OPTS: &[u32] = &[24, 30, 60];

pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Espacio", "Reproducir / pausar"),
    ("Ctrl + Z", "Deshacer"),
    ("Ctrl + Shift + Z / Ctrl + Y", "Rehacer"),
    ("Ctrl + B", "Dividir en el cursor"),
    ("V", "Herramienta de selección"),
    ("B", "Herramienta cuchilla"),
    ("Q", "Recortar inicio hasta el cursor"),
    ("W", "Recortar final desde el cursor"),
    ("Supr / Retroceso", "Eliminar clip seleccionado"),
    ("Ctrl + D", "Duplicar clip"),
    ("M", "Añadir / quitar marcador"),
    ("N", "Activar / desactivar imán"),
    ("Shift + Z", "Ajustar zoom a la ventana"),
    ("Ctrl + S", "Guardar proyecto"),
    ("← / →", "Mover el cursor un fotograma"),
];

// ----------------------------------------------------------------------
// Infraestructura
// ----------------------------------------------------------------------

/// Marco común de los diálogos. Devuelve `false` si se debe cerrar.
fn dialog(ctx: &egui::Context, id: &str, title: &str, width: f32, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    let frame = egui::Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::LINE_SOFT))
        .corner_radius(CornerRadius::from(12))
        .inner_margin(egui::Margin::same(20))
        .shadow(egui::epaint::Shadow { offset: [0, 12], blur: 40, spread: 0, color: Color32::from_black_alpha(140) });
    let mut keep = true;
    let resp = egui::Modal::new(egui::Id::new(("modal", id)))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(150))
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(title)).size(17.0).strong().color(theme::TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::icon_button(ui, "x", 16.0, theme::MUTED, &crate::i18n::tr("Cerrar")).clicked() {
                        keep = false;
                    }
                });
            });
            ui.add_space(10.0);
            if !add(ui) {
                keep = false;
            }
        });
    if resp.should_close() {
        keep = false;
    }
    keep
}

fn note(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE
        .fill(theme::PANEL2)
        .corner_radius(CornerRadius::from(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                icons::icon(ui, "info", 15.0, theme::ACCENT);
                ui.add(egui::Label::new(RichText::new(crate::i18n::tr(text)).size(12.0).color(theme::MUTED)).wrap());
            });
        });
}

fn para(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(13.0).color(theme::TEXT)).wrap());
}

fn footer(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(14.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add);
}

fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE).strong().size(12.5))
            .fill(theme::DANGER)
            .corner_radius(CornerRadius::from(6))
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

fn kv(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(crate::i18n::tr(k)).color(theme::MUTED).size(12.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(crate::i18n::tr(v)).color(theme::TEXT).size(12.5));
        });
    });
}

fn open_in_explorer(path: &std::path::Path) {
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
}

// ----------------------------------------------------------------------
// Entrada
// ----------------------------------------------------------------------

pub fn show(ctx: &egui::Context, app: &mut CupApp) {
    let Some(mut modal) = app.modal.take() else { return };
    let keep = match &mut modal {
        Modal::Pro => dialog(ctx, "pro", "cupcraft Pro", 440.0, |ui| {
            para(ui, "cupcraft es un editor local y gratuito. No hay cuentas, anuncios ni límites de exportación.");
            ui.add_space(8.0);
            note(ui, "Todas las funciones de esta versión están disponibles sin suscripción. Tus archivos nunca salen de tu equipo.");
            let mut close = false;
            footer(ui, |ui| close = primary_button(ui, "Genial").clicked());
            !close
        }),
        Modal::Share => dialog(ctx, "share", "Compartir", 440.0, |ui| {
            para(ui, "cupcraft no sube nada a internet. Para compartir tu video, expórtalo como archivo y envíalo por el medio que prefieras.");
            ui.add_space(8.0);
            note(ui, "También puedes guardar el proyecto como archivo .cupcraft.json para que otra persona lo abra en su cupcraft.");
            let mut close = false;
            let mut export = false;
            let mut save = false;
            footer(ui, |ui| {
                export = primary_button(ui, "Exportar video").clicked();
                save = secondary_button(ui, "Guardar proyecto como archivo").clicked();
                close = secondary_button(ui, "Cerrar").clicked();
            });
            if export {
                app.modal = Some(Modal::Export);
            }
            if save {
                if let Some(id) = app.editor.project().map(|p| p.id.clone()) {
                    app.save_project_file(&id);
                }
            }
            !(close || export || save)
        }),
        Modal::About => dialog(ctx, "about", "Acerca de cupcraft", 440.0, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                icons::paint_logo(ui.painter(), r, Color32::WHITE);
                ui.vertical(|ui| {
                    ui.label(RichText::new("cupcraft").size(18.0).strong().color(theme::TEXT));
                    ui.label(RichText::new(format!("Versión {}", env!("CARGO_PKG_VERSION"))).color(theme::MUTED));
                });
            });
            ui.add_space(8.0);
            para(ui, "Editor de video de escritorio, nativo para Windows (Rust + egui). Funciona sin conexión, sin cuentas y sin privilegios de administrador.");
            ui.add_space(6.0);
            note(ui, "Los proyectos y medios se guardan en %LOCALAPPDATA%\\openCUP. La exportación usa FFmpeg si está disponible en el equipo.");
            let mut close = false;
            footer(ui, |ui| close = primary_button(ui, "Cerrar").clicked());
            !close
        }),
        Modal::Notifications => dialog(ctx, "notif", "Novedades", 420.0, |ui| {
            ui.vertical_centered(|ui| {
                icons::icon(ui, "bell", 30.0, theme::MUTED2);
                ui.label(RichText::new("Estás al día").size(15.0).strong().color(theme::TEXT));
                hint(ui, "No hay notificaciones nuevas. cupcraft funciona sin conexión.");
            });
            let mut close = false;
            footer(ui, |ui| close = primary_button(ui, "Entendido").clicked());
            !close
        }),
        Modal::WhatsNew { version, notes } => dialog(ctx, "whatsnew", &format!("cupcraft {version}"), 460.0, |ui| {
            ui.label(RichText::new("¡Actualización instalada! Novedades de esta versión:").size(13.0).color(theme::MUTED));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
                if notes.trim().is_empty() {
                    para(ui, "Mejoras generales y correcciones.");
                }
                for line in notes.lines() {
                    let t = line.trim();
                    if t.is_empty() {
                        continue;
                    }
                    if let Some(h) = t.strip_prefix('#') {
                        ui.add_space(4.0);
                        ui.label(RichText::new(h.trim_start_matches('#').trim()).size(14.0).strong().color(theme::TEXT));
                    } else if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new("•").color(theme::ACCENT));
                            para(ui, &item.replace("**", "").replace('`', ""));
                        });
                    } else {
                        para(ui, &t.replace("**", "").replace('`', ""));
                    }
                    ui.add_space(3.0);
                }
            });
            true
        }),
        Modal::Bridge => dialog(ctx, "bridge", "Puente IA", 480.0, |ui| {
            para(ui, "En la versión web, el Puente IA abría un puerto local para que asistentes externos controlaran el editor. La versión de escritorio nativa no abre ningún servidor ni puerto: toda la edición ocurre dentro de la ventana de cupcraft.");
            ui.add_space(8.0);
            note(ui, "Para trabajar con un asistente, guarda el proyecto como archivo (.cupcraft.json), edítalo con la herramienta que prefieras y vuelve a abrirlo con «Abrir proyecto desde archivo».");
            let mut close = false;
            let mut open = false;
            footer(ui, |ui| {
                close = primary_button(ui, "Entendido").clicked();
                open = secondary_button(ui, "Abrir proyecto desde archivo").clicked();
            });
            if open {
                app.open_project_file();
            }
            !(close || open)
        }),
        Modal::Unavailable { title, msg } => {
            let (t, m) = (title.clone(), msg.clone());
            dialog(ctx, "unavail", &t, 440.0, |ui| {
                note(ui, &m);
                let mut close = false;
                footer(ui, |ui| close = primary_button(ui, "Entendido").clicked());
                !close
            })
        }
        Modal::Help(tab) => {
            let tab = *tab;
            dialog(ctx, "help", &format!("Ayuda · {}", tab.title()), 440.0, |ui| {
                para(ui, tab.help());
                let mut close = false;
                let mut keys = false;
                footer(ui, |ui| {
                    close = primary_button(ui, "Entendido").clicked();
                    keys = secondary_button(ui, "Atajos de teclado").clicked();
                });
                if keys {
                    app.modal = Some(Modal::Shortcuts);
                }
                !(close || keys)
            })
        }
        Modal::Shortcuts => dialog(ctx, "keys", "Atajos de teclado", 460.0, |ui| {
            egui::Grid::new("keys-grid").num_columns(2).spacing([24.0, 8.0]).striped(false).show(ui, |ui| {
                for (k, d) in SHORTCUTS {
                    egui::Frame::NONE
                        .fill(theme::FIELD)
                        .corner_radius(CornerRadius::from(4))
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new(*k).family(egui::FontFamily::Monospace).size(12.0).color(theme::TEXT));
                        });
                    ui.label(RichText::new(*d).size(12.5).color(theme::MUTED));
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            hint(ui, "Consejo: mantén Alt al arrastrar un clip para desactivar temporalmente el imán.");
            let mut close = false;
            footer(ui, |ui| close = primary_button(ui, "Listo").clicked());
            !close
        }),
        Modal::Settings => settings(ctx, app),
        Modal::Log { filter, only_errors } => log_dialog(ctx, app, filter, only_errors),
        Modal::NewProject { cat, tpl, quality, fps, name } => new_project(ctx, app, cat, tpl, quality, fps, name),
        Modal::Rename { id, name } => {
            let id = id.clone();
            dialog(ctx, "rename", "Renombrar proyecto", 420.0, |ui| {
                ui.label(RichText::new("Nombre del proyecto").color(theme::MUTED).size(12.0));
                let r = ui.add(egui::TextEdit::singleline(name).char_limit(80).desired_width(f32::INFINITY));
                r.request_focus();
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut save = enter;
                let mut cancel = false;
                footer(ui, |ui| {
                    save |= primary_button(ui, "Guardar").clicked();
                    cancel = secondary_button(ui, "Cancelar").clicked();
                });
                if save {
                    app.rename_project(&id, name);
                    app.toast("Proyecto renombrado");
                }
                !(save || cancel)
            })
        }
        Modal::QuitSave { naming, name } => {
            let ctx2 = ctx.clone();
            let title = if *naming { "Nombre del proyecto" } else { "Guardar proyecto" };
            dialog(ctx, "quit-save", title, 440.0, |ui| {
                let mut cancel = false;
                if !*naming {
                    para(ui, "¿Quieres guardar el proyecto antes de cerrar cupcraft?");
                    ui.add_space(4.0);
                    hint(ui, "Si eliges «No», el proyecto no se guarda y se pierden los cambios de esta sesión.");
                    let (mut yes, mut no) = (false, false);
                    footer(ui, |ui| {
                        yes = primary_button(ui, "Sí, guardar").clicked();
                        no = danger_button(ui, "No guardar").clicked();
                        cancel = secondary_button(ui, "Cancelar").clicked();
                    });
                    if yes {
                        *naming = true;
                    }
                    if no {
                        app.quit_discarding(&ctx2);
                    }
                    !(cancel || no)
                } else {
                    ui.label(RichText::new("Ponle un nombre a tu proyecto").color(theme::MUTED).size(12.0));
                    let r = ui.add(egui::TextEdit::singleline(name).char_limit(80).desired_width(f32::INFINITY));
                    r.request_focus();
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let mut save = enter;
                    let mut back = false;
                    footer(ui, |ui| {
                        save |= primary_button(ui, "Guardar y salir").clicked();
                        back = secondary_button(ui, "Atrás").clicked();
                        cancel = secondary_button(ui, "Cancelar").clicked();
                    });
                    if back {
                        *naming = false;
                    }
                    if save && !name.trim().is_empty() {
                        app.quit_saving(&ctx2, name);
                    }
                    !(cancel || (save && !name.trim().is_empty()))
                }
            })
        }
        Modal::ConfirmDelete { id, name } => {
            let (id, name) = (id.clone(), name.clone());
            dialog(ctx, "delete", "Eliminar proyecto", 420.0, |ui| {
                para(ui, &format!("Se eliminará “{name}” y sus medios de este equipo. Esta acción no se puede deshacer."));
                let mut del = false;
                let mut cancel = false;
                footer(ui, |ui| {
                    del = danger_button(ui, "Eliminar").clicked();
                    cancel = secondary_button(ui, "Cancelar").clicked();
                });
                if del {
                    app.delete_project(&id);
                }
                !(del || cancel)
            })
        }
        Modal::Templates => dialog(ctx, "templates", "Plantillas", 760.0, |ui| {
            hint(ui, "Se creará un proyecto nuevo con esta plantilla y se guardará el proyecto actual. Después podrás añadir tus propios medios.");
            ui.add_space(8.0);
            let mut pick = None;
            egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                let cols = 4;
                let w = ((ui.available_width() - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
                for row in panels::TEMPLATES.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for t in row {
                            if panels::template_card(ui, t, w).on_hover_text(format!("{} · {}", t.name, t.ratio)).clicked() {
                                pick = Some(t.id);
                            }
                        }
                    });
                    ui.add_space(8.0);
                }
            });
            let mut close = false;
            footer(ui, |ui| close = secondary_button(ui, "Cerrar").clicked());
            if let Some(id) = pick {
                if let Some(t) = panels::TEMPLATES.iter().find(|t| t.id == id) {
                    panels::apply_template(app, t);
                }
            }
            !(close || pick.is_some())
        }),
        Modal::CapCut => capcut(ctx, app),
        Modal::Export => export(ctx, app),
        Modal::SaveAdjust { name } => {
            dialog(ctx, "save-adjust", "Guardar ajuste", 400.0, |ui| {
                ui.label(RichText::new("Nombre del ajuste").color(theme::MUTED).size(12.0));
                let r = ui.add(egui::TextEdit::singleline(name).char_limit(40).desired_width(f32::INFINITY));
                r.request_focus();
                let mut save = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut cancel = false;
                footer(ui, |ui| {
                    save |= primary_button(ui, "Guardar").clicked();
                    cancel = secondary_button(ui, "Cancelar").clicked();
                });
                if save {
                    let adjust = app
                        .editor
                        .selection()
                        .and_then(|id| app.editor.project()?.find_clip(id).map(|(_, c)| c.adjust.clone()))
                        .map(|a| serde_json::to_value(a).unwrap_or(json!({})));
                    match adjust {
                        Some(adj) => {
                            let label = if name.trim().is_empty() { "Ajuste".to_string() } else { name.trim().to_string() };
                            app.custom_adjusts.push(json!({
                                "id": cupcraft_core::new_id("adj"),
                                "label": label,
                                "category": "Tuyos",
                                "adjust": adj,
                            }));
                            app.save_custom_adjusts();
                            app.adjust_cat = "Tuyos".into();
                            app.toast(format!("Ajuste guardado: {label}"));
                        }
                        None => app.toast("Selecciona un clip de video o imagen para guardar su ajuste"),
                    }
                }
                !(save || cancel)
            })
        }
    };
    if keep && app.modal.is_none() {
        app.modal = Some(modal);
    }
}

// ----------------------------------------------------------------------
// Reportar problema (GitHub Issues)
// ----------------------------------------------------------------------

const ISSUES_URL: &str = "https://github.com/Aitor2010aitor/cupcraft/issues/new";
/// Tope de caracteres del registro dentro de la URL (GitHub rechaza URLs muy largas).
const REPORT_LOG_MAX: usize = 2800;

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn log_line(e: &crate::app::LogEntry) -> String {
    let tag = match e.kind {
        LogKind::Info => "INFO",
        LogKind::Warn => "AVISO",
        LogKind::Error => "ERROR",
    };
    let secs = e.time as i64;
    format!("[{:02}:{:02}:{:02}] {tag:<5} {}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60, e.text)
}

/// Abre un issue nuevo en GitHub con plantilla y el final del registro ya
/// pegado. El registro completo también se copia al portapapeles.
fn report_problem(ctx: &egui::Context, app: &mut CupApp, full_log: &str) {
    ctx.copy_text(full_log.to_string());
    // Solo el final del registro (lo más reciente), recortado en límite de línea.
    let total = full_log.chars().count();
    let tail: String = if total > REPORT_LOG_MAX {
        let cut: String = full_log.chars().skip(total - REPORT_LOG_MAX).collect();
        match cut.find('\n') {
            Some(i) => format!("[…recortado; registro completo copiado al portapapeles]\n{}", &cut[i + 1..]),
            None => cut,
        }
    } else {
        full_log.to_string()
    };
    let body = format!(
        "## ¿Qué pasó?\n(Describe el problema)\n\n## ¿Cómo reproducirlo?\n1. \n2. \n3. \n\n## ¿Qué esperabas que pasara?\n\n\n## Entorno\n- cupcraft: {}\n- Sistema: Windows\n\n## Registro\n```\n{}\n```\n\n> Si el registro salió recortado, pega aquí el completo (Ctrl+V): ya está en tu portapapeles.\n",
        env!("CARGO_PKG_VERSION"),
        tail.replace("\r\n", "\n")
    );
    let url = format!("{ISSUES_URL}?title={}&body={}&labels=bug", url_encode("[Bug] "), url_encode(&body));
    // `rundll32` no pasa por cmd, así que `&` en la URL no se interpreta.
    match std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn() {
        Ok(_) => app.toast("Abriendo GitHub… registro copiado al portapapeles"),
        Err(e) => app.toast_error(format!("No se pudo abrir el navegador: {e}")),
    }
}

// ----------------------------------------------------------------------
// Ajustes
// ----------------------------------------------------------------------

fn settings(ctx: &egui::Context, app: &mut CupApp) -> bool {
    let projects = app.list_projects().len();
    let used = app.storage_bytes();
    dialog(ctx, "settings", "Ajustes", 480.0, |ui| {
        panels::section(ui, "Almacenamiento");
        kv(ui, "Modo", "Local · sin cuenta");
        kv(ui, "Proyectos", &projects.to_string());
        kv(ui, "Espacio usado", &format_bytes(used));
        panels::section(ui, "Preferencias");
        kv(ui, "Tema", "Oscuro");
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::i18n::tr("Idioma de la app")).color(theme::MUTED).size(12.5));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let en = crate::i18n::is_english();
                if panels::chip(ui, "English", en).clicked() && !en {
                    crate::i18n::set_english(true);
                }
                if panels::chip(ui, "Español", !en).clicked() && en {
                    crate::i18n::set_english(false);
                }
            });
        });
        panels::section(ui, "Integraciones");
        let mut capcut = false;
        let mut bridge = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new("CapCut").color(theme::MUTED).size(12.5));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| capcut = secondary_button(ui, "Abrir").clicked());
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Puente IA").color(theme::MUTED).size(12.5));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| bridge = secondary_button(ui, "Ver").clicked());
        });
        ui.add_space(8.0);
        note(ui, "Los proyectos, medios y ajustes se guardan en %LOCALAPPDATA%\\openCUP. No se necesitan permisos de administrador.");
        let mut keys = false;
        let mut done = false;
        let mut folder = false;
        let mut report = false;
        footer(ui, |ui| {
            done = primary_button(ui, "Listo").clicked();
            keys = secondary_button(ui, "Atajos").clicked();
            folder = secondary_button(ui, "Abrir carpeta de datos").clicked();
            report = secondary_button(ui, "Reportar problema").clicked();
        });
        if report {
            let text = app.log.iter().map(log_line).collect::<Vec<_>>().join("\n");
            report_problem(ui.ctx(), app, &text);
        }
        if folder {
            let _ = std::process::Command::new("explorer").arg(crate::app::app_dir()).spawn();
        }
        if capcut {
            app.open_capcut();
        } else if bridge {
            app.modal = Some(Modal::Bridge);
        } else if keys {
            app.modal = Some(Modal::Shortcuts);
        }
        !(done || keys || capcut || bridge)
    })
}

// ----------------------------------------------------------------------
// Registro
// ----------------------------------------------------------------------

fn log_dialog(ctx: &egui::Context, app: &mut CupApp, filter: &mut String, only_errors: &mut bool) -> bool {
    let n = app.log.len();
    let title = format!("Registro ({n})");
    dialog(ctx, "log", &title, 680.0, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(filter).hint_text("Filtrar…").desired_width(260.0));
            ui.checkbox(only_errors, "Solo errores");
        });
        ui.add_space(6.0);
        let q = filter.to_lowercase();
        let rows: Vec<String> = app
            .log
            .iter()
            .filter(|e| !*only_errors || e.kind == LogKind::Error)
            .filter(|e| q.is_empty() || e.text.to_lowercase().contains(&q))
            .map(|e| {
                let tag = match e.kind {
                    LogKind::Info => "INFO",
                    LogKind::Warn => "AVISO",
                    LogKind::Error => "ERROR",
                };
                let secs = e.time as i64;
                format!("[{:02}:{:02}:{:02}] {tag:<5} {}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60, e.text)
            })
            .collect();
        egui::Frame::NONE.fill(theme::BG).corner_radius(CornerRadius::from(6)).inner_margin(8).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height((ui.ctx().content_rect().height() - 260.0).max(360.0)).stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                if rows.is_empty() {
                    hint(ui, "No hay entradas.");
                }
                for r in &rows {
                    let color = if r.contains("ERROR") { theme::DANGER } else if r.contains("AVISO") { theme::WARN } else { theme::MUTED };
                    ui.label(RichText::new(r).family(egui::FontFamily::Monospace).size(11.5).color(color));
                }
            });
        });
        hint(ui, "Hora en UTC.");
        let mut close = false;
        let mut copy = false;
        let mut save = false;
        let mut clear = false;
        let mut report = false;
        footer(ui, |ui| {
            close = primary_button(ui, "Cerrar").clicked();
            report = secondary_button(ui, "Reportar en GitHub").clicked();
            clear = secondary_button(ui, "Vaciar").clicked();
            copy = secondary_button(ui, "Copiar").clicked();
            save = secondary_button(ui, "Descargar .txt").clicked();
        });
        let text = rows.join("\r\n");
        if copy {
            ui.ctx().copy_text(text.clone());
            app.toast("Registro copiado al portapapeles");
        }
        if save {
            if let Some(p) = rfd::FileDialog::new().set_file_name("cupcraft-registro.txt").add_filter("Texto", &["txt"]).save_file() {
                match std::fs::write(&p, &text) {
                    Ok(_) => app.toast("Registro guardado"),
                    Err(e) => app.toast_error(format!("No se pudo guardar el registro: {e}")),
                }
            }
        }
        if report {
            let all = app.log.iter().map(log_line).collect::<Vec<_>>().join("\n");
            report_problem(ui.ctx(), app, &all);
        }
        if clear {
            app.log.clear();
        }
        !close
    })
}

// ----------------------------------------------------------------------
// Nuevo proyecto
// ----------------------------------------------------------------------

fn ratio_preview(ui: &mut egui::Ui, w: u32, h: u32, ratio: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(248.0, 172.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 8.0, theme::BG);
    let inner = rect.shrink(18.0);
    let s = (inner.width() / w as f32).min(inner.height() / h as f32);
    let r = Rect::from_center_size(inner.center(), Vec2::new(w as f32 * s, h as f32 * s));
    paint::rounded_gradient(p, r, 4, Color32::from_rgb(0x1d, 0x3c, 0x40), Color32::from_rgb(0x10, 0x1c, 0x1e));
    p.rect_stroke(r, 4.0, Stroke::new(1.5, theme::ACCENT), StrokeKind::Inside);
    p.text(r.center(), egui::Align2::CENTER_CENTER, ratio, egui::FontId::proportional(14.0), theme::TEXT);
}

fn new_project(
    ctx: &egui::Context,
    app: &mut CupApp,
    cat: &mut usize,
    tpl: &mut usize,
    quality: &mut usize,
    fps: &mut usize,
    name: &mut String,
) -> bool {
    dialog(ctx, "new-project", "Elige el diseño de tu vídeo", 860.0, |ui| {
        *cat = (*cat).min(DESIGN_CATS.len() - 1);
        let designs = DESIGN_CATS[*cat].1;
        *tpl = (*tpl).min(designs.len() - 1);
        let mut create: Option<(String, &'static str, u32, u32, u32, Value)> = None;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            // Categorías
            ui.vertical(|ui| {
                ui.set_width(150.0);
                ui.label(RichText::new("Elegir diseño").color(theme::MUTED).size(12.0));
                ui.add_space(4.0);
                for (i, (c, _)) in DESIGN_CATS.iter().enumerate() {
                    let active = *cat == i;
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(150.0, 30.0), Sense::click());
                    if active || resp.hovered() {
                        ui.painter().rect_filled(r, 6.0, if active { theme::ACCENT_SOFT } else { theme::PANEL2 });
                    }
                    ui.painter().text(
                        r.left_center() + Vec2::new(10.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        *c,
                        egui::FontId::proportional(13.0),
                        if active { theme::ACCENT } else { theme::TEXT },
                    );
                    if resp.clicked() {
                        *cat = i;
                        *tpl = 0;
                    }
                }
            });
            // Plantillas
            ui.vertical(|ui| {
                ui.set_width(220.0);
                ui.label(RichText::new("Plantilla").color(theme::MUTED).size(12.0));
                ui.add_space(4.0);
                for (i, d) in DESIGN_CATS[*cat].1.iter().enumerate() {
                    let active = *tpl == i;
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(220.0, 40.0), Sense::click());
                    let p = ui.painter();
                    p.rect_filled(r, 6.0, if active { theme::ACCENT_SOFT } else if resp.hovered() { theme::PANEL2 } else { theme::PANEL });
                    if active {
                        p.rect_stroke(r, 6.0, Stroke::new(1.0, theme::ACCENT), StrokeKind::Inside);
                    }
                    // Icono con la forma de la relación de aspecto.
                    let s = 18.0 / d.w.max(d.h) as f32;
                    let ir = Rect::from_center_size(r.left_center() + Vec2::new(18.0, 0.0), Vec2::new(d.w as f32 * s, d.h as f32 * s));
                    p.rect_stroke(ir, 2.0, Stroke::new(1.3, if active { theme::ACCENT } else { theme::MUTED }), StrokeKind::Inside);
                    p.text(r.left_top() + Vec2::new(36.0, 7.0), egui::Align2::LEFT_TOP, d.label, egui::FontId::proportional(12.5), theme::TEXT);
                    p.text(
                        r.left_top() + Vec2::new(36.0, 23.0),
                        egui::Align2::LEFT_TOP,
                        format!("{} · {}×{}", d.ratio, d.w, d.h),
                        egui::FontId::proportional(11.0),
                        theme::MUTED,
                    );
                    if resp.clicked() {
                        *tpl = i;
                    }
                    if resp.double_clicked() {
                        let (w, h) = scaled(d, QUALITIES[*quality].1);
                        create = Some((name.clone(), d.ratio, w, h, FPS_OPTS[*fps], json!({ "id": d.id, "label": d.label })));
                    }
                    ui.add_space(4.0);
                }
            });
            // Calidad, fps y vista previa
            ui.vertical(|ui| {
                let d = &DESIGN_CATS[*cat].1[*tpl];
                ui.label(RichText::new("Calidad").color(theme::MUTED).size(12.0));
                ui.horizontal(|ui| {
                    for (i, (l, _)) in QUALITIES.iter().enumerate() {
                        if chip(ui, l, *quality == i).clicked() {
                            *quality = i;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Fotogramas por segundo").color(theme::MUTED).size(12.0));
                ui.horizontal(|ui| {
                    for (i, f) in FPS_OPTS.iter().enumerate() {
                        if chip(ui, &format!("{f} fps"), *fps == i).clicked() {
                            *fps = i;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Vista previa").color(theme::MUTED).size(12.0));
                let (w, h) = scaled(d, QUALITIES[*quality].1);
                ratio_preview(ui, w, h, d.ratio);
                ui.label(RichText::new(format!("{w}×{h} · {}", d.ratio)).color(theme::TEXT).size(12.5));
                hint(ui, "La vista previa muestra la relación de aspecto del lienzo");
                ui.add_space(8.0);
                ui.label(RichText::new("Nombre").color(theme::MUTED).size(12.0));
                ui.add(egui::TextEdit::singleline(name).char_limit(80).desired_width(248.0));
            });
        });
        let mut later = false;
        let mut cont = false;
        let mut cancel = false;
        footer(ui, |ui| {
            cont = primary_button(ui, "Continuar").clicked();
            later = secondary_button(ui, "Decidir más tarde").clicked();
            cancel = secondary_button(ui, "Cancelar").clicked();
        });
        if cont {
            let d = &DESIGN_CATS[*cat].1[*tpl];
            let (w, h) = scaled(d, QUALITIES[*quality].1);
            create = Some((name.clone(), d.ratio, w, h, FPS_OPTS[*fps], json!({ "id": d.id, "label": d.label })));
        }
        if later {
            create = Some((name.clone(), "16:9", 1920, 1080, 30, json!({ "id": "later" })));
        }
        let done = create.is_some();
        if let Some((n, ratio, w, h, f, design)) = create {
            app.save_current();
            app.create_designed_project(&n, ratio, w, h, f, design);
            app.toast("Proyecto creado");
        }
        !(done || cancel)
    })
}

/// Escala el diseño para que su lado corto coincida con la calidad elegida.
fn scaled(d: &Design, short: u32) -> (u32, u32) {
    let s = short as f64 / d.w.min(d.h) as f64;
    let even = |v: f64| ((v / 2.0).round() * 2.0) as u32;
    (even(d.w as f64 * s), even(d.h as f64 * s))
}

// ----------------------------------------------------------------------
// CapCut
// ----------------------------------------------------------------------

fn capcut(ctx: &egui::Context, app: &mut CupApp) -> bool {
    dialog(ctx, "capcut", "Proyectos de CapCut", 760.0, |ui| {
        if let Some((title, msg)) = app.capcut.error.clone() {
            egui::Frame::NONE.fill(Color32::from_rgb(0x3a, 0x1c, 0x1c)).corner_radius(CornerRadius::from(8)).inner_margin(10).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(title).strong().color(theme::DANGER));
                ui.add(egui::Label::new(RichText::new(msg).size(12.0).color(theme::TEXT)).wrap());
            });
            ui.add_space(8.0);
        }
        if let Some((name, status, prog)) = app.capcut.importing.clone() {
            ui.label(RichText::new(format!("Importando “{name}”")).strong().color(theme::TEXT));
            ui.add(egui::ProgressBar::new(prog).fill(theme::ACCENT).text(status));
            ui.ctx().request_repaint();
        } else if app.capcut.scanning {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Buscando borradores de CapCut…").color(theme::MUTED));
            });
        } else if app.capcut.list.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                icons::icon(ui, "folderOpen", 32.0, theme::MUTED2);
                ui.label(RichText::new("No hay proyectos de CapCut").size(15.0).strong().color(theme::TEXT));
                hint(ui, "Abre CapCut y guarda un borrador, o define la variable de entorno openCUP_CAPCUT_ROOT con la carpeta de borradores.");
            });
            ui.add_space(20.0);
        } else {
            let list = app.capcut.list.clone();
            let mut import: Option<(String, String)> = None;
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                let cols = 3;
                let w = ((ui.available_width() - 10.0 * (cols as f32 - 1.0)) / cols as f32).floor();
                for row in list.chunks(cols) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for it in row {
                            if let Some(x) = capcut_card(ui, app, it, w) {
                                import = Some(x);
                            }
                        }
                    });
                    ui.add_space(10.0);
                }
            });
            if let Some((id, name)) = import {
                app.capcut_import(&id, &name);
            }
        }
        ui.add_space(8.0);
        note(ui, "cupcraft lee draft_content.json y copia los medios a su almacenamiento local. Los archivos originales de CapCut no se modifican.");
        let mut done = false;
        let mut rescan = false;
        let busy = app.capcut.importing.is_some();
        footer(ui, |ui| {
            done = primary_button(ui, "Listo").clicked();
            ui.add_enabled_ui(!busy && !app.capcut.scanning, |ui| rescan = secondary_button(ui, "Volver a buscar").clicked());
        });
        if rescan {
            app.capcut_scan();
        }
        !done
    })
}

fn capcut_card(ui: &mut egui::Ui, app: &mut CupApp, it: &Value, w: f32) -> Option<(String, String)> {
    let id = paint::str_of(it, "id", "").to_string();
    let name = paint::str_of(it, "name", "Proyecto").to_string();
    let ratio = paint::str_of(it, "ratio", "16:9").to_string();
    let dur = paint::num_of(it, "duration", 0.0);
    let clips = paint::num_of(it, "clips", 0.0) as u64;
    let counts = it.get("counts").cloned().unwrap_or(json!({}));
    let files: u64 = ["video", "image", "audio"].iter().map(|k| counts.get(*k).and_then(|v| v.as_u64()).unwrap_or(0)).sum();
    let width = paint::num_of(it, "width", 1920.0) as u64;
    let height = paint::num_of(it, "height", 1080.0) as u64;
    let fps = paint::num_of(it, "fps", 30.0).round() as u64;
    let modified = paint::num_of(it, "modified", 0.0) as i64;
    let cover = paint::str_of(it, "cover", "").to_string();
    let mut out = None;
    egui::Frame::NONE.fill(theme::PANEL2).corner_radius(CornerRadius::from(8)).inner_margin(8).show(ui, |ui| {
        ui.set_width(w - 16.0);
        let (img, _) = ui.allocate_exact_size(Vec2::new(w - 16.0, (w - 16.0) * 0.56), Sense::hover());
        ui.painter().rect_filled(img, 6.0, theme::BG);
        let key = format!("capcut-cover:{id}");
        let ctx = ui.ctx().clone();
        if !cover.is_empty() && app.video.texture(&ctx, &key).is_none() {
            app.video.request_image(&key, std::path::Path::new(&cover));
        }
        if let Some((tex, size)) = app.video.texture(&ctx, &key) {
            let s = (img.width() / size.x).min(img.height() / size.y);
            ui.painter_at(img).image(
                tex,
                Rect::from_center_size(img.center(), size * s),
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            icons::paint(ui.painter(), "film", img.center(), 26.0, theme::MUTED2);
        }
        let badge = |p: &egui::Painter, pos: egui::Pos2, txt: &str, fill: Color32| {
            let font = egui::FontId::proportional(10.0);
            let g = p.layout_no_wrap(txt.to_string(), font.clone(), Color32::WHITE);
            let r = Rect::from_min_size(pos, Vec2::new(g.size().x + 8.0, 15.0));
            p.rect_filled(r, 3.0, fill);
            p.text(r.center(), egui::Align2::CENTER_CENTER, txt, font, Color32::WHITE);
        };
        badge(ui.painter(), img.min + Vec2::new(6.0, 6.0), &ratio, Color32::from_black_alpha(170));
        if clips == 0 {
            badge(ui.painter(), img.min + Vec2::new(46.0, 6.0), "Vacío", paint::with_alpha(theme::WARN, 0.85));
        }
        ui.add_space(4.0);
        let font = egui::FontId::proportional(13.0);
        let t = paint::ellipsize(ui.painter(), &name, &font, w - 20.0);
        ui.label(RichText::new(t).strong().color(theme::TEXT));
        hint(ui, &format!("{} · {} clip(s) · {} archivo(s)", format_time(dur), clips, files));
        hint(ui, &format!("{width} × {height} · {fps} fps"));
        if modified > 0 {
            hint(ui, &time_ago(modified));
        }
        ui.add_space(4.0);
        let busy = app.capcut.importing.is_some();
        ui.add_enabled_ui(!busy, |ui| {
            if primary_button(ui, "Abrir").clicked() {
                out = Some((id.clone(), name.clone()));
            }
        });
    });
    out
}

// ----------------------------------------------------------------------
// Exportación
// ----------------------------------------------------------------------

fn export(ctx: &egui::Context, app: &mut CupApp) -> bool {
    let status = app.export_handle.as_ref().map(|h| (h.status(), h.progress()));
    dialog(ctx, "export", "Exportar", 460.0, |ui| {
        match status.clone() {
            Some((ExportStatus::Running(msg), prog)) => {
                ui.label(RichText::new("Exportando video…").strong().color(theme::TEXT));
                ui.add_space(6.0);
                ui.add(egui::ProgressBar::new(prog).fill(theme::ACCENT).show_percentage());
                hint(ui, &msg);
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                let mut cancel = false;
                footer(ui, |ui| cancel = danger_button(ui, "Cancelar").clicked());
                if cancel {
                    if let Some(h) = &app.export_handle {
                        h.cancel();
                    }
                    app.toast("Cancelando exportación…");
                }
                true
            }
            Some((ExportStatus::Done(path), _)) => {
                ui.vertical_centered(|ui| {
                    icons::icon(ui, "check", 32.0, theme::ACCENT);
                    ui.label(RichText::new("Exportación completada").size(15.0).strong().color(theme::TEXT));
                    hint(ui, &path.display().to_string());
                });
                let mut close = false;
                let mut open = false;
                footer(ui, |ui| {
                    close = primary_button(ui, "Listo").clicked();
                    open = secondary_button(ui, "Abrir carpeta").clicked();
                });
                if open {
                    open_in_explorer(&path);
                }
                if close {
                    app.export_handle = None;
                }
                !close
            }
            Some((ExportStatus::Failed(e), _)) => {
                ui.label(RichText::new("La exportación falló").strong().color(theme::DANGER));
                ui.add(egui::Label::new(RichText::new(e).size(12.0).color(theme::TEXT)).wrap());
                let mut close = false;
                let mut retry = false;
                footer(ui, |ui| {
                    close = secondary_button(ui, "Cerrar").clicked();
                    retry = primary_button(ui, "Volver a intentarlo").clicked();
                });
                if close || retry {
                    app.export_handle = None;
                }
                !close
            }
            Some((ExportStatus::Cancelled, _)) | None => {
                if matches!(status, Some((ExportStatus::Cancelled, _))) {
                    note(ui, "La exportación anterior se canceló.");
                    ui.add_space(6.0);
                }
                let e = &mut app.export;
                egui::Grid::new("export-grid").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                    ui.label(RichText::new("Formato").color(theme::MUTED));
                    ui.horizontal(|ui| {
                        for (i, l) in ["MP4 (H.264)", "MP4 (H.265)", "WebM (VP9)"].iter().enumerate() {
                            if chip(ui, l, e.format == i as u8).clicked() {
                                e.format = i as u8;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(RichText::new("Resolución").color(theme::MUTED));
                    ui.horizontal(|ui| {
                        for (i, l) in ["1080p", "720p", "4K"].iter().enumerate() {
                            if chip(ui, l, e.resolution == i as u8).clicked() {
                                e.resolution = i as u8;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(RichText::new("Fotogramas").color(theme::MUTED));
                    ui.horizontal(|ui| {
                        for (i, l) in ["24 fps", "30 fps", "60 fps"].iter().enumerate() {
                            if chip(ui, l, e.fps == i as u8).clicked() {
                                e.fps = i as u8;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(RichText::new("Aceleración").color(theme::MUTED));
                    ui.checkbox(&mut e.use_gpu, "Usar GPU si está disponible");
                    ui.end_row();
                });
                let dur = app.editor.project().map(|p| p.duration()).unwrap_or(0.0);
                ui.add_space(6.0);
                hint(ui, &format!("Duración del proyecto: {}", format_time(dur)));
                if cupcraft_media::FfmpegLocation::find().is_none() {
                    ui.add_space(6.0);
                    note(ui, if cfg!(windows) {
                        "No se encontró FFmpeg. Instálalo para tu usuario (winget install Gyan.FFmpeg) o coloca ffmpeg.exe junto a cupcraft.exe."
                    } else {
                        "No se encontró FFmpeg. Instálalo con: sudo apt install ffmpeg (o usa Subtítulos automáticos, que lo instala pidiendo permiso de administrador)."
                    });
                }
                let mut go = false;
                let mut cancel = false;
                footer(ui, |ui| {
                    go = primary_button(ui, "Exportar").clicked();
                    cancel = secondary_button(ui, "Cancelar").clicked();
                });
                if go {
                    app.export_handle = None;
                    app.start_export(ui.ctx());
                    // start_export vuelve a abrir este diálogo si arranca.
                    return app.export_handle.is_some() && app.modal.is_none();
                }
                if cancel {
                    app.export_handle = None;
                }
                !cancel
            }
        }
    })
}
