//! Operaciones sobre el proyecto: añadir, mover, recortar, dividir, borrar…
//!
//! Es el equivalente a las acciones de `src/store.js`, con la misma semántica
//! (incluido el encaje automático de clips con `resolve_start`).

use crate::model::{new_id, Adjust, Clip, ClipKind, Project, Track, TrackType};

/// Coloca un clip sin solaparse con los demás de su pista.
///
/// Reproduce el algoritmo de `resolveStart` de la app JS: traslada el inicio
/// al final (o al inicio) de cada clip con el que colisiona, hasta 6 pasadas.
pub fn resolve_start(track: &Track, clip_id: Option<&str>, desired: f64, duration: f64) -> f64 {
    let mut others: Vec<(f64, f64)> = track
        .clips
        .iter()
        .filter(|c| clip_id.map_or(true, |id| c.id != id))
        .map(|c| (c.start, c.duration))
        .collect();
    others.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut start = desired.max(0.0);
    for _ in 0..6 {
        let mut moved = false;
        for &(other_start, other_duration) in &others {
            let other_end = other_start + other_duration;
            if start < other_end && start + duration > other_start {
                let prefer_after = desired + duration / 2.0 > other_start + other_duration / 2.0;
                start = if prefer_after { other_end } else { (other_start - duration).max(0.0) };
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    start.max(0.0)
}

/// Crea un clip listo para insertar en el timeline (campos por defecto).
pub fn make_clip(kind: ClipKind, name: impl Into<String>, start: f64, duration: f64) -> Clip {
    Clip {
        id: new_id("clip"),
        media_id: None,
        kind,
        name: name.into(),
        start,
        duration,
        in_point: 0.0,
        speed: 1.0,
        volume: 1.0,
        opacity: 1.0,
        scale: 1.0,
        x: 0.0,
        y: 0.0,
        filter: "none".into(),
        adjust: Adjust::default(),
        fade_in: 0.0,
        fade_out: 0.0,
        text: None,
        sticker: None,
        gl_filter: None,
        gl_filter_name: None,
        gl_filter_amount: None,
        lut: None,
        lut_name: None,
        lut_amount: None,
        extra: Default::default(),
    }
}

impl Project {
    /// Pista destino para un medio: la indicada si coincide en tipo, si no la
    /// primera del tipo adecuado.
    pub fn track_for(&self, wanted: Option<&str>, kind: TrackType) -> Option<&Track> {
        if let Some(id) = wanted {
            if let Some(t) = self.track(id) {
                if t.kind == kind {
                    return Some(t);
                }
            }
        }
        self.tracks.iter().find(|t| t.kind == kind)
    }

    /// Añade un clip de medio a su pista, encajando contra los existentes.
    /// Devuelve el id del clip creado.
    pub fn add_media_clip(
        &mut self,
        track_id: Option<&str>,
        kind: ClipKind,
        name: impl Into<String>,
        duration: f64,
        start: Option<f64>,
        media_id: Option<String>,
    ) -> Option<String> {
        let track_kind = if kind == ClipKind::Audio { TrackType::Audio } else { TrackType::Video };
        let destination = self.track_for(track_id, track_kind)?.id.clone();
        let desired = start.unwrap_or_else(|| {
            self.track(&destination).map(|t| t.end(None)).unwrap_or(0.0)
        });
        let final_start = {
            let track = self.track(&destination)?;
            resolve_start(track, None, desired, duration)
        };
        let mut clip = make_clip(kind, name, final_start, duration);
        clip.media_id = media_id;
        let track = self.track_mut(&destination)?;
        track.clips.push(clip);
        Some(track.clips.last()?.id.clone())
    }

    /// Elimina un clip por id.
    pub fn remove_clip(&mut self, clip_id: &str) -> bool {
        for track in &mut self.tracks {
            let before = track.clips.len();
            track.clips.retain(|c| c.id != clip_id);
            if track.clips.len() != before {
                return true;
            }
        }
        false
    }

    /// Duplica un clip colocándolo justo después del original.
    pub fn duplicate_clip(&mut self, clip_id: &str) -> Option<String> {
        let (track_id, mut clone, desired) = {
            let (track, clip) = self.find_clip(clip_id)?;
            let mut clone = clip.clone();
            clone.id = new_id("clip");
            (track.id.clone(), clone, clip.end())
        };
        let start = {
            let track = self.track(&track_id)?;
            resolve_start(track, None, desired, clone.duration)
        };
        clone.start = start;
        let track = self.track_mut(&track_id)?;
        track.clips.push(clone);
        track.clips.last().map(|c| c.id.clone())
    }

    /// Divide todos los clips que contengan `time` (el playhead).
    /// Si `only` está indicado, solo se divide ese clip.
    pub fn split_at(&mut self, time: f64, only: Option<&str>) -> usize {
        let mut targets: Vec<(String, String, f64, f64, f64, ClipKind)> = Vec::new();
        for track in &self.tracks {
            for clip in &track.clips {
                if let Some(id) = only {
                    if clip.id != id {
                        continue;
                    }
                }
                if time > clip.start + 0.05 && time < clip.end() - 0.05 {
                    targets.push((
                        track.id.clone(),
                        clip.id.clone(),
                        clip.start,
                        clip.duration,
                        clip.in_point,
                        clip.kind,
                    ));
                }
            }
        }
        let mut done = 0;
        for (_track_id, clip_id, start, duration, in_point, _kind) in targets {
            let right = {
                let (_track, clip) = match self.find_clip(&clip_id) {
                    Some(pair) => pair,
                    None => continue,
                };
                let mut right = clip.clone();
                right.id = new_id("clip");
                right.start = time;
                right.duration = start + duration - time;
                right.in_point = in_point + (time - start) * clip_speed(clip);
                right
            };
            let left_duration = time - start;
            let track_id = self.find_clip(&clip_id).map(|(t, _)| t.id.clone());
            let Some(track_id) = track_id else { continue };
            if let Some(clip) = self.find_clip_mut(&clip_id) {
                clip.duration = left_duration;
            } else {
                continue;
            }
            if let Some(track) = self.track_mut(&track_id) {
                track.clips.push(right);
                done += 1;
            }
        }
        done
    }

    /// Aplica un parche de campos a un clip (los objetos se fusionan).
    pub fn update_clip(&mut self, clip_id: &str, patch: serde_json::Value) {
        let Some(clip) = self.find_clip_mut(clip_id) else { return };
        let Some(map) = patch.as_object() else { return };
        for (key, value) in map {
            match key.as_str() {
                "text" | "sticker" | "adjust" => {
                    let current = match key.as_str() {
                        "text" => clip.text.as_ref(),
                        "sticker" => clip.sticker.as_ref(),
                        _ => None,
                    };
                    let mut merged = match current {
                        Some(serde_json::Value::Object(existing)) => existing.clone(),
                        _ => serde_json::Map::new(),
                    };
                    if let Some(extra) = value.as_object() {
                        for (k, v) in extra {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                    let merged = serde_json::Value::Object(merged);
                    match key.as_str() {
                        "text" => clip.text = Some(merged),
                        "sticker" => clip.sticker = Some(merged),
                        _ => {
                            if let Ok(adjust) = serde_json::from_value::<Adjust>(merged) {
                                clip.adjust = adjust;
                            }
                        }
                    }
                }
                "start" => clip.start = value.as_f64().unwrap_or(clip.start).max(0.0),
                "duration" => clip.duration = value.as_f64().unwrap_or(clip.duration).max(0.1),
                "in" => clip.in_point = value.as_f64().unwrap_or(clip.in_point),
                "filter" => {
                    if let Some(v) = value.as_str() {
                        clip.filter = v.to_string();
                    }
                }
                _ => {
                    // Campos escalares conocidos.
                    let target: Option<&mut f64> = match key.as_str() {
                        "speed" => Some(&mut clip.speed),
                        "volume" => Some(&mut clip.volume),
                        "opacity" => Some(&mut clip.opacity),
                        "scale" => Some(&mut clip.scale),
                        "x" => Some(&mut clip.x),
                        "y" => Some(&mut clip.y),
                        "fadeIn" => Some(&mut clip.fade_in),
                        "fadeOut" => Some(&mut clip.fade_out),
                        "glFilterAmount" => clip.gl_filter_amount.as_mut(),
                        "lutAmount" => clip.lut_amount.as_mut(),
                        _ => None,
                    };
                    if let Some(slot) = target {
                        if let Some(v) = value.as_f64() {
                            *slot = v;
                        }
                    } else {
                        // Texto/nombre y filtros especiales.
                        match key.as_str() {
                            "name" => {
                                if let Some(v) = value.as_str() {
                                    clip.name = v.to_string();
                                }
                            }
                            "glFilter" => clip.gl_filter = value.as_str().map(str::to_string),
                            "glFilterName" => clip.gl_filter_name = value.as_str().map(str::to_string),
                            "lut" => clip.lut = value.as_str().map(str::to_string),
                            "lutName" => clip.lut_name = value.as_str().map(str::to_string),
                            other => {
                                // Campo no modelado: se conserva tal cual.
                                clip.extra.insert(other.to_string(), value.clone());
                            }
                        }
                    }
                }
            }
        }
        clip.start = clip.start.max(0.0);
        clip.duration = clip.duration.max(0.1);
    }

    /// Mueve un clip a otro inicio y/o pista (respetando el encaje).
    pub fn move_clip(&mut self, clip_id: &str, start: Option<f64>, track_id: Option<&str>) {
        let Some((origin_id, kind, duration)) = self
            .find_clip(clip_id)
            .map(|(t, c)| (t.id.clone(), t.kind, c.duration))
        else { return };
        let destination = match track_id {
            Some(id) if id != origin_id => self.track(id).filter(|t| t.kind == kind).map(|t| t.id.clone()),
            _ => None,
        };
        let destination_id = destination.unwrap_or_else(|| origin_id.clone());
        let desired = start.unwrap_or_else(|| {
            self.find_clip(clip_id).map(|(_, c)| c.start).unwrap_or(0.0)
        });
        let final_start = match self.track(&destination_id) {
            Some(t) => resolve_start(t, Some(clip_id), desired, duration),
            None => return,
        };
        if destination_id != origin_id {
            let mut moved = None;
            if let Some(origin) = self.track_mut(&origin_id) {
                let pos = origin.clips.iter().position(|c| c.id == clip_id);
                if let Some(pos) = pos {
                    moved = Some(origin.clips.remove(pos));
                }
            }
            if let Some(mut clip) = moved {
                clip.start = final_start;
                if let Some(dest) = self.track_mut(&destination_id) {
                    dest.clips.push(clip);
                }
            }
        } else if let Some(clip) = self.find_clip_mut(clip_id) {
            clip.start = final_start;
        }
    }

    /// Recorta un clip por el borde izquierdo o derecho.
    pub fn trim_clip(&mut self, clip_id: &str, edge: TrimEdge, delta: f64, media_duration: Option<f64>) {
        let Some(clip) = self.find_clip_mut(clip_id) else { return };
        let infinite = matches!(clip.kind, ClipKind::Image | ClipKind::Text | ClipKind::Sticker);
        let source_duration = if infinite {
            f64::INFINITY
        } else {
            let base = media_duration
                .map(|d| (d - clip.in_point).max(0.1))
                .unwrap_or_else(|| (clip.in_point + clip.duration).max(0.1));
            base.max(0.1)
        };
        match edge {
            TrimEdge::Left => {
                let min_start = (clip.start + clip.duration - 0.1).max(0.0);
                let next_start = (clip.start + delta).clamp(0.0, min_start);
                let shift = next_start - clip.start;
                clip.start = next_start;
                clip.duration = (clip.duration - shift).max(0.1);
                if !matches!(clip.kind, ClipKind::Image | ClipKind::Text) {
                    clip.in_point = (clip.in_point + shift * clip.speed).max(0.0);
                }
            }
            TrimEdge::Right => {
                clip.duration = (clip.duration + delta).clamp(0.1, source_duration);
            }
        }
    }

    /// Nombre único para una pista nueva del tipo indicado.
    pub fn next_track_name(&self, kind: TrackType) -> String {
        let count = self.tracks.iter().filter(|t| t.kind == kind).count();
        let base = match kind {
            TrackType::Video => "Vídeo",
            TrackType::Audio => "Audio",
            TrackType::Text => "Texto",
            TrackType::Overlay => "Superposición",
        };
        let mut name = if count == 0 { base.to_string() } else { format!("{base} {}", count + 1) };
        let mut n = count + 2;
        while self.tracks.iter().any(|t| t.name == name) {
            name = format!("{base} {n}");
            n += 1;
        }
        name
    }

    /// Añade una pista vacía y devuelve su id.
    pub fn add_track(&mut self, kind: TrackType) -> String {
        let name = self.next_track_name(kind);
        let track = Track { id: new_id("track"), kind, name, clips: Vec::new() };
        self.tracks.push(track);
        self.tracks.last().map(|t| t.id.clone()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimEdge {
    Left,
    Right,
}

fn clip_speed(clip: &Clip) -> f64 {
    clip.speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Project;

    fn project_with_clip() -> Project {
        let mut p = Project::new("Demo");
        let id = p
            .add_media_clip(None, ClipKind::Image, "foto.png", 5.0, None, Some("m1".into()))
            .unwrap();
        assert!(!id.is_empty());
        p
    }

    fn project_with_video_clip() -> Project {
        let mut p = Project::new("Demo");
        let id = p
            .add_media_clip(None, ClipKind::Video, "toma.mp4", 5.0, None, Some("m1".into()))
            .unwrap();
        assert!(!id.is_empty());
        p
    }

    #[test]
    fn resolve_start_pushes_past_collisions() {
        let mut track = Track { id: "t".into(), kind: TrackType::Video, name: "V".into(), clips: Vec::new() };
        track.clips.push(make_clip(ClipKind::Image, "a", 0.0, 5.0));
        // Deseado dentro del primer clip y más cercano a su final: lo empuja al final.
        let start = resolve_start(&track, None, 4.0, 3.0);
        assert!((start - 5.0).abs() < 1e-9, "start={start}");
        // Deseado antes del clip: lo coloca antes.
        let start = resolve_start(&track, None, 6.0, 3.0);
        assert!((start - 6.0).abs() < 1e-9, "start={start}");
    }

    #[test]
    fn resolve_start_never_goes_negative() {
        let track = Track { id: "t".into(), kind: TrackType::Video, name: "V".into(), clips: Vec::new() };
        assert_eq!(resolve_start(&track, None, -10.0, 2.0), 0.0);
    }

    #[test]
    fn add_media_clip_appends_to_video_track() {
        let mut p = project_with_clip();
        assert_eq!(p.tracks[1].clips.len(), 1);
        let second = p
            .add_media_clip(None, ClipKind::Image, "otra.png", 5.0, None, Some("m2".into()))
            .unwrap();
        let clip = p.find_clip(&second).unwrap().1;
        assert!((clip.start - 5.0).abs() < 1e-9, "debería ir tras el primero");
    }

    #[test]
    fn audio_clip_goes_to_audio_track() {
        let mut p = Project::new("Demo");
        p.add_media_clip(None, ClipKind::Audio, "cancion.mp3", 10.0, None, None);
        assert_eq!(p.tracks[2].clips.len(), 1);
        assert!(p.tracks[1].clips.is_empty());
    }

    #[test]
    fn duplicate_places_clip_after_original() {
        let mut p = project_with_clip();
        let original = p.tracks[1].clips[0].id.clone();
        let copy = p.duplicate_clip(&original).unwrap();
        let (start, id) = {
            let c = p.find_clip(&copy).unwrap().1;
            (c.start, c.id.clone())
        };
        assert!((start - 5.0).abs() < 1e-9);
        assert_ne!(id, original);
        assert_eq!(p.tracks[1].clips.len(), 2);
    }

    #[test]
    fn split_creates_two_halves() {
        let mut p = project_with_clip();
        let id = p.tracks[1].clips[0].id.clone();
        let done = p.split_at(2.0, Some(&id));
        assert_eq!(done, 1);
        assert_eq!(p.tracks[1].clips.len(), 2);
        let left = p.find_clip(&id).unwrap().1;
        assert!((left.duration - 2.0).abs() < 1e-9);
        assert!((left.in_point).abs() < 1e-9);
        let right = p.tracks[1].clips.iter().find(|c| c.id != id).unwrap();
        assert!((right.start - 2.0).abs() < 1e-9);
        assert!((right.duration - 3.0).abs() < 1e-9);
        assert!((right.in_point - 2.0).abs() < 1e-9);
    }

    #[test]
    fn split_outside_clip_does_nothing() {
        let mut p = project_with_clip();
        assert_eq!(p.split_at(99.0, None), 0);
        assert_eq!(p.tracks[1].clips.len(), 1);
    }

    #[test]
    fn trim_left_shifts_start_and_in_point() {
        let mut p = project_with_video_clip();
        let id = p.tracks[1].clips[0].id.clone();
        p.trim_clip(&id, TrimEdge::Left, 1.0, Some(30.0));
        let c = p.find_clip(&id).unwrap().1;
        assert!((c.start - 1.0).abs() < 1e-9);
        assert!((c.duration - 4.0).abs() < 1e-9);
        assert!((c.in_point - 1.0).abs() < 1e-9);
    }

    #[test]
    fn trim_right_respects_media_duration() {
        let mut p = project_with_video_clip();
        let id = p.tracks[1].clips[0].id.clone();
        // El medio dura 6 s y el clip ya ocupa 5: solo puede crecer 1.
        p.trim_clip(&id, TrimEdge::Right, 10.0, Some(6.0));
        let c = p.find_clip(&id).unwrap().1;
        assert!((c.duration - 6.0).abs() < 1e-9, "duration={}", c.duration);
    }

    #[test]
    fn image_clips_have_infinite_length_and_keep_their_in_point() {
        // Las imágenes/textos/stickers no se limitan por la duración del medio
        // y su `in` no se mueve al recortar por la izquierda (igual que en JS).
        let mut p = project_with_clip();
        let id = p.tracks[1].clips[0].id.clone();
        p.trim_clip(&id, TrimEdge::Right, 10_000.0, None);
        assert!((p.find_clip(&id).unwrap().1.duration - 10_005.0).abs() < 1e-9);
        p.trim_clip(&id, TrimEdge::Left, 1.0, Some(30.0));
        let c = p.find_clip(&id).unwrap().1;
        assert_eq!(c.in_point, 0.0, "el in de una imagen no cambia");
    }

    #[test]
    fn trim_left_never_eats_the_whole_clip() {
        let mut p = project_with_clip();
        let id = p.tracks[1].clips[0].id.clone();
        p.trim_clip(&id, TrimEdge::Left, -100.0, Some(30.0));
        let c = p.find_clip(&id).unwrap().1;
        assert!(c.duration >= 0.1 - 1e-9);
    }

    #[test]
    fn update_clip_merges_adjust_and_sets_scalars() {
        let mut p = project_with_clip();
        let id = p.tracks[1].clips[0].id.clone();
        let patch = serde_json::json!({
            "adjust": { "brightness": 120 },
            "opacity": 0.5,
            "filter": "vivid",
            "start": 1.5
        });
        p.update_clip(&id, patch);
        let c = p.find_clip(&id).unwrap().1;
        assert_eq!(c.adjust.brightness, 120.0);
        assert_eq!(c.adjust.contrast, 100.0, "los campos no indicados se conservan");
        assert!((c.opacity - 0.5).abs() < 1e-9);
        assert_eq!(c.filter, "vivid");
        assert!((c.start - 1.5).abs() < 1e-9);
    }

    #[test]
    fn update_clip_clamps_start_and_duration() {
        let mut p = project_with_clip();
        let id = p.tracks[1].clips[0].id.clone();
        p.update_clip(&id, serde_json::json!({ "start": -5, "duration": 0 }));
        let c = p.find_clip(&id).unwrap().1;
        assert_eq!(c.start, 0.0);
        assert!((c.duration - 0.1).abs() < 1e-9);
    }

    #[test]
    fn move_clip_to_other_track_of_same_type() {
        let mut p = Project::new("Demo");
        let a = p.add_track(TrackType::Video);
        let b = p.add_track(TrackType::Video);
        let clip = p
            .add_media_clip(Some(&a), ClipKind::Image, "x.png", 5.0, None, None)
            .unwrap();
        p.move_clip(&clip, Some(0.0), Some(&b));
        assert!(p.track(&a).unwrap().clips.is_empty());
        assert_eq!(p.track(&b).unwrap().clips.len(), 1);
    }

    #[test]
    fn move_clip_refuses_track_of_other_type() {
        let mut p = Project::new("Demo");
        let clip = p
            .add_media_clip(None, ClipKind::Image, "x.png", 5.0, None, None)
            .unwrap();
        let audio = p.tracks[2].id.clone();
        p.move_clip(&clip, Some(0.0), Some(&audio));
        // Sigue en la pista de vídeo.
        assert_eq!(p.tracks[1].clips.len(), 1);
        assert!(p.tracks[2].clips.is_empty());
    }

    #[test]
    fn remove_clip_works() {
        let mut p = project_with_clip();
        let id = p.tracks[1].clips[0].id.clone();
        assert!(p.remove_clip(&id));
        assert!(p.tracks[1].clips.is_empty());
        assert!(!p.remove_clip("no-existe"));
    }

    #[test]
    fn next_track_names_are_unique() {
        let mut p = Project::new("Demo");
        assert_eq!(p.next_track_name(TrackType::Video), "Vídeo 2");
        p.add_track(TrackType::Video);
        assert_eq!(p.next_track_name(TrackType::Video), "Vídeo 3");
        assert_eq!(p.next_track_name(TrackType::Audio), "Audio 2");
    }
}
