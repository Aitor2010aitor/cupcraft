//! Exportación en un hilo de fondo: compone cada fotograma con `Compositor`,
//! lo codifica con FFmpeg (`VideoEncoderSession`), mezcla el audio del
//! proyecto con `AudioMixer` y lo multiplexa en el archivo final.
//! Sustituye a `<canvas>` + `MediaRecorder` + `gif-encoder.js` de la web.

use super::compose::Compositor;
use super::media;
use cupcraft_core::Project;
use cupcraft_media::{ExportFormat, VideoEncoderSession};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Trabajo de exportación preparado en el hilo de UI (rutas ya resueltas).
pub struct ExportJob {
    pub project: Project,
    pub out: PathBuf,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub format: ExportFormat,
    pub use_gpu: bool,
    pub paths: HashMap<String, PathBuf>,
    pub dims: HashMap<String, (u32, u32)>,
    /// Medios que aportan audio (id, ruta).
    pub audio: Vec<(String, PathBuf)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportStatus {
    Running(String),
    Done(PathBuf),
    Failed(String),
    Cancelled,
}

/// Manejador compartido con la UI.
#[derive(Clone)]
pub struct ExportHandle {
    /// Progreso 0‥10000.
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<ExportStatus>>,
}

impl ExportHandle {
    pub fn progress(&self) -> f32 {
        self.progress.load(Ordering::Relaxed) as f32 / 10000.0
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn status(&self) -> ExportStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or(ExportStatus::Failed("estado".into()))
    }
    pub fn is_running(&self) -> bool {
        matches!(self.status(), ExportStatus::Running(_))
    }
    fn set(&self, s: ExportStatus) {
        if let Ok(mut g) = self.status.lock() {
            *g = s;
        }
    }
}

pub fn start(job: ExportJob, ctx: eframe::egui::Context) -> ExportHandle {
    let handle = ExportHandle {
        progress: Arc::new(AtomicU32::new(0)),
        cancel: Arc::new(AtomicBool::new(false)),
        status: Arc::new(Mutex::new(ExportStatus::Running("Preparando…".into()))),
    };
    let h2 = handle.clone();
    std::thread::Builder::new()
        .name("cupcraft-export".into())
        .spawn(move || {
            let result = run(job, &h2, &ctx);
            match result {
                Ok(Some(p)) => h2.set(ExportStatus::Done(p)),
                Ok(None) => h2.set(ExportStatus::Cancelled),
                Err(e) => h2.set(ExportStatus::Failed(e)),
            }
            ctx.request_repaint();
        })
        .ok();
    handle
}

fn run(job: ExportJob, h: &ExportHandle, ctx: &eframe::egui::Context) -> Result<Option<PathBuf>, String> {
    if cupcraft_media::FfmpegLocation::find().is_none() {
        return Err("No se encontró FFmpeg. Instálalo (winget install Gyan.FFmpeg) o colócalo junto a cupcraft.exe.".into());
    }
    let duration = job.project.duration();
    if duration <= 0.0 {
        return Err("El proyecto está vacío: añade algún clip a la línea de tiempo.".into());
    }
    let total = (duration * job.fps as f64).ceil().max(1.0) as u64;
    let ext = job.out.extension().and_then(|e| e.to_str()).unwrap_or("mp4").to_lowercase();
    let tmp = media::work_dir().join(format!("export-{}-video.{ext}", std::process::id()));

    let mut enc = match VideoEncoderSession::start(&tmp, job.w, job.h, job.fps, total, job.format, job.use_gpu) {
        Ok(e) => e,
        Err(e) if job.use_gpu => VideoEncoderSession::start(&tmp, job.w, job.h, job.fps, total, job.format, false)
            .map_err(|e2| format!("No se pudo iniciar el codificador: {e} / {e2}"))?,
        Err(e) => return Err(format!("No se pudo iniciar el codificador: {e}")),
    };

    let mut comp = Compositor::new(job.w, job.h, job.paths.clone(), job.dims.clone());
    let video_share = if job.audio.is_empty() { 1.0 } else { 0.85 };
    for i in 0..total {
        if h.cancel.load(Ordering::Relaxed) {
            enc.abort();
            let _ = std::fs::remove_file(&tmp);
            return Ok(None);
        }
        let t = i as f64 / job.fps as f64;
        let frame = comp.frame(&job.project, t);
        if let Err(e) = enc.push_frame(&frame) {
            enc.abort();
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("Error al codificar el fotograma {i}: {e}"));
        }
        let p = (i + 1) as f32 / total as f32 * video_share;
        h.progress.store((p * 10000.0) as u32, Ordering::Relaxed);
        if i % 5 == 0 {
            h.set(ExportStatus::Running(format!("Fotograma {} de {}", i + 1, total)));
            ctx.request_repaint();
        }
    }
    drop(comp);
    enc.finish().map_err(|e| format!("FFmpeg terminó con error: {e}"))?;

    if job.audio.is_empty() {
        let _ = std::fs::remove_file(&job.out);
        std::fs::rename(&tmp, &job.out)
            .or_else(|_| std::fs::copy(&tmp, &job.out).map(|_| ()))
            .map_err(|e| format!("No se pudo escribir el archivo final: {e}"))?;
        let _ = std::fs::remove_file(&tmp);
    } else {
        h.set(ExportStatus::Running("Mezclando audio…".into()));
        ctx.request_repaint();
        let mut mixer = cupcraft_audio::AudioMixer::new();
        for (id, path) in &job.audio {
            if h.cancel.load(Ordering::Relaxed) {
                let _ = std::fs::remove_file(&tmp);
                return Ok(None);
            }
            if let Some(buf) = media::decode_audio(path, id) {
                mixer.insert_media(id.clone(), buf);
            }
        }
        h.progress.store(9200, Ordering::Relaxed);
        let mix = mixer.render_project_audio(&job.project, 48000);
        h.set(ExportStatus::Running("Multiplexando audio y vídeo…".into()));
        ctx.request_repaint();
        let res = cupcraft_media::mux_audio_video(tmp.as_path(), &mix, job.out.as_path());
        let _ = std::fs::remove_file(&tmp);
        res.map_err(|e| format!("No se pudo añadir el audio: {e}"))?;
    }
    h.progress.store(10000, Ordering::Relaxed);
    Ok(Some(job.out))
}
