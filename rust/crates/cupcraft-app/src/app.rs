//! Estado principal de cupcraft: pantallas, proyecto abierto, almacén, diálogos,
//! registro, tareas en segundo plano (importación, audio, CapCut, voz) y
//! exportación. Todo nativo: sin WebView, navegador ni servidor local.

use crate::services::{self, export::ExportHandle, media};
use crate::theme;
use cupcraft_audio::{AudioBuffer, AudioEngine};
use cupcraft_core::{ClipKind, Editor, Media, Project};
use cupcraft_persist::Store;
use eframe::egui;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
}

/// Las 10 pestañas del panel de recursos (`TABS` de la versión original).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelTab {
    Media,
    Audio,
    Text,
    Stickers,
    Effects,
    Transitions,
    Captions,
    Filters,
    Adjust,
    Templates,
}

impl PanelTab {
    pub const ALL: [PanelTab; 10] = [
        PanelTab::Media,
        PanelTab::Audio,
        PanelTab::Text,
        PanelTab::Stickers,
        PanelTab::Effects,
        PanelTab::Transitions,
        PanelTab::Captions,
        PanelTab::Filters,
        PanelTab::Adjust,
        PanelTab::Templates,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PanelTab::Media => "Archivos multimedia",
            PanelTab::Audio => "Audio",
            PanelTab::Text => "Texto",
            PanelTab::Stickers => "Stickers",
            PanelTab::Effects => "Efectos",
            PanelTab::Transitions => "Transiciones",
            PanelTab::Captions => "Subtítulos",
            PanelTab::Filters => "Filtros",
            PanelTab::Adjust => "Ajuste",
            PanelTab::Templates => "Plantillas",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            PanelTab::Media => "Archivos",
            PanelTab::Audio => "Audio",
            PanelTab::Text => "Texto",
            PanelTab::Stickers => "Stickers",
            PanelTab::Effects => "Efectos",
            PanelTab::Transitions => "Transic.",
            PanelTab::Captions => "Subtít.",
            PanelTab::Filters => "Filtros",
            PanelTab::Adjust => "Ajustar",
            PanelTab::Templates => "Plantillas",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            PanelTab::Media => "film",
            PanelTab::Audio => "music",
            PanelTab::Text => "text",
            PanelTab::Stickers => "sticker",
            PanelTab::Effects => "sparkles",
            PanelTab::Transitions => "transition",
            PanelTab::Captions => "captions",
            PanelTab::Filters => "palette",
            PanelTab::Adjust => "adjust",
            PanelTab::Templates => "templates",
        }
    }

    /// Textos de ayuda (`PANEL_HELP`), adaptados al almacenamiento local nativo.
    pub fn help(self) -> &'static str {
        match self {
            PanelTab::Media => "Importa videos, imágenes y audio desde tu equipo. También puedes arrastrarlos directamente a la ventana. Los archivos se copian al almacenamiento local de cupcraft (%LOCALAPPDATA%\\openCUP).",
            PanelTab::Audio => "Añade música y efectos de sonido, o graba tu voz con el micrófono. El volumen y los fundidos se ajustan desde el panel Detalles.",
            PanelTab::Text => "Añade títulos y subtítulos. Haz clic en un estilo para insertarlo en el cursor y edítalo desde el panel Detalles.",
            PanelTab::Stickers => "Stickers organizados por categorías. Se insertan como capas independientes que puedes escalar, mover, rotar y animar con fundidos.",
            PanelTab::Effects => "Efectos visuales en tiempo real: desenfoque, luz, grano, viñeta, espejo, vibración y más. Puedes combinarlos con filtros y ajustes.",
            PanelTab::Transitions => "Transiciones entre dos clips: se ponen en el corte (botón «+» en la línea de tiempo) y se mezclan con el clip anterior de la misma pista.",
            PanelTab::Captions => "Añade subtítulos manuales con estilos, importa un archivo SRT o genera subtítulos automáticos desde la voz de un clip (Whisper local).",
            PanelTab::Filters => "Los filtros cambian el color del clip seleccionado. Se agrupan por familias y se incluyen al exportar.",
            PanelTab::Adjust => "Ajusta brillo, contraste, saturación, opacidad, escala y rotación del clip seleccionado.",
            PanelTab::Templates => "Crea un proyecto nuevo con una composición inicial de texto y relación de aspecto. Después añade tus propios medios.",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ToolMode {
    #[default]
    Select,
    Split,
}

/// Vista del raíl izquierdo del editor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LeftView {
    Media,
    Timeline,
    Secondary,
}

/// Diálogos modales (sustituyen a los `openModal` de la versión web).
#[derive(Clone, PartialEq)]
pub enum Modal {
    Pro,
    Share,
    Shortcuts,
    About,
    Settings,
    Notifications,
    /// Novedades que se enseñan al abrir la app justo después de actualizarla.
    WhatsNew { version: String, notes: String },
    Bridge,
    Unavailable { title: String, msg: String },
    Help(PanelTab),
    Log { filter: String, only_errors: bool },
    NewProject { cat: usize, tpl: usize, quality: usize, fps: usize, name: String },
    Rename { id: String, name: String },
    ConfirmDelete { id: String, name: String },
    Templates,
    CapCut,
    Export,
    SaveAdjust { name: String },
    /// Al cerrar la app con un proyecto abierto: ¿guardar? (`naming` = paso de elegir nombre).
    QuitSave { naming: bool, name: String },
}

/// Ajustes del diálogo de exportación.
pub struct ExportState {
    pub format: u8,     // 0 = MP4 H.264, 1 = MP4 H.265, 2 = WebM VP9
    pub resolution: u8, // 0 = 1080p, 1 = 720p, 2 = 4K
    pub fps: u8,        // 0 = 24, 1 = 30, 2 = 60
    pub use_gpu: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Warn,
    Error,
}

pub struct LogEntry {
    pub time: f64,
    pub kind: LogKind,
    pub text: String,
}

/// Mensajes de los hilos de fondo hacia la UI.
pub enum JobMsg {
    Imported { res: Result<Media, String>, to_timeline: bool, project: String },
    AudioDecoded { project: String, id: String, buf: AudioBuffer },
    CapCutScan(Vec<Value>),
    CapCutProgress(String, f32),
    CapCutDone(Result<(Project, usize, Vec<String>), String>),
    Tts(Result<Media, String>),
    WhisperProgress(String, f32),
    WhisperInstalled(Result<(), String>),
    VoiceProgress(String, f32),
    VoiceInstalled(Result<(), String>),
    /// (subtítulos con tiempos del tramo, inicio del clip en la línea de tiempo)
    Transcribed(Result<(Vec<Value>, f64), String>),
}

/// Estado del diálogo de proyectos de CapCut.
#[derive(Default)]
pub struct CapCutState {
    pub scanning: bool,
    pub list: Vec<Value>,
    pub importing: Option<(String, String, f32)>, // (nombre, estado, progreso)
    pub error: Option<(String, String)>,
}

