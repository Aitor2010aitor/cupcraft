//! Matting IA (recorte de fondo) — port nativo de `src/filtros/bg-matting.js`.
//!
//! Ejecuta el modelo **BackgroundMattingV2** (ONNX, MIT, PeterL1n) con `tract`
//! (Rust puro: sin biblioteca nativa de ONNX Runtime ni privilegios de
//! administrador, un solo `.exe` autocontenido) para estimar el alpha del
//! sujeto a partir del fotograma y una placa de fondo (imagen o color sólido).
//!
//! El postproceso —limpieza de borde (`cleanAlpha`) y estabilidad temporal
//! (`blendTemporal`)— replica 1:1 el de la web. El modelo requiere las mismas
//! entradas que la versión web: `src` y `bgr`, ambas `[1, 3, 288, 512]`, y
//! devuelve el alpha `pha`. Si el archivo del modelo no está presente el
//! matting queda desactivado y el clip se dibuja tal cual (sin recorte).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tract_onnx::prelude::*;

/// Dimensiones de la red (idénticas a `bg-matting.js`: el modelo HD exportado
/// necesita un lado >= ~283 px).
const NET_W: usize = 512;
const NET_H: usize = 288;

const TEMPORAL_HISTORY: usize = 10;
const TEMPORAL_MAX_DELTA: f64 = 0.22; // segundos

type Plan = Arc<TypedRunnableModel>;

pub struct Matte {
    plan: Plan,
    n_in: usize,
    src_idx: usize,
    bgr_idx: usize,
    pha_out: usize,
    temporal: HashMap<String, Vec<(f64, Vec<f32>)>>,
}

/// Rutas candidatas del modelo, en orden: junto al ejecutable (release),
/// variable de entorno y árbol de desarrollo. Se devuelve la primera existente.
pub fn find_model() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("CUPCRAFT_MATTE_MODEL") {
        candidates.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("models").join("bgm_hd.onnx"));
            candidates.push(dir.join("bgm_hd.onnx"));
        }
    }
    // Árbol de desarrollo: <repo>/models/bgm_hd.onnx.
    candidates.push(PathBuf::from("models/bgm_hd.onnx"));
    candidates.push(PathBuf::from("../models/bgm_hd.onnx"));
    candidates.into_iter().find(|p| p.exists())
}

impl Matte {
    /// Carga y optimiza el modelo. Devuelve `None` si el archivo no existe o si
    /// el modelo no puede construirse (el matting queda entonces desactivado).
    pub fn load(path: &Path) -> Option<Matte> {
        match Self::build(path) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("cupcraft: matting IA no disponible: {e}");
                None
            }
        }
    }

    fn build(path: &Path) -> TractResult<Matte> {
        let mut model = tract_onnx::onnx().model_for_path(path)?;
        // Ambas entradas comparten forma [1, 3, 288, 512].
        let fact = InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 3, NET_H, NET_W));
        let n_in = model.inputs.len();
        for i in 0..n_in {
            model.set_input_fact(i, fact.clone())?;
        }
        let typed = model.into_optimized()?;
        let in_names: Vec<String> =
            typed.inputs.iter().map(|o| typed.node(o.node).name.to_lowercase()).collect();
        let out_names: Vec<String> =
            typed.outputs.iter().map(|o| typed.node(o.node).name.to_lowercase()).collect();
        let src_idx = in_names
            .iter()
            .position(|n| n.contains("src") || n.contains("fore"))
            .unwrap_or(0);
        let bgr_idx = in_names
            .iter()
            .position(|n| n.contains("bgr") || n.contains("back") || n == "bg")
            .unwrap_or(if src_idx == 0 { 1.min(n_in.saturating_sub(1)) } else { 0 });
        let pha_out = out_names
            .iter()
            .position(|n| n.contains("pha") || n.contains("alpha"))
            .unwrap_or(0);
        let plan = typed.into_runnable()?;
        Ok(Matte { plan, n_in: n_in.max(2), src_idx, bgr_idx, pha_out, temporal: HashMap::new() })
    }

    /// Calcula el alpha (`NET_W`×`NET_H`, valores 0..1) del sujeto. `src`/`bg`
    /// son RGBA sin premultiplicar a sus propias dimensiones. Devuelve `None`
    /// si la inferencia falla (el clip se dibuja entonces sin recorte).
    #[allow(clippy::too_many_arguments)]
    pub fn alpha(
        &mut self,
        src: &[u8],
        sw: usize,
        sh: usize,
        bg: &[u8],
        bw: usize,
        bh: usize,
        edge: f32,
        temporal: f32,
        key: &str,
        time: f64,
    ) -> Option<Vec<f32>> {
        let src_chw = to_net_chw(src, sw, sh);
        let bg_chw = to_net_chw(bg, bw, bh);
        let raw = self.run(&src_chw, &bg_chw).ok()?;
        let cleaned = clean_alpha(&raw, edge.clamp(0.0, 1.0));
        Some(self.blend_temporal(cleaned, key, time, temporal.clamp(0.0, 1.0)))
    }

    fn run(&self, src_chw: &[f32], bg_chw: &[f32]) -> TractResult<Vec<f32>> {
        let shape = [1, 3, NET_H, NET_W];
        let src_t: Tensor = Tensor::from_shape(&shape, src_chw)?;
        let bgr_t: Tensor = Tensor::from_shape(&shape, bg_chw)?;
        let mut inputs: TVec<TValue> = tvec!();
        for i in 0..self.n_in {
            if i == self.src_idx {
                inputs.push(src_t.clone().into());
            } else if i == self.bgr_idx {
                inputs.push(bgr_t.clone().into());
            } else {
                // Entrada inesperada (el modelo de la web solo usa src/bgr).
                inputs.push(src_t.clone().into());
            }
        }
        let out = self.plan.run(inputs)?;
        let view = out[self.pha_out].to_plain_array_view::<f32>()?;
        let mut alpha = vec![0.0f32; NET_W * NET_H];
        for (dst, v) in alpha.iter_mut().zip(view.iter()) {
            *dst = *v;
        }
        Ok(alpha)
    }

    /// `blendTemporal` de la web: mezcla con el fotograma vecino más cercano
    /// (±0,22 s) para estabilizar el recorte. Guarda el alpha recién inferido,
    /// no el mezclado, para no depender del orden de precálculo.
    fn blend_temporal(&mut self, alpha: Vec<f32>, key: &str, time: f64, strength: f32) -> Vec<f32> {
        if key.is_empty() || strength <= 0.001 || !time.is_finite() {
            return alpha;
        }
        let frames = self.temporal.entry(key.to_string()).or_default();
        let mut best = f64::INFINITY;
        let mut nearest: Option<&Vec<f32>> = None;
        for (t, a) in frames.iter() {
            let delta = (t - time).abs();
            if delta < best {
                best = delta;
                nearest = Some(a);
            }
        }
        let mut out = alpha.clone();
        if let Some(near) = nearest {
            if best <= TEMPORAL_MAX_DELTA {
                let weight = strength * (1.0 - (best / TEMPORAL_MAX_DELTA) as f32) * 0.45;
                if weight > 0.001 && near.len() == out.len() {
                    for (o, n) in out.iter_mut().zip(near.iter()) {
                        *o = *o * (1.0 - weight) + *n * weight;
                    }
                }
            }
        }
        frames.push((time, alpha));
        if frames.len() > TEMPORAL_HISTORY {
            frames.remove(0);
        }
        out
    }
}

