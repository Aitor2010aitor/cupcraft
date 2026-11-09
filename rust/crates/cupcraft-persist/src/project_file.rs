//! Archivo de proyecto `.opencup.json`: mismo formato que exporta e importa
//! la versión JS (`exportProjectFile`/`importProjectFile` de `src/store.js`).
//!
//! Estructura:
//! ```json
//! {
//!   "kind": "openCUP-project",
//!   "fileVersion": 1,
//!   "app": "openCUP",
//!   "exportedAt": 1730000000000,
//!   "project": { ...proyecto sin `media`... },
//!   "media": [ { "id": "...", "mime": "...", "data": "data:...;base64,..." } ]
//! }
//! ```

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use cupcraft_core::{Media, Project};
use serde::{Deserialize, Serialize};

/// Identificador de tipo del archivo (nombre técnico interno).
pub const PROJECT_FILE_KIND: &str = "openCUP-project";
/// Versión del formato de archivo.
pub const PROJECT_FILE_VERSION: u32 = 1;
/// Límite de medios incrustados (256 MiB), igual que en la app JS.
pub const PROJECT_FILE_EMBED_LIMIT: u64 = 256 * 1024 * 1024;

/// Un medio dentro del archivo: metadatos + (opcionalmente) sus bytes en
/// data URL. Los bytes pueden faltar si el medio superaba el límite al
/// exportar; entonces se cuenta como "missing" al importar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaEntry {
    #[serde(flatten)]
    pub meta: Media,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Resultado de importar un archivo.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportReport {
    pub id: String,
    pub name: String,
    /// Medios descritos en el archivo (con o sin bytes).
    pub media: usize,
    /// Medios sin archivo legible (igual que el contador `missing` de la app JS).
    pub missing: usize,
}

/// Error al leer un archivo de proyecto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectFileError {
    /// JSON inválido o tipo desconocido.
    NotAProject,
    /// `kind` correcto pero falta `project.tracks`.
    Malformed,
}

impl std::fmt::Display for ProjectFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectFileError::NotAProject => f.write_str("El archivo no es un proyecto válido de cupcraft"),
            ProjectFileError::Malformed => f.write_str("El archivo de proyecto está incompleto (faltan pistas)"),
        }
    }
}

impl std::error::Error for ProjectFileError {}

/// Payload completo del archivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub kind: String,
    #[serde(rename = "fileVersion")]
    pub file_version: u32,
    pub app: String,
    #[serde(rename = "exportedAt")]
    pub exported_at: i64,
    pub project: Project,
    #[serde(default)]
    pub media: Vec<MediaEntry>,
}

impl ProjectFile {
    /// Construye el payload de exportación (la app JS le llama `payload`).
    pub fn new(project: Project, media: Vec<MediaEntry>) -> Self {
        Self {
            kind: PROJECT_FILE_KIND.into(),
            file_version: PROJECT_FILE_VERSION,
            app: "openCUP".into(),
            exported_at: cupcraft_core::now_millis(),
            project,
            media,
        }
    }

    /// Analiza el contenido de un `.opencup.json` con la misma validación que
    /// `importProjectFile`: tipo correcto y `project.tracks` presente.
    pub fn parse(text: &str) -> Result<Self, ProjectFileError> {
        let payload: serde_json::Value = serde_json::from_str(text).map_err(|_| ProjectFileError::NotAProject)?;
        if payload.get("kind").and_then(|v| v.as_str()) != Some(PROJECT_FILE_KIND)
            || payload.get("project").and_then(|v| v.get("tracks")).is_none()
        {
            return Err(ProjectFileError::NotAProject);
        }
        serde_json::from_value(payload).map_err(|_| ProjectFileError::Malformed)
    }

    /// Separa los metadatos de los bytes incrustados, como hace la app JS al
    /// importar: se conserva todo lo descrito y se cuentan los que no traen
    /// archivo.
    pub fn split_media(&self) -> (Vec<Media>, usize) {
        let mut metas = Vec::new();
        let mut missing = 0;
        for entry in &self.media {
            if entry.meta.id.is_empty() {
                continue;
            }
            if entry.data.is_none() {
                missing += 1;
            }
            metas.push(entry.meta.clone());
        }
        (metas, missing)
    }