/// Grabación de voz con el micrófono (CPAL, sin navegador).
pub struct Recorder {
    pub stream: cpal::Stream,
    pub samples: std::sync::Arc<std::sync::Mutex<Vec<f32>>>,
    pub rate: u32,
    pub channels: u16,
    pub started: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HomeSort {
    Recent,
    Name,
    Oldest,
}

pub struct CupApp {
    pub screen: Screen,
    pub editor: Editor,
    pub store: Option<Store>,
    pub store_path: PathBuf,
    pub toast: Option<(String, f64)>,
    pub modal: Option<Modal>,
    pub log: Vec<LogEntry>,
    pub jobs_tx: Sender<JobMsg>,
    pub jobs_rx: Receiver<JobMsg>,
    pub pending_jobs: usize,
    pub video: services::video::VideoService,
    pub export: ExportState,
    pub export_handle: Option<ExportHandle>,
    pub capcut: CapCutState,
    pub capcut_allowed: cupcraft_capcut::AllowedMedia,
    pub recorder: Option<Recorder>,

    // Reproducción / línea de tiempo
    pub playhead: f64,
    /// Compositor del visor para las transiciones (misma tubería que el export).
    /// Compositor del visor en segundo plano (transiciones / filtros GL).
    pub preview_comp: Option<services::preview_comp::PreviewComp>,
    pub playing: bool,
    /// Botón de volumen del reproductor (`data-act="mute"`).
    pub muted: bool,
    /// Calidad de la vista previa: 0 máxima, 1 alta, 2 media, 3 baja.
    pub preview_quality: u8,
    /// Diálogo de relación de aspecto personalizada y sus valores.
    pub custom_ratio_open: bool,
    pub custom_ratio: (f32, f32),
    pub last_tick: f64,
    pub zoom: f64,
    pub snap: bool,
    pub tool_mode: ToolMode,
    pub updater: services::updater::Updater,
    /// Estado del proyecto al abrirlo (para descartar los cambios si se sale sin guardar).
    pub session_snapshot: Option<Project>,
    /// El proyecto se creó en esta sesión y el usuario aún no ha confirmado guardarlo.
    pub session_new: bool,
    /// Permite que la ventana se cierre de verdad (ya se ha respondido al aviso).
    pub allow_close: bool,
    /// Salir sin volver a guardar en `on_exit`.
    pub skip_save_on_exit: bool,
    pub fps: u32,
    pub audio_engine: Option<AudioEngine>,
    pub audio_buffers: HashMap<String, AudioBuffer>,

    // Editor: menús y paneles
    pub panel_tab: PanelTab,
    pub left_view: LeftView,
    pub menu_open: bool,
    pub settings_menu_open: bool,
    pub title_buf: String,
    pub media_filter: u8, // 0 todo, 1 video, 2 imagen, 3 audio
    pub media_search: String,
    pub audio_search: String,
    pub filter_category: Option<String>,
    pub filter_query: String,
    pub adjust_cat: String,
    pub custom_adjusts: Vec<Value>,
    pub text_tab: u8, // 0 estilos, 1 burbujas, 2 efectos, 3 texto a voz
    pub text_search: String,
    pub text_content: String,
    pub selected_text_style: Option<String>,
    pub sticker_group: usize,
    pub sticker_search: String,
    pub effect_group: u8,
    pub effect_search: String,
    pub transition_category: String,
    pub transition_duration: f64,
    pub transition_search: String,
    pub caption_style: String,
    pub caption_text: String,
    pub caption_duration: f64,
    /// Resaltar la palabra que se está diciendo (estilo TikTok).
    pub caption_karaoke: bool,
    pub caption_karaoke_color: String,
    pub caption_karaoke_mode: String,
    pub tts_text: String,
    pub tts_voice: String,
    pub tts_speed: f32,
    pub tts_busy: bool,
    /// Estilo de voz (`services::voices::STYLES`).
    pub tts_style: String,
    /// Crear también los subtítulos de lo que dice la voz.
    pub tts_captions: bool,
    /// Texto cuya voz se está generando, para subtitularlo al terminar.
    pub tts_pending_captions: Option<String>,
    /// Descarga de voces: (estado, progreso).
    pub voice_busy: Option<(String, f32)>,
    /// Subtítulos automáticos: (estado, progreso) mientras instala/transcribe.
    pub whisper_busy: Option<(String, f32)>,
    pub whisper_lang: String,
    /// Clip a transcribir mientras el usuario elige el efecto de los subtítulos.
    pub caption_dialog: Option<String>,
    /// Efecto elegido en el diálogo (`karaoke::PRESETS`) y si sustituye a los subtítulos actuales.
    pub caption_preset: usize,
    pub caption_replace: bool,

