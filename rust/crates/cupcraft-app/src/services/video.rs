//! Decodificación de fotogramas de vídeo e imágenes para la vista previa.
//!
//! Sustituye a los elementos `<video>`/`<img>` del `MediaPool` web: un hilo de
//! fondo mantiene un proceso FFmpeg por medio que emite fotogramas RGBA
//! (`-f rawvideo`). Durante la reproducción se leen secuencialmente; al saltar
//! (scrub) el proceso se reinicia en el nuevo instante.

use crate::paint::{apply_fx, Fx};
use eframe::egui;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub const STREAM_FPS: f64 = 30.0;

/// Proceso FFmpeg que entrega fotogramas RGBA consecutivos desde `start`.
pub struct FrameStream {
    child: Child,
    stdout: ChildStdout,
    pub w: u32,
    pub h: u32,
    pub fps: f64,
    /// Instante (en el medio) del siguiente fotograma que se leerá.
    pub next_t: f64,
    pub ended: bool,
    pub path: PathBuf,
    /// Último fotograma leído: si se vuelve a pedir el mismo instante (deslizadores
    /// con el vídeo quieto) se reutiliza en vez de leer más o relanzar ffmpeg.
    last: Option<Vec<u8>>,
}

impl FrameStream {
    pub fn open(path: &Path, start: f64, w: u32, h: u32, fps: f64) -> Option<Self> {
        let loc = cupcraft_media::FfmpegLocation::find()?;
        let vf = format!("fps={fps},scale={w}:{h}:flags=bilinear");
        let mut child = cupcraft_media::ffmpeg::command(&loc.ffmpeg_path)
            .args(["-v", "error", "-nostdin", "-ss", &format!("{:.3}", start.max(0.0)), "-i"])
            .arg(path)
            .args(["-an", "-sn", "-vf", &vf, "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        Some(Self { child, stdout, w, h, fps, next_t: start.max(0.0), ended: false, path: path.to_path_buf(), last: None })
    }

    /// Lee el siguiente fotograma. `None` al llegar al final del medio.
    pub fn read_frame(&mut self) -> Option<Vec<u8>> {
        if self.ended {
            return None;
        }
        let mut buf = vec![0u8; (self.w * self.h * 4) as usize];
        match self.stdout.read_exact(&mut buf) {
            Ok(()) => {
                self.next_t += 1.0 / self.fps;
                Some(buf)
            }
            Err(_) => {
                self.ended = true;
                None
            }
        }
    }

    /// Avanza hasta el fotograma que cubre `t` y lo devuelve.
    pub fn frame_at(&mut self, t: f64) -> Option<Vec<u8>> {
        // El fotograma ya leído cubre [next_t - 1/fps, next_t).
        if let Some(l) = &self.last {
            if t + 1e-6 < self.next_t && t >= self.next_t - 1.5 / self.fps {
                return Some(l.clone());
            }
        }
        let mut last = None;
        loop {
            match self.read_frame() {
                Some(f) => {
                    last = Some(f);
                    if self.next_t > t + 1e-6 {
                        break;
                    }
                }
                None => break,
            }
        }
        if last.is_some() {
            self.last = last.clone();
        }
        last
    }
}

impl Drop for FrameStream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Tamaño de decodificación que respeta la proporción del medio dentro de
/// una caja `max_w × max_h` (dimensiones pares, requisito de FFmpeg).
pub fn fit_dims(src: Option<(u32, u32)>, max_w: u32, max_h: u32) -> (u32, u32) {
    let (sw, sh) = src.unwrap_or((16, 9));
    let s = (max_w as f64 / sw as f64).min(max_h as f64 / sh as f64).min(1.0);
    let w = ((sw as f64 * s).round() as u32).max(2) & !1;
    let h = ((sh as f64 * s).round() as u32).max(2) & !1;
    (w.min(max_w.max(2)), h.min(max_h.max(2)))
}

#[derive(Clone)]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgba: Arc<Vec<u8>>,
    pub generation: u64,
}

enum Req {
    Video { key: String, path: PathBuf, t: f64, w: u32, h: u32, playing: bool },
    Image { key: String, path: PathBuf },
}

/// Hash estable de una cadena de color para invalidar la textura procesada en
/// caché cuando el usuario cambia un filtro, efecto o ajuste del clip.
fn chain_hash(chain: &[Fx]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    chain.len().hash(&mut h);
    for fx in chain {
        for v in [fx.brightness, fx.contrast, fx.saturate, fx.grayscale, fx.sepia, fx.hue, fx.invert, fx.opacity, fx.blur] {
            v.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

impl Req {
    fn key(&self) -> &str {
        match self {
            Req::Video { key, .. } | Req::Image { key, .. } => key,
        }
    }
}

type Frames = Arc<Mutex<HashMap<String, Frame>>>;

/// Servicio de fotogramas de la vista previa (vive en `CupApp`).
pub struct VideoService {
    tx: Sender<Req>,
    frames: Frames,
    textures: HashMap<String, (egui::TextureHandle, u64)>,
    // Texturas con la cadena de color aplicada por píxel (paridad con el
    // compositor de exportación). Clave → (textura, generación del fotograma,
    // hash del estado de color). Se recalcula solo si cambia el fotograma o el
    // color/filtro/efecto/ajuste del clip.
    fx_textures: HashMap<String, (egui::TextureHandle, u64, u64)>,
    requested_images: HashMap<String, ()>,
    last_req: HashMap<String, (i64, u32, u32)>,
}

impl VideoService {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = channel();
        let frames: Frames = Arc::new(Mutex::new(HashMap::new()));
        let f2 = frames.clone();
        std::thread::Builder::new()
            .name("cupcraft-video".into())
            .spawn(move || worker(rx, f2, ctx))
            .ok();
        Self {
            tx,
            frames,
            textures: HashMap::new(),
            fx_textures: HashMap::new(),
            requested_images: HashMap::new(),
            last_req: HashMap::new(),
        }
    }

    /// Pide el fotograma `t` (segundos dentro del medio) de un vídeo.
    pub fn request_video(&mut self, key: &str, path: &Path, t: f64, w: u32, h: u32, playing: bool) {
        let q = (t * STREAM_FPS).floor() as i64;
        if self.last_req.get(key) == Some(&(q, w, h)) {
            return;
        }
        self.last_req.insert(key.to_string(), (q, w, h));
        let _ = self.tx.send(Req::Video { key: key.to_string(), path: path.to_path_buf(), t, w, h, playing });
    }

    pub fn request_image(&mut self, key: &str, path: &Path) {
        if self.requested_images.contains_key(key) {
            return;
        }
        self.requested_images.insert(key.to_string(), ());
        let _ = self.tx.send(Req::Image { key: key.to_string(), path: path.to_path_buf() });
    }

    /// Publica un fotograma ya decodificado (GIF animado) bajo `key`; la
    /// textura se renueva solo cuando cambia `generation`.
    pub fn push_frame(&mut self, key: &str, w: u32, h: u32, rgba: Arc<Vec<u8>>, generation: u64) {
        if let Ok(mut frames) = self.frames.lock() {
            if frames.get(key).map(|f| f.generation) != Some(generation) {
                frames.insert(key.to_string(), Frame { w, h, rgba, generation });
            }
        }
    }

    /// Textura actual del medio (se actualiza cuando llega un fotograma nuevo).
    pub fn texture(&mut self, ctx: &egui::Context, key: &str) -> Option<(egui::TextureId, egui::Vec2)> {
        let frame = self.frames.lock().ok()?.get(key).cloned()?;
        let size = [frame.w as usize, frame.h as usize];
        match self.textures.get_mut(key) {
            Some((tex, gen)) if *gen == frame.generation => {}
            Some((tex, gen)) => {
                tex.set(egui::ColorImage::from_rgba_unmultiplied(size, &frame.rgba), egui::TextureOptions::LINEAR);
                *gen = frame.generation;
            }
            None => {
                let tex = ctx.load_texture(
                    format!("media-{key}"),
                    egui::ColorImage::from_rgba_unmultiplied(size, &frame.rgba),
                    egui::TextureOptions::LINEAR,
                );
                self.textures.insert(key.to_string(), (tex, frame.generation));
            }
        }
        let (tex, _) = self.textures.get(key)?;
        Some((tex.id(), egui::vec2(frame.w as f32, frame.h as f32)))
    }

    /// Textura del medio con la cadena de color (`chain`) aplicada **por píxel**,
    /// exactamente igual que el compositor de exportación (`apply_fx`). Así la
    /// previsualización refleja saturación, contraste, gris, sepia, tono, etc.
    /// en tiempo real, no como un tinte sólido. Reutiliza la textura en caché
    /// mientras no cambien ni el fotograma ni el estado de color del clip, y
    /// reparte el bucle por píxel entre núcleos con `rayon`.
    pub fn texture_fx(&mut self, ctx: &egui::Context, key: &str, chain: &[Fx], warp: Option<crate::paint::Warp>, ai: Option<super::ai_local::AiCtx>) -> Option<(egui::TextureId, egui::Vec2)> {
        if chain.is_empty() && warp.is_none() && ai.is_none() {
            return self.texture(ctx, key);
        }
        let frame = self.frames.lock().ok()?.get(key).cloned()?;
        let size = [frame.w as usize, frame.h as usize];
        // La deformación geométrica se pliega en la clave de caché igual que el
        // color, y se aplica a la fuente antes de la cadena de color.
        let hash = chain_hash(chain) ^ warp.map(|w| w.hash()).unwrap_or(0) ^ ai.as_ref().map(|a| a.hash()).unwrap_or(0);
        let fresh = matches!(self.fx_textures.get(key), Some((_, gen, h)) if *gen == frame.generation && *h == hash);
        if !fresh {
            // Deformación geométrica real (si la hay) antes del color.
            let mut warped: Option<Vec<u8>> = warp.map(|wp| {
                crate::paint::warp_rgba(&frame.rgba, frame.w as usize, frame.h as usize, &wp)
            });
            if let Some(a) = &ai {
                let mut v = warped.take().unwrap_or_else(|| frame.rgba.to_vec());
                super::ai_local::process(a, &mut v, frame.w as usize, frame.h as usize);
                warped = Some(v);
            }
            let base: &[u8] = warped.as_deref().unwrap_or(&frame.rgba);
            let mut out = vec![0u8; frame.rgba.len()];
            out.par_chunks_mut(4).zip(base.par_chunks(4)).for_each(|(o, px)| {
                let c = egui::Color32::from_rgba_unmultiplied(px[0], px[1], px[2], px[3]);
                let r = chain.iter().fold(c, |c, fx| apply_fx(c, fx));
                o.copy_from_slice(&r.to_array());
            });
            let img = egui::ColorImage::from_rgba_unmultiplied(size, &out);
            match self.fx_textures.get_mut(key) {
                Some((tex, gen, h)) => {
                    tex.set(img, egui::TextureOptions::LINEAR);
                    *gen = frame.generation;
                    *h = hash;
                }
                None => {
                    let tex = ctx.load_texture(format!("media-fx-{key}"), img, egui::TextureOptions::LINEAR);
                    self.fx_textures.insert(key.to_string(), (tex, frame.generation, hash));
                }
            }
        }
        let (tex, ..) = self.fx_textures.get(key)?;
        Some((tex.id(), egui::vec2(frame.w as f32, frame.h as f32)))
    }

    /// Último fotograma decodificado (para miniaturas / captura).
    pub fn frame(&self, key: &str) -> Option<Frame> {
        self.frames.lock().ok()?.get(key).cloned()
    }
}

fn worker(rx: Receiver<Req>, frames: Frames, ctx: egui::Context) {
    let mut streams: HashMap<String, FrameStream> = HashMap::new();
    let mut generation = 0u64;
    while let Ok(first) = rx.recv() {
        // Nos quedamos solo con la petición más reciente de cada medio.
        let mut pending: Vec<Req> = vec![first];
        while let Ok(r) = rx.try_recv() {
            pending.retain(|p| p.key() != r.key());
            pending.push(r);
        }
        for req in pending {
            match req {
                Req::Image { key, path } => {
                    if let Ok(img) = image::open(&path) {
                        let img = if img.width() > 1920 || img.height() > 1920 {
                            img.thumbnail(1920, 1920)
                        } else {
                            img
                        };
                        let rgba = img.to_rgba8();
                        generation += 1;
                        if let Ok(mut map) = frames.lock() {
                            map.insert(
                                key,
                                Frame { w: rgba.width(), h: rgba.height(), rgba: Arc::new(rgba.into_raw()), generation },
                            );
                        }
                        ctx.request_repaint();
                    }
                }
                Req::Video { key, path, t, w, h, playing } => {
                    let reuse = streams.get(&key).map(|s| {
                        s.w == w
                            && s.h == h
                            && s.path == path
                            && !s.ended
                            && t + 0.5 / s.fps >= s.next_t - 1.0 / s.fps
                            && t <= s.next_t + if playing { 1.5 } else { 0.25 }
                    });
                    if reuse != Some(true) {
                        streams.remove(&key);
                        if let Some(s) = FrameStream::open(&path, t, w, h, STREAM_FPS) {
                            streams.insert(key.clone(), s);
                        }
                    }
                    let Some(stream) = streams.get_mut(&key) else { continue };
                    if let Some(buf) = stream.frame_at(t) {
                        generation += 1;
                        if let Ok(mut map) = frames.lock() {
                            map.insert(key.clone(), Frame { w, h, rgba: Arc::new(buf), generation });
                        }
                        ctx.request_repaint();
                    }
                    if streams.len() > 6 {
                        let other: Vec<String> = streams.keys().filter(|k| **k != key).take(1).cloned().collect();
                        for k in other {
                            streams.remove(&k);
                        }
                    }
                }
            }
        }
    }
}
