//! Estado principal de cupcraft (pantallas, proyecto abierto, almacén, toasts).

use crate::home;
use crate::theme;
use cupcraft_core::Editor;
use cupcraft_persist::Store;
use eframe::egui;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanelTab {
    Media,
    Text,
    Filters,
    Adjust,
}

pub struct CupApp {
    pub screen: Screen,
    pub editor: Editor,
    pub store: Option<Store>,
    pub store_path: PathBuf,
    pub toast: Option<(String, f64)>,
    pub pending_delete: Option<(String, f64)>,
    pub rename_id: Option<String>,
    pub rename_buf: String,
    pub playhead: f64,
    pub playing: bool,
    pub last_tick: f64,
    pub zoom: f64,
    pub snap: bool,
    pub panel_tab: PanelTab,
    pub filter_category: Option<String>,
    pub filter_query: String,
    pub adjust_cat: String,
    pub text_search: String,
    pub text_content: String,
    pub selected_text_style: Option<String>,
    pub media_search: String,
    pub status: String,
    pub fps: u32,
}

impl CupApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let store_path = store_path();
        let store = Store::open(&store_path).ok().or_else(|| Store::open_in_memory().ok());
        let mut app = Self {
            screen: Screen::Home,
            editor: Editor::new(),
            store,
            store_path,
            toast: None,
            pending_delete: None,
            rename_id: None,
            rename_buf: String::new(),
            playhead: 0.0,
            playing: false,
            last_tick: now_secs(),
            zoom: 1.0,
            snap: true,
            panel_tab: PanelTab::Media,
            filter_category: None,
            filter_query: String::new(),
            adjust_cat: "Básicos".to_string(),
            text_search: String::new(),
            text_content: "Tu título aquí".to_string(),
            selected_text_style: None,
            media_search: String::new(),
            status: String::new(),
            fps: 30,
        };
        let _ = &mut app;
        app
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), now_secs() + 2.6));
    }

    pub fn mutate_project<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut cupcraft_core::Project) -> R,
    {
        self.editor.mutate(f)
    }

    pub fn save_current(&mut self) {
        if let (Some(store), Some(project)) = (self.store.as_mut(), self.editor.project()) {
            let mut p = project.clone();
            p.updated_at = cupcraft_core::now_millis();
            let _ = store.save_project(&p);
        }
    }

    pub fn open_project(&mut self, id: &str) {
        if let Some(store) = &self.store {
            if let Ok(Some(p)) = store.get_project(id) {
                self.editor.open(p);
                self.screen = Screen::Editor;
                self.playhead = 0.0;
                self.playing = false;
            }
        }
    }

    pub fn new_project(&mut self) {
        let p = cupcraft_core::Project::new("Sin título");
        let id = p.id.clone();
        if let Some(store) = &mut self.store {
            let _ = store.save_project(&p);
        }
        self.editor.open(p);
        self.screen = Screen::Editor;
        self.playhead = 0.0;
        self.playing = false;
        let _ = id;
    }

    pub fn back_home(&mut self) {
        self.save_current();
        self.playing = false;
        self.screen = Screen::Home;
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        let now = now_secs();
        if self.playing {
            let dt = now - self.last_tick;
            self.last_tick = now;
            self.playhead += dt;
            if let Some(p) = self.editor.project() {
                let len = p.timeline_length();
                if self.playhead > len {
                    self.playhead = len;
                    self.playing = false;
                }
            }
            ctx.request_repaint();
        }
        // toast expiry
        if let Some((_, exp)) = &self.toast {
            if now > *exp {
                self.toast = None;
            } else {
                ctx.request_repaint();
            }
        }
        // pending delete expiry
        if let Some((_, exp)) = &self.pending_delete {
            if now > *exp {
                self.pending_delete = None;
            }
        }
    }

    pub fn delete_project(&mut self, id: &str) {
        if let Some(store) = &mut self.store {
            let _ = store.delete_project(id);
        }
        self.toast("Proyecto eliminado");
    }

    pub fn rename_project(&mut self, id: &str, name: &str) {
        if let Some(store) = &mut self.store {
            if let Ok(Some(mut p)) = store.get_project(id) {
                p.name = name.to_string();
                p.updated_at = cupcraft_core::now_millis();
                let _ = store.save_project(&p);
            }
        }
    }
}

impl eframe::App for CupApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        match self.screen {
            crate::app::Screen::Home => crate::home::show(ui, self),
            crate::app::Screen::Editor => crate::editor::show(ui, self),
        }
    }
}

fn store_path() -> PathBuf {
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(base).join("openCUP").join("store.sqlite")
    } else {
        PathBuf::from("store.sqlite")
    }
}

fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub fn show_toast(ctx: &egui::Context, app: &CupApp) {
    if let Some((msg, _)) = &app.toast {
        let msg = msg.clone();
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -24.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(theme::PANEL3)
                    .stroke(egui::Stroke::new(1.0, theme::LINE_SOFT))
                    .corner_radius(egui::CornerRadius::from(8))
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(msg).color(theme::TEXT).size(13.0));
                    });
            });
    }
}