    // Inicio
    pub home_sort: HomeSort,
    pub home_search: String,
    pub home_search_open: bool,
    pub editpilot: String,
    pub more_tools_open: bool,
    pub editpilot_focus: bool,
}

impl CupApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let store_path = store_path();
        if let Some(dir) = store_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let store = Store::open(&store_path).ok().or_else(|| Store::open_in_memory().ok());
        let (jobs_tx, jobs_rx) = channel();
        let mut app = Self {
            screen: Screen::Home,
            editor: Editor::new(),
            store,
            store_path,
            toast: None,
            modal: services::updater::take_whats_new().map(|(version, notes)| Modal::WhatsNew { version, notes }),
            log: Vec::new(),
            jobs_tx,
            jobs_rx,
            pending_jobs: 0,
            video: services::video::VideoService::new(cc.egui_ctx.clone()),
            export: ExportState { format: 0, resolution: 0, fps: 1, use_gpu: true },
            export_handle: None,
            capcut: CapCutState::default(),
            capcut_allowed: Default::default(),
            recorder: None,
            playhead: 0.0,
            preview_comp: None,
            playing: false,
            muted: false,
            preview_quality: 0,
            custom_ratio_open: false,
            custom_ratio: (16.0, 9.0),
            last_tick: now_secs(),
            zoom: 1.0,
            snap: true,
            tool_mode: ToolMode::Select,
            updater: services::updater::Updater::start(&cc.egui_ctx),
            session_snapshot: None,
            session_new: false,
            allow_close: false,
            skip_save_on_exit: false,
            fps: 30,
            audio_engine: None,
            audio_buffers: HashMap::new(),
            panel_tab: PanelTab::Media,
            left_view: LeftView::Media,
            menu_open: false,
            settings_menu_open: false,
            title_buf: String::new(),
            media_filter: 0,
            media_search: String::new(),
            audio_search: String::new(),
            filter_category: None,
            filter_query: String::new(),
            adjust_cat: "Básicos".to_string(),
            custom_adjusts: load_custom_adjusts(),
            text_tab: 0,
            text_search: String::new(),
            text_content: "Tu título aquí".to_string(),
            selected_text_style: None,
            sticker_group: 0,
            sticker_search: String::new(),
            effect_group: 0,
            effect_search: String::new(),
            transition_category: "Todos".to_string(),
            transition_duration: 0.6,
            transition_search: String::new(),
            caption_style: "classic".to_string(),
            caption_text: String::new(),
            caption_duration: 2.5,
            caption_karaoke: true,
            caption_karaoke_color: "#ffe600".to_string(),
            caption_karaoke_mode: "pop".to_string(),
            tts_text: String::new(),
            tts_voice: String::new(),
            tts_speed: 1.0,
            tts_busy: false,
            tts_style: "none".to_string(),
            tts_captions: true,
            tts_pending_captions: None,
            voice_busy: None,
            whisper_busy: None,
            caption_dialog: None,
            caption_preset: 1,
            caption_replace: true,
            whisper_lang: "auto".to_string(),
            home_sort: HomeSort::Recent,
            home_search: String::new(),
            home_search_open: false,
            editpilot: String::new(),
            more_tools_open: false,
            editpilot_focus: false,
        };
        app.log(LogKind::Info, format!("cupcraft {} iniciado (interfaz nativa egui)", env!("CARGO_PKG_VERSION")));
        app.log(LogKind::Info, format!("Almacén local: {}", app.store_path.display()));
        if cupcraft_media::FfmpegLocation::find().is_none() {
            app.log(LogKind::Warn, "FFmpeg no encontrado: la vista previa de vídeo y la exportación lo necesitan.");
        }
        app
    }

    // ------------------------------------------------------------------
    // Registro y avisos
    // ------------------------------------------------------------------

    pub fn log(&mut self, kind: LogKind, text: impl Into<String>) {
        self.log.push(LogEntry { time: now_secs(), kind, text: text.into() });
        if self.log.len() > 20000 {
            self.log.drain(0..2000);
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.log(LogKind::Info, msg.clone());
        self.toast = Some((msg, now_secs() + 2.6));
    }

    pub fn toast_error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.log(LogKind::Error, msg.clone());
        self.toast = Some((msg, now_secs() + 4.0));
    }

    pub fn unavailable(&mut self, title: &str, msg: &str) {
        self.modal = Some(Modal::Unavailable { title: title.into(), msg: msg.into() });
    }

    pub fn ai_unavailable(&mut self) {
        self.unavailable(
            "Funciones de IA",
            "La generación y el procesamiento con IA necesitan un servicio en la nube. Esta versión local incluye EditPilot como borrador de texto y todas las herramientas de edición manual.",
        );
    }

    // ------------------------------------------------------------------
    // Proyecto
    // ------------------------------------------------------------------

    pub fn mutate_project<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Project) -> R,
    {
        let r = self.editor.mutate(f);
        self.sync_engine();
        r
    }

    /// Copia el proyecto al motor de audio tras cada cambio.
    pub fn sync_engine(&mut self) {
        if let (Some(eng), Some(p)) = (&self.audio_engine, self.editor.project()) {
            *eng.project.lock() = p.clone();
        }
    }

    pub fn save_current(&mut self) {
        self.sync_engine();
        if let (Some(store), Some(project)) = (self.store.as_mut(), self.editor.project()) {
            let mut p = project.clone();
            p.updated_at = cupcraft_core::now_millis();
            if let Err(e) = store.save_project(&p) {
                let msg = format!("No se pudo guardar el proyecto: {e}");
                self.log(LogKind::Error, msg);
            }
        }
    }

    pub fn list_projects(&self) -> Vec<Project> {
        self.store.as_ref().and_then(|s| s.list_projects().ok()).unwrap_or_default()
    }

    fn enter_editor(&mut self, p: Project) {
        if let Some(eng) = &self.audio_engine {
            eng.pause();
        }
        self.audio_buffers.clear();
        let engine = AudioEngine::new(p.clone());
        engine.set_muted(self.muted);
        self.title_buf = p.name.clone();
        self.session_snapshot = Some(p.clone());
        self.session_new = false;
        self.fps = p.fps.max(1);
        let pid = p.id.clone();
        let medias = p.media.clone();
        self.editor.open(p);
        self.audio_engine = Some(engine);
        self.screen = Screen::Editor;
        self.playhead = 0.0;
        self.playing = false;
        self.menu_open = false;
        self.settings_menu_open = false;
        // Decodifica en segundo plano el audio de los medios del proyecto.
        for m in medias {
            self.decode_media_audio(&pid, &m);
        }
    }

    pub fn open_project(&mut self, id: &str) {
        let found = self.store.as_ref().and_then(|s| s.get_project(id).ok().flatten());
        match found {
            Some(p) => {
                self.log(LogKind::Info, format!("Proyecto abierto: {}", p.name));
                self.enter_editor(p);
            }
            None => self.toast_error("No se encontró el proyecto"),
        }
    }

    /// Crea y abre un proyecto vacío.
    pub fn new_project_named(&mut self, name: &str) -> String {
        let name = if name.trim().is_empty() { "Proyecto sin título" } else { name.trim() };
        let p = Project::new(name);
        let id = p.id.clone();
        if let Some(store) = &mut self.store {
            let _ = store.save_project(&p);
        }
        self.enter_editor(p);
        self.session_new = true;
        id
    }

    /// Crea un proyecto con diseño (relación, resolución, fps).
    pub fn create_designed_project(&mut self, name: &str, ratio: &str, w: u32, h: u32, fps: u32, design: Value) {
        self.new_project_named(name);
        let res = format!("{}p", w.min(h));
        let ratio = ratio.to_string();
        self.editor.mutate(|p| {
            p.ratio = ratio;
            p.resolution = res;
            p.fps = fps;
            p.extra.insert("design".into(), design);
        });
        self.fps = fps;
        self.sync_engine();
        self.save_current();
    }

    /// Cierre de la app: con un proyecto abierto pregunta antes si se guarda.
    pub fn request_quit(&mut self, ctx: &egui::Context) {
        if self.screen == Screen::Editor {
            if let Some(name) = self.editor.project().map(|p| p.name.clone()) {
                if !matches!(self.modal, Some(Modal::QuitSave { .. })) {
                    self.modal = Some(Modal::QuitSave { naming: false, name });
                }
                return;
            }
        }
        self.allow_close = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Respuesta «Sí» al salir: pone nombre, guarda y cierra.
    pub fn quit_saving(&mut self, ctx: &egui::Context, name: &str) {
        if let Some(id) = self.editor.project().map(|p| p.id.clone()) {
            self.rename_project(&id, name);
        }
        self.save_current();
        self.allow_close = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Respuesta «No» al salir: el proyecto no se guarda. Uno nuevo se borra;
    /// uno existente vuelve al estado con el que se abrió.
    pub fn quit_discarding(&mut self, ctx: &egui::Context) {
        let id = self.editor.project().map(|p| p.id.clone());
        let snapshot = self.session_snapshot.take();
        if let Some(store) = &mut self.store {
            match (self.session_new, id, snapshot) {
                (true, Some(id), _) => {
                    let _ = store.delete_project(&id);
                }
                (false, _, Some(snap)) => {
                    let _ = store.save_project(&snap);
                }
                _ => {}
            }
        }
        self.skip_save_on_exit = true;
        self.allow_close = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub fn back_home(&mut self) {
        self.save_current();
        self.audio_pause();
        self.playing = false;
        self.stop_recording(false);
        self.screen = Screen::Home;
    }

    pub fn delete_project(&mut self, id: &str) {
        if let Some(store) = &mut self.store {
            let _ = store.delete_project(id);
        }
        self.toast("Proyecto eliminado");
    }

    pub fn rename_project(&mut self, id: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if self.editor.project().map(|p| p.id == id).unwrap_or(false) {
            let n = name.to_string();
            self.editor.mutate(|p| p.name = n);
            self.title_buf = name.to_string();
            self.save_current();
            return;
        }
        if let Some(store) = &mut self.store {
            if let Ok(Some(mut p)) = store.get_project(id) {
                p.name = name.to_string();
                p.updated_at = cupcraft_core::now_millis();
                let _ = store.save_project(&p);
            }
        }
    }

    /// "Guardar proyecto como archivo": contenedor nativo `.cupcraft` (ZIP con
    /// `project.json` + `assets/`), portable para compartir sin perder medios.
    pub fn save_project_file(&mut self, id: &str) {
        let Some(store) = self.store.as_mut() else { return };
        let name = store.get_project(id).ok().flatten().map(|p| p.name).unwrap_or_default();
        let bytes = match store.export_cupcraft(id) {
            Ok(b) => b,
            Err(e) => return self.toast_error(format!("No se pudo preparar el archivo: {e}")),
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(cupcraft_persist::export_file_name_cupcraft(&name))
            .add_filter("Proyecto de cupcraft", &[cupcraft_persist::CUPCRAFT_EXT])
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, &bytes) {
            Ok(_) => self.toast(format!("“{name}” guardado como archivo")),
            Err(e) => self.toast_error(format!("No se pudo escribir el archivo: {e}")),
        }
    }

    /// "Abrir proyecto desde archivo": acepta el contenedor `.cupcraft` (ZIP) y,
    /// por compatibilidad, los `.opencup.json` de texto.
    pub fn open_project_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Proyecto de cupcraft", &[cupcraft_persist::CUPCRAFT_EXT, "json"])
            .pick_file()
        else {
            return;
        };
        self.open_project_from_path(&path);
    }

    /// Abre un proyecto desde un archivo en disco: contenedor `.cupcraft` (ZIP,
    /// detectado por su firma) o, por compatibilidad, `.opencup.json` de texto.
    /// La usan tanto el diálogo «Abrir proyecto» como soltar el archivo en la
    /// ventana.
    pub fn open_project_from_path(&mut self, path: &Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return self.toast_error(format!("No se pudo leer el archivo: {e}")),
        };
        let is_zip = bytes.get(0..4) == Some(&cupcraft_persist::ZIP_MAGIC);
        let result = match self.store.as_mut() {
            Some(store) if is_zip => store.import_cupcraft(&bytes),
            Some(store) => match String::from_utf8(bytes) {
                Ok(text) => store.import_project_file(&text),
                Err(_) => return self.toast_error("El archivo no es un proyecto válido"),
            },
            None => return,
        };
        let report = match result {
            Ok(r) => r,
            Err(e) => return self.toast_error(format!("El archivo no es un proyecto válido: {e}")),
        };
        self.open_project(&report.id);
        let name = self.editor.project().map(|p| p.name.clone()).unwrap_or_default();
        if report.missing > 0 {
            self.log(LogKind::Warn, format!("{} medio(s) no incluidos en el archivo", report.missing));
        }
        self.toast(format!("Proyecto “{name}” abierto desde el archivo"));
    }

    /// ¿Es la ruta un archivo de proyecto de cupcraft (por extensión)?
    fn is_project_path(path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case(cupcraft_persist::CUPCRAFT_EXT))
            .unwrap_or(false)
    }

    // ------------------------------------------------------------------
    // Reproducción
    // ------------------------------------------------------------------

    pub fn audio_play(&mut self) {
        if let Some(eng) = &self.audio_engine {
            eng.seek(self.playhead);
            eng.play();
        }
    }

    pub fn audio_pause(&mut self) {
        if let Some(eng) = &self.audio_engine {
            eng.pause();
        }
    }

    pub fn audio_seek(&mut self, t: f64) {
        if let Some(eng) = &self.audio_engine {
            eng.seek(t);
        }
    }

    pub fn toggle_play(&mut self) {
        if self.playing {
            self.playing = false;
            self.audio_pause();
        } else {
            let len = self.editor.project().map(|p| p.timeline_length()).unwrap_or(0.0);
            if self.playhead >= len - 0.01 {
                self.playhead = 0.0;
            }
            self.playing = true;
            self.last_tick = now_secs();
            self.audio_play();
        }
    }

    pub fn seek(&mut self, t: f64) {
        let len = self.editor.project().map(|p| p.timeline_length()).unwrap_or(0.0);
        self.playhead = t.clamp(0.0, len.max(0.0));
        self.audio_seek(self.playhead);
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        let now = now_secs();
        self.poll_jobs(ctx);
        if self.playing {
            let has_out = self.audio_engine.as_ref().map(|e| e.has_output() && e.is_playing.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(false);
            if has_out {
                if let Some(eng) = &self.audio_engine {
                    self.playhead = eng.current_time();
                }
            } else {
                self.playhead += now - self.last_tick;
            }
            self.last_tick = now;
            if let Some(p) = self.editor.project() {
                let len = p.duration();
                if self.playhead >= len {
                    self.playhead = len;
                    self.playing = false;
                    self.audio_pause();
                }
            }
            ctx.request_repaint();
        }
        if let Some((_, exp)) = &self.toast {
            if now > *exp {
                self.toast = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
        if self.recorder.is_some() || self.pending_jobs > 0 || self.capcut.scanning {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
    }

    // ------------------------------------------------------------------
    // Medios
    // ------------------------------------------------------------------

    pub fn import_media_dialog(&mut self) {
        let Some(files) = rfd::FileDialog::new()
            .set_title("Importar medios")
            .add_filter("Video, imagen y audio", media::IMPORT_EXTS)
            .pick_files()
        else {
            return;
        };
        self.import_paths(files, false);
    }

    pub fn import_audio_dialog(&mut self) {
        let Some(files) = rfd::FileDialog::new()
            .set_title("Importar audio")
            .add_filter("Audio", &["mp3", "wav", "ogg", "oga", "opus", "m4a", "aac", "flac"])
            .pick_files()
        else {
            return;
        };
        self.import_paths(files, true);
    }

    /// Importa archivos en segundo plano (abre su propia conexión al almacén).
    pub fn import_paths(&mut self, paths: Vec<PathBuf>, to_timeline: bool) {
        let Some(pid) = self.editor.project().map(|p| p.id.clone()) else { return };
        for path in paths {
            let tx = self.jobs_tx.clone();
            let store_path = self.store_path.clone();
            let pid = pid.clone();
            self.pending_jobs += 1;
            self.log(LogKind::Info, format!("Importando {}", path.display()));
            std::thread::spawn(move || {
                let mut store = Store::open(&store_path).ok();
                let res = media::import_file(store.as_mut(), &path);
                let _ = tx.send(JobMsg::Imported { res, to_timeline, project: pid });
            });
        }
    }

    fn decode_media_audio(&mut self, project: &str, m: &Media) {
        if !media::has_audio(m) || self.audio_buffers.contains_key(&m.id) {
            return;
        }
        let Some(path) = media::ensure_file(self.store.as_ref(), m) else { return };
        let tx = self.jobs_tx.clone();
        let id = m.id.clone();
        let pid = project.to_string();
        std::thread::spawn(move || {
            if let Some(buf) = media::decode_audio(&path, &id) {
                let _ = tx.send(JobMsg::AudioDecoded { project: pid, id, buf });
            }
        });
    }

    pub fn media_path(&self, m: &Media) -> Option<PathBuf> {
        media::ensure_file(self.store.as_ref(), m)
    }

    /// Añade un medio de la biblioteca a la línea de tiempo en el cabezal.
    pub fn add_media_to_timeline(&mut self, media_id: &str) {
        let Some(m) = self.editor.project().and_then(|p| p.media.iter().find(|m| m.id == media_id).cloned()) else {
            return;
        };
        let kind = match m.kind.as_deref() {
            Some("audio") => ClipKind::Audio,
            Some("image") => ClipKind::Image,
            _ => ClipKind::Video,
        };
        let dur = m.duration.unwrap_or(if kind == ClipKind::Image { 5.0 } else { 5.0 }).max(0.1);
        let name = m.name.clone().unwrap_or_else(|| "Medio".into());
        let ph = self.playhead;
        let id = self.mutate_project(|p| p.add_media_clip(None, kind, name.clone(), dur, Some(ph), Some(m.id.clone()))).flatten();
        if let Some(id) = id {
            self.editor.select(Some(id));
            self.save_current();
            self.toast(format!("“{name}” añadido a la línea de tiempo"));
        }
    }

    pub fn remove_media(&mut self, media_id: &str) {
        let mid = media_id.to_string();
        self.mutate_project(|p| {
            let ids: Vec<String> = p
                .tracks
                .iter()
                .flat_map(|t| t.clips.iter())
                .filter(|c| c.media_id.as_deref() == Some(mid.as_str()))
                .map(|c| c.id.clone())
                .collect();
            for id in ids {
                p.remove_clip(&id);
            }
            p.media.retain(|m| m.id != mid);
        });
        if let Some(eng) = &self.audio_engine {
            eng.mixer.lock().remove_media(media_id);
        }
        self.save_current();
        self.toast("Medio eliminado del proyecto");
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.jobs_rx.try_recv() {
            ctx.request_repaint();
            match msg {
                JobMsg::Imported { res, to_timeline, project } => {
                    self.pending_jobs = self.pending_jobs.saturating_sub(1);
                    match res {
                        Ok(m) => self.on_media_imported(m, to_timeline, &project),
                        Err(e) => self.toast_error(e),
                    }
                }
                JobMsg::AudioDecoded { project, id, buf } => {
                    if self.editor.project().map(|p| p.id == project).unwrap_or(false) {
                        if let Some(eng) = &self.audio_engine {
                            eng.mixer.lock().insert_media(id.clone(), buf.clone());
                        }
                        self.audio_buffers.insert(id, buf);
                    }
                }
                JobMsg::CapCutScan(list) => {
                    self.capcut.scanning = false;
                    self.log(LogKind::Info, format!("CapCut: {} borrador(es) detectado(s)", list.len()));
                    self.capcut.list = list;
                }
                JobMsg::CapCutProgress(status, p) => {
                    if let Some(imp) = &mut self.capcut.importing {
                        imp.1 = status;
                        imp.2 = p;
                    }
                }
                JobMsg::CapCutDone(res) => {
                    self.capcut.importing = None;
                    match res {
                        Ok((project, count, warnings)) => {
                            if let Some(store) = &mut self.store {
                                let _ = store.save_project(&project);
                            }
                            for w in &warnings {
                                self.log(LogKind::Warn, format!("CapCut: {w}"));
                            }
                            self.modal = None;
                            self.enter_editor(project);
                            if let Some(first) = warnings.first() {
                                self.toast(format!("Importado con {} aviso(s): {first}", warnings.len()));
                            } else {
                                self.toast(format!("Proyecto de CapCut importado: {count} archivo(s)"));
                            }
                        }
                        Err(e) => {
                            self.log(LogKind::Error, format!("CapCut: {e}"));
                            self.capcut.error = Some(("La importación falló".into(), e));
                        }
                    }
                }
                JobMsg::WhisperProgress(msg, p) => self.whisper_busy = Some((msg, p)),
                JobMsg::WhisperInstalled(res) => {
                    self.whisper_busy = None;
                    match res {
                        Ok(()) => self.toast("Motor Whisper instalado"),
                        Err(e) => self.toast_error(format!("Whisper: {e}")),
                    }
                }
                JobMsg::Transcribed(res) => {
                    self.whisper_busy = None;
                    match res {
                        Ok((mut items, offset)) => {
                            for it in &mut items {
                                if let Some(s) = it.get("start").and_then(Value::as_f64) {
                                    it["start"] = json!(s + offset);
                                }
                                if let Some(e) = it.get("end").and_then(Value::as_f64) {
                                    it["end"] = json!(e + offset);
                                }
                            }
                            if items.is_empty() {
                                self.toast("No se detectó voz en el clip");
                            } else {
                                let replace = self.caption_replace;
                                crate::panels::apply_captions(self, items, replace);
                            }
                        }
                        Err(e) => self.toast_error(format!("Transcripción: {e}")),
                    }
                }
                JobMsg::VoiceProgress(msg, p) => self.voice_busy = Some((msg, p)),
                JobMsg::VoiceInstalled(res) => {
                    self.voice_busy = None;
                    match res {
                        Ok(()) => {
                            crate::panels::refresh_voices();
                            self.toast("Voz instalada");
                        }
                        Err(e) => self.toast_error(format!("Voz: {e}")),
                    }
                }
                JobMsg::Tts(res) => {
                    self.tts_busy = false;
                    let caption_text = self.tts_pending_captions.take();
                    match res {
                        Ok(m) => {
                            let pid = self.editor.project().map(|p| p.id.clone()).unwrap_or_default();
                            let (start, dur) = (self.playhead, m.duration.unwrap_or(0.0));
                            self.on_media_imported(m, true, &pid);
                            // Subtítulos de lo que dice la voz, justo sobre su clip.
                            if let Some(text) = caption_text.filter(|_| dur > 0.1) {
                                let items = crate::karaoke::chunk_text(&text, 4, start, dur);
                                crate::panels::apply_captions(self, items, false);
                            }
                        }
                        Err(e) => self.toast_error(format!("Texto a voz: {e}")),
                    }
                }
            }
        }
    }

    fn on_media_imported(&mut self, m: Media, to_timeline: bool, project: &str) {
        if self.editor.project().map(|p| p.id != project).unwrap_or(true) {
            return;
        }
        let name = m.name.clone().unwrap_or_default();
        let mid = m.id.clone();
        let mc = m.clone();
        self.mutate_project(|p| p.media.push(mc));
        self.decode_media_audio(project, &m);
        if m.extra.get("probeFailed").and_then(|v| v.as_bool()).unwrap_or(false) && m.kind.as_deref() == Some("video") {
            self.log(LogKind::Warn, format!("{name}: FFmpeg no disponible, duración estimada en 5 s"));
        }
        if to_timeline {
            self.add_media_to_timeline(&mid);
        } else {
            self.save_current();
            self.toast(format!("“{name}” importado"));
        }
    }

    // ------------------------------------------------------------------
    // CapCut (lectura local de draft_content.json)
    // ------------------------------------------------------------------

    pub fn open_capcut(&mut self) {
        self.modal = Some(Modal::CapCut);
        self.capcut_scan();
    }

    pub fn capcut_scan(&mut self) {
        self.capcut.scanning = true;
        self.capcut.error = None;
        let tx = self.jobs_tx.clone();
        let allowed = self.capcut_allowed.clone();
        std::thread::spawn(move || {
            let list = cupcraft_capcut::scan_projects(&allowed);
            let _ = tx.send(JobMsg::CapCutScan(list));
        });
    }

    pub fn capcut_import(&mut self, id: &str, name: &str) {
        self.capcut.importing = Some((name.to_string(), "Leyendo draft_content.json…".into(), 0.0));
        self.capcut.error = None;
        let tx = self.jobs_tx.clone();
        let allowed = self.capcut_allowed.clone();
        let store_path = self.store_path.clone();
        let id = id.to_string();
        std::thread::spawn(move || {
            let res = import_capcut(&id, &allowed, &store_path, &tx);
            let _ = tx.send(JobMsg::CapCutDone(res));
        });
    }

    // ------------------------------------------------------------------
    // Exportación
    // ------------------------------------------------------------------

    pub fn start_export(&mut self, ctx: &egui::Context) {
        let Some(project) = self.editor.project().cloned() else { return };
        if project.duration() <= 0.0 {
            return self.toast_error("El proyecto está vacío: añade algún clip a la línea de tiempo.");
        }
        let (format, ext, label) = match self.export.format {
            1 => (cupcraft_media::ExportFormat::Mp4H265, "mp4", "MP4 (H.265)"),
            2 => (cupcraft_media::ExportFormat::WebmVp9, "webm", "WebM (VP9)"),
            _ => (cupcraft_media::ExportFormat::Mp4H264, "mp4", "MP4 (H.264)"),
        };
        let Some(out) = rfd::FileDialog::new()
            .set_title("Exportar video")
            .set_file_name(format!("{}.{ext}", sanitize(&project.name)))
            .add_filter(label, &[ext])
            .save_file()
        else {
            return;
        };
        let short = match self.export.resolution {
            1 => 720,
            2 => 2160,
            _ => 1080,
        };
        let fps = match self.export.fps {
            0 => 24,
            2 => 60,
            _ => 30,
        };
        let (w, h) = services::compose::canvas_size(&project.ratio, short);
        let mut paths = HashMap::new();
        let mut dims = HashMap::new();
        let mut audio = Vec::new();
        for m in &project.media {
            if let Some(p) = self.media_path(m) {
                if media::has_audio(m) {
                    audio.push((m.id.clone(), p.clone()));
                }
                paths.insert(m.id.clone(), p);
            }
            if let Some(d) = media::media_dims(m) {
                dims.insert(m.id.clone(), d);
            }
        }
        self.log(LogKind::Info, format!("Exportando {w}×{h} @ {fps} fps ({label}) → {}", out.display()));
        let job = services::export::ExportJob {
            project,
            out,
            w,
            h,
            fps,
            format,
            use_gpu: self.export.use_gpu,
            paths,
            dims,
            audio,
        };
        self.export_handle = Some(services::export::start(job, ctx.clone()));
        self.modal = Some(Modal::Export);
    }

    /// Fotograma del visor con el compositor completo (transiciones). Se compone
    /// en un hilo aparte; mientras llega, devuelve `None` y el visor pinta las
    /// capas por su cuenta. Al reproducir baja a 360p para ir fluido.
    pub fn preview_composite(&mut self, ctx: &egui::Context, project: &Project, t: f64) -> Option<egui::TextureId> {
        let h = if self.playing { 360 } else { 540 };
        let mut comp = self.preview_comp.take().unwrap_or_else(|| services::preview_comp::PreviewComp::new(ctx));
        let id = comp.frame(ctx, project, t, h, || {
            let mut paths = HashMap::new();
            let mut dims = HashMap::new();
            for m in &project.media {
                if let Some(p) = self.media_path(m) {
                    paths.insert(m.id.clone(), p);
                }
                if let Some(d) = media::media_dims(m) {
                    dims.insert(m.id.clone(), d);
                }
            }
            (paths, dims)
        });
        self.preview_comp = Some(comp);
        id
    }

    /// "Capturar fotograma": compone el instante actual y lo guarda como PNG.
    pub fn capture_frame(&mut self) {
        let Some(project) = self.editor.project().cloned() else { return };
        let Some(out) = rfd::FileDialog::new()
            .set_title("Capturar fotograma")
            .set_file_name(format!("{}-{:.0}.png", sanitize(&project.name), self.playhead * 1000.0))
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        let (w, h) = services::compose::canvas_size(&project.ratio, 1080);
        let mut paths = HashMap::new();
        let mut dims = HashMap::new();
        for m in &project.media {
            if let Some(p) = self.media_path(m) {
                paths.insert(m.id.clone(), p);
            }
            if let Some(d) = media::media_dims(m) {
                dims.insert(m.id.clone(), d);
            }
        }
        let mut comp = services::compose::Compositor::new(w, h, paths, dims);
        let rgba = comp.frame(&project, self.playhead);
        match image::RgbaImage::from_raw(w, h, rgba).map(|img| img.save(&out)) {
            Some(Ok(_)) => self.toast("Fotograma guardado como PNG"),
            Some(Err(e)) => self.toast_error(format!("No se pudo guardar la imagen: {e}")),
            None => self.toast_error("No se pudo componer el fotograma"),
        }
    }

    // ------------------------------------------------------------------
    // Voz: texto a voz (Piper local) y grabación con micrófono (CPAL)
    // ------------------------------------------------------------------

    pub fn tts_root() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    pub fn generate_tts(&mut self) {
        let text = self.tts_text.trim().to_string();
        if text.is_empty() {
            return self.toast("Escribe el texto que quieres convertir en voz");
        }
        if self.editor.project().is_none() {
            return;
        }
        self.tts_busy = true;
        self.tts_pending_captions = self.tts_captions.then(|| text.clone());
        let tx = self.jobs_tx.clone();
        let voice = self.tts_voice.clone();
        let speed = self.tts_speed;
        let style = self.tts_style.clone();
        let store_path = self.store_path.clone();
        std::thread::spawn(move || {
            let res = cupcraft_tts::generate(&Self::tts_root(), &text, &voice, speed)
                .and_then(|wav| services::voices::apply_style(wav, &style))
                .and_then(|wav| {
                let mut store = Store::open(&store_path).ok();
                let short: String = text.chars().take(24).collect();
                media::import_bytes(store.as_mut(), &format!("Voz - {short}.wav"), "wav", wav)
            });
            let _ = tx.send(JobMsg::Tts(res));
        });
    }

    /// Descarga el motor Piper y una voz en español (acción explícita del usuario).
    pub fn install_piper(&mut self) {
        if self.voice_busy.is_some() {
            return;
        }
        self.voice_busy = Some(("Preparando descarga…".into(), 0.0));
        let tx = self.jobs_tx.clone();
        std::thread::spawn(move || {
            let tx2 = tx.clone();
            let res = services::voices::install_piper(&move |m, p| {
                let _ = tx2.send(JobMsg::VoiceProgress(m, p));
            });
            let _ = tx.send(JobMsg::VoiceInstalled(res));
        });
    }

    /// Descarga una voz extra de Piper (acción explícita del usuario).
    pub fn install_voice(&mut self, voice_id: &str) {
        if self.voice_busy.is_some() {
            return;
        }
        let Some(v) = services::voices::CATALOG.iter().find(|v| v.id == voice_id) else { return };
        let Some(models) = cupcraft_tts::models_dir(&Self::tts_root()) else {
            return self.toast_error("No se encuentra la carpeta de Piper (tools\\piper)");
        };
        self.voice_busy = Some(("Preparando descarga…".into(), 0.0));
        let tx = self.jobs_tx.clone();
        std::thread::spawn(move || {
            let tx2 = tx.clone();
            let res = services::voices::install_voice(&models, v, &move |m, p| {
                let _ = tx2.send(JobMsg::VoiceProgress(m, p));
            });
            let _ = tx.send(JobMsg::VoiceInstalled(res));
        });
    }

    /// Descarga motor y modelo Whisper (acción explícita del usuario).
    pub fn install_whisper(&mut self) {
        if self.whisper_busy.is_some() {
            return;
        }
        self.whisper_busy = Some(("Preparando descarga…".into(), 0.0));
        let tx = self.jobs_tx.clone();
        std::thread::spawn(move || {
            let tx2 = tx.clone();
            let res = services::whisper::install(&move |m, p| {
                let _ = tx2.send(JobMsg::WhisperProgress(m, p));
            });
            let _ = tx.send(JobMsg::WhisperInstalled(res));
        });
    }

    /// Transcribe el clip seleccionado (vídeo o audio) y crea los subtítulos.
    pub fn transcribe_selected(&mut self, clip_id: &str) {
        if self.whisper_busy.is_some() {
            return;
        }
        let Some(project) = self.editor.project() else { return };
        let Some((_, clip)) = project.find_clip(clip_id) else { return };
        let Some(m) = clip.media_id.as_deref().and_then(|mid| project.media.iter().find(|m| m.id == mid)) else {
            return self.toast("El clip seleccionado no tiene audio que transcribir");
        };
        let Some(path) = self.media_path(m) else {
            return self.toast_error("No se encuentra el archivo del medio");
        };
        let (in_point, dur, start, speed) = (clip.in_point, clip.duration, clip.start, clip.speed.max(0.01));
        let lang = self.whisper_lang.clone();
        let words = self.caption_karaoke;
        self.whisper_busy = Some(("Transcribiendo… (puede tardar)".into(), -1.0));
        let tx = self.jobs_tx.clone();
        std::thread::spawn(move || {
            let res = services::whisper::transcribe(&path, in_point, dur * speed, &lang, words).map(|mut items| {
                // Tiempo del medio → tiempo del clip (velocidad).
                for it in &mut items {
                    for k in ["start", "end"] {
                        if let Some(v) = it.get(k).and_then(Value::as_f64) {
                            it[k] = json!(v / speed);
                        }
                    }
                    if let Some(ws) = it.get_mut("words").and_then(Value::as_array_mut) {
                        for w in ws {
                            for k in ["s", "e"] {
                                if let Some(v) = w.get(k).and_then(Value::as_f64) {
                                    w[k] = json!(v / speed);
                                }
                            }
                        }
                    }
                }
                (items, start)
            });
            let _ = tx.send(JobMsg::Transcribed(res));
        });
    }

    pub fn start_recording(&mut self) {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let Some(device) = host.default_input_device() else {
            return self.toast_error("No se encontró ningún micrófono");
        };
        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => return self.toast_error(format!("Micrófono no disponible: {e}")),
        };
        let samples = std::sync::Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
        let rate = config.sample_rate().0;
        let channels = config.channels();
        let err_fn = |e| eprintln!("cupcraft: error de micrófono: {e}");
        let s2 = samples.clone();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    if let Ok(mut v) = s2.lock() {
                        v.extend_from_slice(data);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    if let Ok(mut v) = s2.lock() {
                        v.extend(data.iter().map(|s| *s as f32 / 32768.0));
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &_| {
                    if let Ok(mut v) = s2.lock() {
                        v.extend(data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0));
                    }
                },
                err_fn,
                None,
            ),
            other => return self.toast_error(format!("Formato de micrófono no compatible: {other:?}")),
        };
        let stream = match stream {
            Ok(s) => s,
            Err(e) => return self.toast_error(format!("No se pudo abrir el micrófono: {e}")),
        };
        if let Err(e) = stream.play() {
            return self.toast_error(format!("No se pudo iniciar la grabación: {e}"));
        }
        self.recorder = Some(Recorder { stream, samples, rate, channels, started: now_secs() });
        self.toast("Grabando voz…");
    }

    pub fn recording_secs(&self) -> f64 {
        self.recorder.as_ref().map(|r| now_secs() - r.started).unwrap_or(0.0)
    }

    pub fn stop_recording(&mut self, keep: bool) {
        let Some(rec) = self.recorder.take() else { return };
        drop(rec.stream);
        if !keep {
            return;
        }
        let data = rec.samples.lock().map(|v| v.clone()).unwrap_or_default();
        if data.is_empty() {
            return self.toast_error("La grabación está vacía");
        }
        let wav = encode_wav(&data, rec.rate, rec.channels);
        let pid = self.editor.project().map(|p| p.id.clone()).unwrap_or_default();
        let n = self.editor.project().map(|p| p.media.iter().filter(|m| m.name.as_deref().unwrap_or("").starts_with("Grabación")).count()).unwrap_or(0) + 1;
        match media::import_bytes(self.store.as_mut(), &format!("Grabación {n}.wav"), "wav", wav) {
            Ok(m) => self.on_media_imported(m, true, &pid),
            Err(e) => self.toast_error(e),
        }
    }

    // ------------------------------------------------------------------
    // Ajustes personalizados
    // ------------------------------------------------------------------

    pub fn save_custom_adjusts(&mut self) {
        let path = app_dir().join("custom_adjusts.json");
        let _ = std::fs::create_dir_all(app_dir());
        if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&self.custom_adjusts).unwrap_or_default()) {
            self.log(LogKind::Error, format!("No se pudieron guardar los ajustes: {e}"));
        }
    }

    /// Espacio usado por el almacenamiento local (base de datos + caché).
    pub fn storage_bytes(&self) -> u64 {
        dir_size(&app_dir())
    }
}

