//! Modelo de datos de cupcraft.
//!
//! El modelo replica el JSON que hoy guarda la app web (IndexedDB y el archivo
//! `.opencup.json`), de modo que un proyecto exportado desde la versión JS se
//! abre aquí sin conversión. Los campos que aún no se modelan se conservan en
//! `extra` para no perder información al reescribir el proyecto.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Tipos de pista que reconoce la app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackType {
    Video,
    Audio,
    Text,
    Overlay,
}

/// Tipos de clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipKind {
    Video,
    Image,
    Audio,
    Text,
    Sticker,
}

/// Ajustes de color del clip (los mismos valores que usa el panel Ajustar).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adjust {
    #[serde(default = "adjust_default")]
    pub brightness: f64,
    #[serde(default = "adjust_default")]
    pub contrast: f64,
    #[serde(default = "adjust_default")]
    pub saturate: f64,
}

fn adjust_default() -> f64 {
    100.0
}

impl Default for Adjust {
    fn default() -> Self {
        Self { brightness: 100.0, contrast: 100.0, saturate: 100.0 }
    }
}

/// Un clip de la línea de tiempo. Los campos opcionales son los que solo
/// existen en ciertos tipos de clip (texto, sticker, filtros GL/LUT).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_id: Option<String>,
    pub kind: ClipKind,
    #[serde(default)]
    pub name: String,
    pub start: f64,
    pub duration: f64,
    /// Punto de entrada dentro del medio original (segundos).
    #[serde(default, rename = "in")]
    pub in_point: f64,
    #[serde(default = "one")]
    pub speed: f64,
    #[serde(default = "one")]
    pub volume: f64,
    #[serde(default = "one")]
    pub opacity: f64,
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    /// Filtro de color de la línea propia (`none` = sin filtro).
    #[serde(default = "none_filter")]
    pub filter: String,
    #[serde(default)]
    pub adjust: Adjust,
    #[serde(default)]
    pub fade_in: f64,
    #[serde(default)]
    pub fade_out: f64,

    // --- Solo texto ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Value>,
    // --- Solo sticker ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker: Option<Value>,
    // --- Filtros con motor GL ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gl_filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gl_filter_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gl_filter_amount: Option<f64>,
    // --- Filtros con LUT (proyectos antiguos) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lut: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lut_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lut_amount: Option<f64>,

    /// Cualquier otro campo que llegue del JSON (keyframes, animaciones…)
    /// se conserva tal cual para no perder datos al guardar.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn one() -> f64 {
    1.0
}
fn none_filter() -> String {
    "none".to_string()
}

impl Clip {
    /// Fin del clip en la línea de tiempo.
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }
}

/// Una pista de la línea de tiempo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: TrackType,
    pub name: String,
    #[serde(default)]
    pub clips: Vec<Clip>,
}

impl Track {
    /// Último fin de los clips de la pista, opcionalmente ignorando uno.
    pub fn end(&self, exclude: Option<&str>) -> f64 {
        self.clips
            .iter()
            .filter(|c| exclude.map_or(true, |id| c.id != id))
            .map(Clip::end)
            .fold(0.0f64, f64::max)
    }
}

/// Metadatos de un medio importado (el blob vive en el almacén aparte).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Proyecto completo: tal y como se guarda en IndexedDB y en `.opencup.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default = "default_ratio")]
    pub ratio: String,
    #[serde(default = "default_resolution")]
    pub resolution: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default)]
    pub media: Vec<Media>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_ratio() -> String {
    "16:9".to_string()
}
fn default_resolution() -> String {
    "1080p".to_string()
}
fn default_fps() -> u32 {
    30
}

