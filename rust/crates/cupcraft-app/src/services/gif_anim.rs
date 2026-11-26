//! GIF animados: se decodifican una sola vez (todos los fotogramas) y se
//! reproducen en bucle según el tiempo del clip, tanto en la vista previa como
//! en la exportación. Un GIF de un solo fotograma devuelve `None` y sigue el
//! camino normal de imagen estática.

use image::AnimationDecoder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const MAX_FRAMES: usize = 1200;

pub struct GifAnim {
    pub w: u32,
    pub h: u32,
    frames: Vec<Arc<Vec<u8>>>,
    /// Instante de fin de cada fotograma (segundos, acumulado).
    ends: Vec<f64>,
    pub total: f64,
}

impl GifAnim {
    /// Índice y píxeles RGBA del fotograma visible en `t` (con bucle).
    pub fn frame_at(&self, t: f64) -> (usize, Arc<Vec<u8>>) {
        let t = if self.total > 0.0 { t.max(0.0) % self.total } else { 0.0 };
        let idx = self.ends.iter().position(|&e| t < e).unwrap_or(self.frames.len() - 1);
        (idx, self.frames[idx].clone())
    }
}

fn is_gif(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("gif")).unwrap_or(false)
}

fn decode(path: &Path) -> Option<GifAnim> {
    let file = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    let decoder = image::codecs::gif::GifDecoder::new(file).ok()?;
    let mut frames = Vec::new();
    let mut ends = Vec::new();
    let mut acc = 0.0;
    let (mut w, mut h) = (0, 0);
    for frame in decoder.into_frames().take(MAX_FRAMES) {
        let frame = frame.ok()?;
        let (num, den) = frame.delay().numer_denom_ms();
        // Los GIF con retardo 0 se muestran a ~10 fps, como los navegadores.
        let mut delay = if den == 0 { 0.1 } else { num as f64 / den as f64 / 1000.0 };
        if delay < 0.02 {
            delay = 0.1;
        }
        acc += delay;
        let buf = frame.into_buffer();
        w = buf.width();
        h = buf.height();
        frames.push(Arc::new(buf.into_raw()));
        ends.push(acc);
    }
    if frames.len() < 2 {
        return None;
    }
    Some(GifAnim { w, h, frames, ends, total: acc })
}

/// Animación del GIF en `path`, con caché. `None` si no es un GIF animado.
pub fn load(path: &Path) -> Option<Arc<GifAnim>> {
    if !is_gif(path) {
        return None;
    }
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<GifAnim>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().ok()?.get(path) {
        return hit.clone();
    }
    let anim = decode(path).map(Arc::new);
    cache.lock().ok()?.insert(path.to_path_buf(), anim.clone());
    anim
}

#[cfg(test)]
mod tests {
    #[test]
    fn gif_real_de_capcut_es_animado() {
        let p = std::path::Path::new(r"C:\Users\AITOR\AppData\Local\CapCut\User Data\Cache\artistEffect\7599477172557843730\71a4427d6e11cd51634d829625e56ea9\final.gif");
        if !p.exists() { return; }
        let a = super::load(p).expect("debe ser animado");
        assert!(a.total > 0.0);
        assert_ne!(a.frame_at(0.0).0, a.frame_at(a.total * 0.5).0);
        println!("{} fotogramas aprox, {}s", a.frame_at(a.total).0, a.total);
    }
}