impl eframe::App for CupApp {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        // Archivos soltados en la ventana (sustituye a drag & drop del navegador).
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| p.is_absolute()).collect());
        if !dropped.is_empty() {
            // Un `.cupcraft` soltado es un proyecto, no un medio: ábrelo en vez
            // de intentar importarlo a la biblioteca.
            let (projects, media): (Vec<PathBuf>, Vec<PathBuf>) =
                dropped.into_iter().partition(|p| Self::is_project_path(p));
            if let Some(path) = projects.into_iter().next() {
                self.open_project_from_path(&path);
            }
            if !media.is_empty() {
                if self.screen == Screen::Home {
                    self.new_project_named("Proyecto sin título");
                }
                // Al soltar desde el explorador, el clip va directo a la
                // línea de tiempo (en el cabezal) además de a la biblioteca.
                self.import_paths(media, true);
            }
        }
        // Cierre de la ventana (botón, Alt+F4…): se intercepta para preguntar si se guarda.
        if !self.allow_close && ctx.input(|i| i.viewport().close_requested()) {
            if self.screen == Screen::Editor && self.editor.project().is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.request_quit(&ctx);
            }
        }
        match self.screen {
            Screen::Home => crate::home::show(ui, self),
            Screen::Editor => crate::editor::show(ui, self),
        }
        crate::dialogs::show(&ctx, self);
        show_toast(&ctx, self);
        if self.updater.ui(&ctx) {
            if self.screen == Screen::Editor {
                self.save_current();
            }
            services::updater::relaunch();
        }
    }

    fn on_exit(&mut self) {
        if self.screen == Screen::Editor && !self.skip_save_on_exit {
            self.save_current();
        }
        if let Some(h) = &self.export_handle {
            h.cancel();
        }
    }
}

