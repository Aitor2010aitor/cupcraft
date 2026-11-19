//! Estado principal de cupcraft (pantallas, proyecto abierto, almacén, toasts).

use crate::theme;
use cupcraft_audio::AudioEngine;
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
    Stickers,
    Effects,
    Transitions,
    Filters,
    Adjust,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ToolMode {
    #[default]
    Select,
    Split,
}

/// Estado del diálogo de exportación (None = cerrado).
#[derive(Default)]
pub struct ExportState {
    pub open: bool,
    pub format: u8,          // 0 = MP4 H.264, 1 = MP4 H.265, 2 = WebM VP9
    pub resolution: u8,      // 0 = 1080p, 1 = 720p, 2 = 4K
    pub fps: u8,             // 0 = 30, 1 = 60
    pub use_gpu: bool,
    pub progress: f32,       // 0.0–1.0, -1.0 = error
    pub running: bool,
    pub message: String,
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
    pub tool_mode: ToolMode,
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
    /// Motor de audio en tiempo real (Some cuando hay proyecto abierto).
    pub audio_engine: Option<AudioEngine>,
    /// Estado del diálogo de exportación.
    pub export: ExportState,
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
            tool_mode: ToolMode::Select,
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
            audio_engine: None,
            export: ExportState { use_gpu: true, ..Default::default() },
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
                let engine = AudioEngine::new(p.clone());
                self.editor.open(p);
                self.audio_engine = Some(engine);
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
        let engine = AudioEngine::new(p.clone());
        self.editor.open(p);
        self.audio_engine = Some(engine);
        self.screen = Screen::Editor;
        self.playhead = 0.0;
        self.playing = false;
        let _ = id;
    }

    pub fn back_home(&mut self) {
        self.save_current();
        self.audio_pause();
        self.playing = false;
        self.screen = Screen::Home;
    }

    /// Arranca la reproducción de audio y vídeo.
    pub fn audio_play(&mut self) {
        if let Some(eng) = &self.audio_engine {
            eng.seek(self.playhead);
            eng.play();
        }
    }

    /// Pausa el audio.
    pub fn audio_pause(&mut self) {
        if let Some(eng) = &self.audio_engine {
            eng.pause();
        }
    }

    /// Busca el audio a un instante concreto.
    pub fn audio_seek(&mut self, t: f64) {
        if let Some(eng) = &self.audio_engine {
            eng.seek(t);
        }
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        let now = now_secs();
        if self.playing {
            // Si el engine de audio está corriendo, tomamos su reloj como fuente de verdad
            // (así el vídeo y el audio van sincronizados exactamente).
            if let Some(eng) = &self.audio_engine {
                self.playhead = eng.current_time();
            } else {
                let dt = now - self.last_tick;
                self.playhead += dt;
            }
            self.last_tick = now;
            if let Some(p) = self.editor.project() {
                let len = p.timeline_length();
                if self.playhead >= len {
                    self.playhead = len;
                    self.playing = false;
                    self.audio_pause();
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
        egui::Area::new(egui::Id::unique("toast"))
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