/// Reescala el RGBA de origen a `NET_W`×`NET_H` (bilineal, como `drawImage`) y
/// lo convierte a CHW normalizado [0,1], solo canales RGB.
fn to_net_chw(rgba: &[u8], w: usize, h: usize) -> Vec<f32> {
    let plane = NET_W * NET_H;
    let mut out = vec![0.0f32; 3 * plane];
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return out;
    }
    for y in 0..NET_H {
        // Centro del texel de destino en coordenadas de origen.
        let fy = ((y as f32 + 0.5) * h as f32 / NET_H as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let y0 = fy.floor() as usize;
        let y1 = (y0 + 1).min(h - 1);
        let wy = fy - y0 as f32;
        for x in 0..NET_W {
            let fx = ((x as f32 + 0.5) * w as f32 / NET_W as f32 - 0.5).clamp(0.0, (w - 1) as f32);
            let x0 = fx.floor() as usize;
            let x1 = (x0 + 1).min(w - 1);
            let wx = fx - x0 as f32;
            let p = y * NET_W + x;
            for c in 0..3 {
                let s00 = rgba[(y0 * w + x0) * 4 + c] as f32;
                let s10 = rgba[(y0 * w + x1) * 4 + c] as f32;
                let s01 = rgba[(y1 * w + x0) * 4 + c] as f32;
                let s11 = rgba[(y1 * w + x1) * 4 + c] as f32;
                let top = s00 + (s10 - s00) * wx;
                let bot = s01 + (s11 - s01) * wx;
                out[c * plane + p] = (top + (bot - top) * wy) / 255.0;
            }
        }
    }
    out
}

/// `cleanAlpha` de la web: limpieza de borde con dos pasadas separables
/// (horizontal + vertical) y una curva suave (`smoothstep`), inspirada en el
/// pase DilateErode. `strength` ∈ [0,1].
fn clean_alpha(alpha: &[f32], strength: f32) -> Vec<f32> {
    if strength <= 0.001 {
        return alpha.to_vec();
    }
    let (w, h) = (NET_W, NET_H);
    let spread = (strength * 0.5).min(0.5);
    let contrast = strength * 0.35;
    let mut horizontal = vec![0.0f32; alpha.len()];
    let mut out = vec![0.0f32; alpha.len()];
    for y in 0..h {
        let row = y * w;
        for x in 0..w {
            let i = row + x;
            let left = alpha[row + if x > 0 { x - 1 } else { 0 }];
            let right = alpha[row + if x < w - 1 { x + 1 } else { w - 1 }];
            horizontal[i] = alpha[i] * (1.0 - spread) + (left + right) * 0.5 * spread;
        }
    }
    for y in 0..h {
        let row = y * w;
        for x in 0..w {
            let i = row + x;
            let up = horizontal[if y > 0 { y - 1 } else { 0 } * w + x];
            let down = horizontal[if y < h - 1 { y + 1 } else { h - 1 } * w + x];
            let smooth = horizontal[i] * (1.0 - spread) + (up + down) * 0.5 * spread;
            let shaped = smooth * smooth * (3.0 - 2.0 * smooth);
            out[i] = (smooth * (1.0 - contrast) + shaped * contrast).clamp(0.0, 1.0);
        }
    }
    out
}

/// Muestrea el alpha de la red (`NET`) en coordenada de lienzo completo, como
/// hace la web al escribir el canal alpha del canvas de salida.
pub fn sample(alpha: &[f32], x: usize, y: usize, w: usize, h: usize) -> f32 {
    let sx = ((x * NET_W) / w.max(1)).min(NET_W - 1);
    let sy = ((y * NET_H) / h.max(1)).min(NET_H - 1);
    alpha[sy * NET_W + sx]
}