// ----------------------------------------------------------------------
// Utilidades
// ----------------------------------------------------------------------

pub fn app_dir() -> PathBuf {
    let base = if let Ok(p) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(p)
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support")).unwrap_or_else(|_| std::env::temp_dir())
    } else if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(x)
    } else {
        std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("share")).unwrap_or_else(|_| std::env::temp_dir())
    };
    base.join("openCUP")
}

fn store_path() -> PathBuf {
    app_dir().join("store.sqlite")
}

fn load_custom_adjusts() -> Vec<Value> {
    std::fs::read_to_string(app_dir().join("custom_adjusts.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
        .unwrap_or_default()
}

fn dir_size(p: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(p) else { return 0 };
    rd.flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() { "cupcraft".into() } else { s }
}

pub fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn encode_wav(data: &[f32], rate: u32, channels: u16) -> Vec<u8> {
    let bytes_len = (data.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + bytes_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + bytes_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&bytes_len.to_le_bytes());
    for s in data {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

/// Importa un borrador de CapCut: copia los medios al almacén local y
/// reconstruye pistas y clips con los ids nuevos.
fn import_capcut(
    id: &str,
    allowed: &cupcraft_capcut::AllowedMedia,
    store_path: &Path,
    tx: &Sender<JobMsg>,
) -> Result<(Project, usize, Vec<String>), String> {
    let draft = cupcraft_capcut::read_draft(id, allowed)?;
    let name = draft.get("name").and_then(|v| v.as_str()).unwrap_or("Proyecto CapCut").to_string();
    let mut project = Project::new(name);
    project.ratio = draft.get("ratio").and_then(|v| v.as_str()).unwrap_or("16:9").to_string();
    let w = draft.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let h = draft.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;
    project.resolution = format!("{}p", w.min(h));
    project.fps = draft.get("fps").and_then(|v| v.as_f64()).unwrap_or(30.0).round().max(1.0) as u32;
    let mut warnings: Vec<String> = draft
        .get("warnings")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|w| w.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut store = Store::open(store_path).ok();
    let mut map: HashMap<String, String> = HashMap::new();
    let medias = draft.get("media").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = medias.len().max(1);
    let mut count = 0;
    for (i, m) in medias.iter().enumerate() {
        let path = m.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let cid = m.get("capcutId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mname = m.get("name").and_then(|v| v.as_str()).unwrap_or(path);
        let _ = tx.send(JobMsg::CapCutProgress(format!("Copiando {} ({}/{})", mname, i + 1, total), i as f32 / total as f32));
        if path.is_empty() || m.get("missing").and_then(|v| v.as_bool()).unwrap_or(false) {
            warnings.push(format!("Falta el archivo de “{mname}”"));
            continue;
        }
        match media::import_file(store.as_mut(), Path::new(path)) {
            Ok(med) => {
                map.insert(cid, med.id.clone());
                project.media.push(med);
                count += 1;
            }
            Err(e) => warnings.push(format!("{mname}: {e}")),
        }
    }
    let _ = tx.send(JobMsg::CapCutProgress("Reconstruyendo la línea de tiempo…".into(), 0.97));
    let tracks = draft.get("tracks").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut built = Vec::new();
    for t in tracks {
        let mut t = t.clone();
        if let Some(clips) = t.get_mut("clips").and_then(|v| v.as_array_mut()) {
            clips.retain_mut(|c| {
                if let Some(mid) = c.get("mediaId").and_then(|v| v.as_str()).map(String::from) {
                    match map.get(&mid) {
                        Some(new_id) => c["mediaId"] = json!(new_id),
                        None => return false,
                    }
                } else if c.get("mediaId").is_some() {
                    if let Some(o) = c.as_object_mut() {
                        o.remove("mediaId");
                    }
                }
                true
            });
        }
        match serde_json::from_value::<cupcraft_core::Track>(t) {
            Ok(track) => built.push(track),
            Err(e) => warnings.push(format!("Pista omitida: {e}")),
        }
    }
    if !built.is_empty() {
        project.tracks = built;
    }
    warnings.truncate(40);
    Ok((project, count, warnings))
}

pub fn show_toast(ctx: &egui::Context, app: &CupApp) {
    if let Some((msg, _)) = &app.toast {
        let msg = msg.clone();
        egui::Area::new(egui::Id::new("cupcraft-toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -24.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
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