    /// Devuelve el informe de importación (id, nombre y contadores).
    pub fn import_report(&self) -> ImportReport {
        let (metas, missing) = self.split_media();
        ImportReport {
            id: self.project.id.clone(),
            name: self.project.name.clone(),
            media: metas.len(),
            missing,
        }
    }

    /// Serializa el archivo listo para escribirse a disco.
    pub fn to_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

/// Convierte una data URL (`data:<mime>;base64,<datos>` o con payload
/// codificado porcentaje) en (mime, bytes), replicando `dataUrlToBlob`.
pub fn decode_data_url(data_url: &str, fallback_mime: &str) -> Result<(String, Vec<u8>), ProjectFileError> {
    let rest = data_url.strip_prefix("data:").ok_or(ProjectFileError::NotAProject)?;
    let (header, payload) = rest.split_once(',').ok_or(ProjectFileError::NotAProject)?;
    let mut parts = header.split(';');
    let mime = parts
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .filter(|_| !header.starts_with(';'))
        .unwrap_or_else(|| {
            if fallback_mime.is_empty() {
                "application/octet-stream".into()
            } else {
                fallback_mime.into()
            }
        });
    let is_base64 = header.split(';').any(|p| p == "base64");
    let bytes = if is_base64 {
        B64.decode(payload.trim()).map_err(|_| ProjectFileError::NotAProject)?
    } else {
        percent_decode(payload)
    };
    Ok((mime, bytes))
}

/// Codifica bytes como data URL base64 (equivale a `blobToDataURL`).
pub fn encode_data_url(mime: &str, bytes: &[u8]) -> String {
    let mime = if mime.is_empty() { "application/octet-stream" } else { mime };
    format!("data:{mime};base64,{}", B64.encode(bytes))
}

fn percent_decode(input: &str) -> Vec<u8> {
    let raw = input.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'%' if i + 2 < raw.len() + 1 && i + 2 <= raw.len() - 1 => {
                let hex = std::str::from_utf8(&raw[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(raw[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    out
}

/// Nombre de archivo propuesto para una exportación (misma limpieza que la
/// app JS: caracteres prohibidos → `-`, espacios colapsados, tope 60).
pub fn export_file_name(project_name: &str) -> String {
    let cleaned = project_name
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '-' } else { c })
        .collect::<String>();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = collapsed.chars().take(60).collect();
    let trimmed = truncated.trim().to_string();
    let base = if trimmed.is_empty() { "proyecto".to_string() } else { trimmed };
    format!("{base}.opencup.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cupcraft_core::{ClipKind, Project, TrackType};

    fn sample_project() -> Project {
        let mut p = Project::new("Rodaje otoño");
        p.id = "prj_fixture".into();
        p.created_at = 1_700_000_000_000;
        p.updated_at = 1_700_000_100_000;
        p.add_media_clip(None, ClipKind::Image, "foto.png", 5.0, None, Some("m1".into()));
        p.tracks
            .iter_mut()
            .find(|t| t.kind == TrackType::Video)
            .unwrap()
            .clips[0]
            .filter = "vivid".into();
        p
    }

    fn sample_file() -> ProjectFile {
        let mut p = sample_project();
        p.media = vec![Media {
            id: "m1".into(),
            kind: Some("image".into()),
            name: Some("foto.png".into()),
            duration: None,
            thumb: None,
            extra: Default::default(),
        }];
        let entries = vec![MediaEntry {
            meta: p.media[0].clone(),
            mime: Some("image/png".into()),
            data: Some(encode_data_url("image/png", &[1, 2, 3, 4])),
        }];
        p.media.clear();
        ProjectFile::new(p, entries)
    }

    #[test]
    fn round_trip_preserves_project_and_media() {
        let file = sample_file();
        let text = file.to_text();
        let parsed = ProjectFile::parse(&text).unwrap();
        assert_eq!(parsed.kind, PROJECT_FILE_KIND);
        assert_eq!(parsed.file_version, PROJECT_FILE_VERSION);
        assert_eq!(parsed.app, "openCUP");
        assert_eq!(parsed.project.name, "Rodaje otoño");
        assert_eq!(parsed.project.tracks.len(), 3);
        assert_eq!(parsed.media.len(), 1);
        assert_eq!(parsed.media[0].meta.id, "m1");
    }

    #[test]
    fn rejects_foreign_or_broken_files() {
        assert_eq!(ProjectFile::parse("no soy json").unwrap_err(), ProjectFileError::NotAProject);
        assert_eq!(ProjectFile::parse("{}").unwrap_err(), ProjectFileError::NotAProject);
        assert_eq!(
            ProjectFile::parse(r#"{"kind":"capcut-project","project":{"tracks":[]}}"#).unwrap_err(),
            ProjectFileError::NotAProject,
            "otro tipo de archivo se rechaza"
        );
        assert_eq!(
            ProjectFile::parse(r#"{"kind":"openCUP-project","project":{"id":"x"}}"#).unwrap_err(),
            ProjectFileError::NotAProject,
            "sin tracks se rechaza"
        );
    }

    #[test]
    fn missing_media_is_counted_like_in_the_js_import() {
        let mut file = sample_file();
        file.media.push(MediaEntry {
            meta: Media {
                id: "m2".into(),
                kind: Some("video".into()),
                name: Some("clip.mp4".into()),
                duration: Some(3.0),
                thumb: None,
                extra: Default::default(),
            },
            mime: None,
            data: None,
        });
        file.media.push(MediaEntry {
            meta: Media {
                id: "".into(),
                kind: None,
                name: None,
                duration: None,
                thumb: None,
                extra: Default::default(),
            },
            mime: None,
            data: None,
        });
        let (metas, missing) = file.split_media();
        assert_eq!(metas.len(), 2, "las entradas sin id se ignoran");
        assert_eq!(missing, 1, "solo cuenta la que traía id y no bytes");
        let report = file.import_report();
        assert_eq!(report.media, 2);
        assert_eq!(report.missing, 1);
    }

    #[test]
    fn data_url_codec_matches_the_blob_helpers() {
        let (mime, bytes) = decode_data_url("data:image/png;base64,AQIDBA==", "").unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, vec![1, 2, 3, 4]);
        // Sin mime usa el de reserva.
        let (mime, _) = decode_data_url("data:;base64,AQID", "image/webp").unwrap();
        assert_eq!(mime, "image/webp");
        // Payload con porcentajes (no base64).
        let (mime, bytes) = decode_data_url("data:text/plain,a%20b", "").unwrap();
        assert_eq!(mime, "text/plain");
        assert_eq!(bytes, b"a b");
        // Incompatible con dataUrlToBlob: sin coma → error.
        assert!(decode_data_url("data:image/png;base64", "").is_err());
    }

    #[test]
    fn export_file_name_is_sanitized_like_in_js() {
        assert_eq!(export_file_name("Mi proyecto"), "Mi proyecto.opencup.json");
        assert_eq!(export_file_name("a/b:c*d?"), "a-b-c-d-.opencup.json");
        assert_eq!(export_file_name("  "), "proyecto.opencup.json");
        let long = "x".repeat(100);
        assert_eq!(export_file_name(&long).len(), 60 + ".opencup.json".len());
    }

    #[test]
    fn embedded_media_obeys_the_embed_limit() {
        // La decisión de incrustar es del que exporta; aquí comprobamos que el
        // límite declarado es el de la app JS.
        assert_eq!(PROJECT_FILE_EMBED_LIMIT, 256 * 1024 * 1024);
        let file = sample_file();
        let entry = &file.media[0];
        let bytes = decode_data_url(entry.data.as_deref().unwrap(), "").unwrap().1;
        assert!(bytes.len() as u64 <= PROJECT_FILE_EMBED_LIMIT);
    }
}
