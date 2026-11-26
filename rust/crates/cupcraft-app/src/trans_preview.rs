//! Vista previa animada de una transición al pasar el cursor por su tarjeta.
//! Usa el compositor real (el mismo que el visor y la exportación) sobre un
//! mini proyecto con dos imágenes de muestra: A (clip anterior) → B.

use crate::services::compose::Compositor;
use cupcraft_core::{Clip, Project};
use eframe::egui;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::cell::RefCell;

const W: u32 = 192;
const H: u32 = 108;
/// Inicio del clip B en el mini proyecto.
const CUT: f64 = 2.0;

struct State {
    comp: Compositor,
    project: Project,
    tex: Option<egui::TextureHandle>,
}

fn sample(path: &PathBuf, a: [f32; 3], b: [f32; 3], circle: bool) -> Option<()> {
    let mut img = image::RgbaImage::new(W * 2, H * 2);
    let (w, h) = (img.width() as f32, img.height() as f32);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let (u, v) = (x as f32 / w, y as f32 / h);
        let t = (u * 0.7 + v * 0.3).clamp(0.0, 1.0);
        let mut c = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
        // Figura central para que se aprecie el movimiento.
        let (dx, dy) = ((u - 0.5) * w / h, v - 0.5);
        let inside = if circle { dx * dx + dy * dy < 0.07 } else { dx.abs() < 0.24 && dy.abs() < 0.24 };
        if inside {
            c = [1.0 - c[0] * 0.3, 1.0 - c[1] * 0.3, 1.0 - c[2] * 0.3];
        }
        *px = image::Rgba([(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]);
    }
    img.save(path).ok()
}

fn clip(id: &str, media: &str, start: f64, dur: f64, extra: Value) -> Option<Clip> {
    let mut c: Clip = serde_json::from_value(json!({
        "id": id, "mediaId": media, "kind": "image", "name": id, "start": start, "duration": dur,
        "inPoint": 0.0, "speed": 1.0, "volume": 1.0, "opacity": 1.0, "scale": 1.0,
        "x": 0.0, "y": 0.0, "rotate": 0.0, "filter": "none",
        "adjust": { "brightness": 100.0, "contrast": 100.0, "saturate": 100.0 },
        "fadeIn": 0.0, "fadeOut": 0.0
    }))
    .ok()?;
    if let Some(o) = extra.as_object() {
        c.extra.extend(o.clone());
    }
    Some(c)
}

fn init() -> Option<State> {
    let dir = std::env::temp_dir().join("cupcraft-trans-preview");
    std::fs::create_dir_all(&dir).ok()?;
    let pa = dir.join("a.png");
    let pb = dir.join("b.png");
    if !pa.exists() {
        sample(&pa, [0.05, 0.35, 0.42], [0.10, 0.75, 0.70], true)?;
    }
    if !pb.exists() {
        sample(&pb, [0.55, 0.18, 0.55], [0.95, 0.45, 0.30], false)?;
    }
    let paths = HashMap::from([("pvA".to_string(), pa), ("pvB".to_string(), pb)]);
    let dims = HashMap::from([("pvA".to_string(), (W * 2, H * 2)), ("pvB".to_string(), (W * 2, H * 2))]);
    let mut project = Project::new("preview");
    project.ratio = "16:9".into();
    let video = project.tracks.iter_mut().find(|t| t.id == "track_video")?;
    video.clips.push(clip("A", "pvA", 0.0, CUT, json!({}))?);
    video.clips.push(clip("B", "pvB", CUT, 4.0, json!({}))?);
    Some(State { comp: Compositor::new(W, H, paths, dims), project, tex: None })
}

thread_local! {
    // El compositor no es `Send`; la interfaz siempre corre en el mismo hilo.
    static STATE: RefCell<Option<Option<State>>> = const { RefCell::new(None) };
}

/// Fotograma animado de la transición `kind` (bucle: A → transición → B).
/// Devuelve la textura para pintar en la tarjeta.
pub fn frame(ctx: &egui::Context, kind: &str, dur: f64) -> Option<egui::TextureId> {
    STATE.with(|cell| {
    let mut slot = cell.borrow_mut();
    let st = slot.get_or_insert_with(init).as_mut()?;
    let dur = dur.clamp(0.2, 2.4);
    let (fi, fo) = crate::panels::transition_fades(kind);
    if let Some(b) = st.project.find_clip_mut("B") {
        b.extra.insert("transition".into(), json!(kind));
        b.extra.insert("transitionDuration".into(), json!(dur));
        b.fade_in = fi;
        b.fade_out = fo;
    }
    // A visible 0,35 s, la transición y B quieto 0,6 s.
    let cycle = dur + 0.95;
    let ph = ctx.input(|i| i.time) % cycle;
    let t = CUT - 0.35 + ph;
    let rgba = st.comp.frame(&st.project, t);
    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], &rgba);
    match &mut st.tex {
        Some(tex) => tex.set(img, egui::TextureOptions::LINEAR),
        None => st.tex = Some(ctx.load_texture("transition-hover", img, egui::TextureOptions::LINEAR)),
    }
    ctx.request_repaint();
    st.tex.as_ref().map(|t| t.id())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transicion_libre_oscurece_el_anterior_y_aclara_el_siguiente() {
        let mut st = init().expect("mini proyecto");
        let b = st.project.find_clip_mut("B").unwrap();
        b.extra.insert("transition".into(), json!("black"));
        b.extra.insert("transitionDuration".into(), json!(0.8));
        b.extra.insert("transitionLead".into(), json!(0.4));
        let lum = |f: &Vec<u8>| f.chunks(4).map(|p| p[0] as u64 + p[1] as u64 + p[2] as u64).sum::<u64>();
        let before = lum(&st.comp.frame(&st.project, CUT - 0.45));
        let mid_a = lum(&st.comp.frame(&st.project, CUT - 0.2));
        let at_cut = lum(&st.comp.frame(&st.project, CUT + 0.001));
        let mid_b = lum(&st.comp.frame(&st.project, CUT + 0.2));
        let after = lum(&st.comp.frame(&st.project, CUT + 0.6));
        assert!(mid_a < before, "el anterior se oscurece antes del corte");
        assert!(at_cut < mid_a, "en el corte está oscuro");
        assert!(mid_b > at_cut, "el siguiente se aclara después del corte");
        assert!(after > mid_b, "y vuelve la luz completa");
    }

    #[test]
    fn la_vista_previa_de_transicion_se_mueve() {
        let mut st = init().expect("mini proyecto");
        let b = st.project.find_clip_mut("B").unwrap();
        b.extra.insert("transition".into(), json!("slide-left"));
        b.extra.insert("transitionDuration".into(), json!(0.8));
        let a = st.comp.frame(&st.project, CUT - 0.2);
        let mid = st.comp.frame(&st.project, CUT + 0.4);
        let end = st.comp.frame(&st.project, CUT + 1.5);
        assert!(a.iter().any(|&v| v > 0), "A no es negro");
        assert_ne!(a, mid);
        assert_ne!(mid, end);
    }
}
