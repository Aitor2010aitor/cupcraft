//! Compositor del visor en segundo plano. Durante transiciones y filtros GL el
//! visor necesita la tubería completa del export; hacerlo en el hilo de la
//! interfaz congelaba la app (abrir ffmpeg, componer 540p, subir textura).
//! Aquí un hilo propio compone siempre el último instante pedido (descarta los
//! intermedios) y la interfaz solo sube el resultado cuando llega.

use super::compose::Compositor;
use cupcraft_core::Project;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Medios que necesita el compositor (se resuelven en el hilo de la interfaz).
pub type MediaMap = (HashMap<String, PathBuf>, HashMap<String, (u32, u32)>);

struct Req {
    project: Project,
    t: f64,
    w: u32,
    h: u32,
    /// Cambia cuando cambian los medios o el formato: hay que rehacer el compositor.
    media_key: (usize, String),
    media: Option<MediaMap>,
}

struct Resp {
    t: f64,
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

pub struct PreviewComp {
    tx: Sender<Req>,
    rx: Receiver<Resp>,
    tex: Option<egui::TextureHandle>,
    /// Instante del fotograma que hay en la textura.
    tex_t: Option<f64>,
    /// Última petición enviada (instante, tamaño y proyecto) para no repetirla.
    sent: Option<(f64, u32, Project)>,
    media_key: Option<(usize, String)>,
}

impl PreviewComp {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, req_rx) = channel::<Req>();
        let (resp_tx, rx) = channel::<Resp>();
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new().name("cupcraft-preview-comp".into()).spawn(move || {
            let mut comp: Option<(Compositor, (usize, String), u32, u32)> = None;
            let mut media: Option<MediaMap> = None;
            while let Ok(mut req) = req_rx.recv() {
                // Solo interesa la petición más reciente.
                while let Ok(r) = req_rx.try_recv() {
                    let m = req.media.take();
                    req = r;
                    if req.media.is_none() {
                        req.media = m;
                    }
                }
                if let Some(m) = req.media.take() {
                    media = Some(m);
                    comp = None;
                }
                let fresh = matches!(&comp, Some((_, k, w, h)) if *k == req.media_key && *w == req.w && *h == req.h);
                if !fresh {
                    let (paths, dims) = media.clone().unwrap_or_default();
                    comp = Some((Compositor::new(req.w, req.h, paths, dims), req.media_key.clone(), req.w, req.h));
                }
                let Some((c, ..)) = comp.as_mut() else { continue };
                let rgba = c.frame(&req.project, req.t);
                if resp_tx.send(Resp { t: req.t, w: req.w, h: req.h, rgba }).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { tx, rx, tex: None, tex_t: None, sent: None, media_key: None }
    }

    /// Pide el fotograma `t` de `project` a `h` px de alto y devuelve la textura
    /// más reciente si corresponde a un instante cercano (si no, `None` y el
    /// visor pinta las capas por su cuenta mientras llega).
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        project: &Project,
        t: f64,
        h: u32,
        resolve_media: impl FnOnce() -> MediaMap,
    ) -> Option<egui::TextureId> {
        // Recoge lo que haya terminado el hilo (nos quedamos con lo último).
        let mut last = None;
        while let Ok(r) = self.rx.try_recv() {
            last = Some(r);
        }
        if let Some(r) = last {
            let img = egui::ColorImage::from_rgba_unmultiplied([r.w as usize, r.h as usize], &r.rgba);
            match &mut self.tex {
                Some(tex) if tex.size() == [r.w as usize, r.h as usize] => tex.set(img, egui::TextureOptions::LINEAR),
                _ => self.tex = Some(ctx.load_texture("preview-comp", img, egui::TextureOptions::LINEAR)),
            }
            self.tex_t = Some(r.t);
        }
        let need = !matches!(&self.sent, Some((st, sh, sp)) if *st == t && *sh == h && sp == project);
        if need {
            let key = (project.media.len(), project.ratio.clone());
            let media = if self.media_key.as_ref() != Some(&key) {
                self.media_key = Some(key.clone());
                Some(resolve_media())
            } else {
                None
            };
            let (w, h2) = super::compose::canvas_size(&project.ratio, h);
            let _ = self.tx.send(Req { project: project.clone(), t, w, h: h2, media_key: key, media });
            self.sent = Some((t, h, project.clone()));
        }
        // Un fotograma de hasta ~0,25 s de retraso es aceptable; más viejo no.
        match (self.tex_t, &self.tex) {
            (Some(tt), Some(tex)) if (tt - t).abs() <= 0.25 => Some(tex.id()),
            _ => None,
        }
    }
}