impl Project {
    /// Crea un proyecto vacío con las tres pistas por defecto.
    pub fn new(name: impl Into<String>) -> Self {
        let now = now_millis();
        Self {
            id: new_id("prj"),
            name: name.into(),
            created_at: now,
            updated_at: now,
            ratio: default_ratio(),
            resolution: default_resolution(),
            fps: default_fps(),
            media: Vec::new(),
            thumb: None,
            tracks: vec![
                Track { id: "track_text".into(), kind: TrackType::Text, name: "Texto".into(), clips: Vec::new() },
                Track { id: "track_video".into(), kind: TrackType::Video, name: "Video 1".into(), clips: Vec::new() },
                Track { id: "track_audio".into(), kind: TrackType::Audio, name: "Audio 1".into(), clips: Vec::new() },
            ],
            extra: Map::new(),
        }
    }

    /// Duración total del proyecto (fin del último clip).
    pub fn duration(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(Clip::end)
            .fold(0.0f64, f64::max)
    }

    /// Longitud de la regla: siempre deja 4 s de margen, mínimo 10 s.
    pub fn timeline_length(&self) -> f64 {
        (self.duration() + 4.0).max(10.0)
    }

    pub fn track(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn track_mut(&mut self, id: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    /// Busca un clip por id y devuelve también su pista.
    pub fn find_clip(&self, id: &str) -> Option<(&Track, &Clip)> {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter().map(move |c| (t, c)))
            .find(|(_, c)| c.id == id)
    }

    /// Busca un clip por id de forma mutable (la pista se recupera aparte con
    /// `track_mut` si hace falta; devolver ambas a la vez sería solaparse).
    pub fn find_clip_mut(&mut self, id: &str) -> Option<&mut Clip> {
        for track in &mut self.tracks {
            let Some(index) = track.clips.iter().position(|c| c.id == id) else { continue };
            return Some(&mut track.clips[index]);
        }
        None
    }
}

/// Millisegundos desde el epoch (equivalente a `Date.now()`).
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Identificador con el mismo formato que genera la app JS
/// (`prefijo_<base36 aleatorio>`).
pub fn new_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    let mut seed = nanos as u64 ^ 0x9E37_79B9_7F4A_7C15;
    // xorshift simple: suficiente para ids no colisionantes en sesión.
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    format!("{prefix}_{}", base36(seed))
}

fn base36(mut value: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    // Rellena a 8 caracteres para que los ids se vean uniformes.
    while out.len() < 8 {
        out.insert(0, b'0');
    }
    String::from_utf8(out).unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_new_has_default_tracks() {
        let p = Project::new("Demo");
        assert_eq!(p.tracks.len(), 3);
        assert_eq!(p.tracks[0].kind, TrackType::Text);
        assert_eq!(p.tracks[1].kind, TrackType::Video);
        assert_eq!(p.tracks[2].kind, TrackType::Audio);
        assert_eq!(p.ratio, "16:9");
        assert_eq!(p.fps, 30);
    }

    #[test]
    fn duration_and_timeline_length() {
        let mut p = Project::new("Demo");
        p.tracks[1].clips.push(Clip {
            id: "c1".into(),
            media_id: None,
            kind: ClipKind::Image,
            name: "foto".into(),
            start: 2.0,
            duration: 5.0,
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
            extra: Map::new(),
        });
        assert_eq!(p.duration(), 7.0);
        assert_eq!(p.timeline_length(), 11.0);
    }

    #[test]
    fn ids_look_like_the_js_ones() {
        let id = new_id("clip");
        assert!(id.starts_with("clip_"));
        assert!(id.len() > 8);
    }

    #[test]
    fn unknown_fields_survive_a_round_trip() {
        let json = r#"{
            "id":"prj_1","name":"X","createdAt":1,"updatedAt":2,
            "ratio":"9:16","resolution":"720p","fps":24,"media":[],"tracks":[],
            "customFlag": true
        }"#;
        let p: Project = serde_json::from_str(json).unwrap();
        assert_eq!(p.ratio, "9:16");
        assert_eq!(p.fps, 24);
        assert_eq!(p.extra.get("customFlag"), Some(&Value::Bool(true)));
        let out = serde_json::to_string(&p).unwrap();
        assert!(out.contains("customFlag"));
    }
}
