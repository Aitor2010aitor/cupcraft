//! Almacén local equivalente a `src/db.js` (IndexedDB → SQLite).
//!
//! Dos tablas con la misma forma que los object stores de la app JS:
//! - `projects`: clave `id`, fila completa en JSON (`projects` guarda el
//!   proyecto incluido el listado de metadatos de medios en `media`).
//! - `media`: clave `id`, los bytes del blob y sus metadatos aparte.
//!
//! Las operaciones replican los nombres y la semántica de `db.*`: `listProjects`
//! ordena por `updatedAt` descendente y `save*` hace *put* (insertar o
//! sobrescribir).

use crate::project_file::{decode_data_url, encode_data_url, ProjectFile};
use cupcraft_core::{Media, Project};
use rusqlite::{params, Connection, OptionalExtension};

/// Registro de medio como lo guarda `db.saveMedia({ id, blob, meta })`.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRecord {
    pub meta: Media,
    pub mime: String,
    pub bytes: Vec<u8>,
}

impl MediaRecord {
    /// Vista previa de data URL (el equivalente a `blob` incrustado).
    pub fn data_url(&self) -> String {
        encode_data_url(&self.mime, &self.bytes)
    }
}

/// Error de disco/esquema del almacén.
#[derive(Debug)]
pub enum StoreError {
    Sql(rusqlite::Error),
    /// Proyecto o medio inexistente cuando la operación lo exige.
    NotFound(String),
    /// Medio corrupto dentro de un archivo de proyecto.
    CorruptMedia(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sql(e) => write!(f, "error de la base de datos: {e}"),
            StoreError::NotFound(what) => write!(f, "no existe: {what}"),
            StoreError::CorruptMedia(what) => write!(f, "medio corrupto: {what}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        StoreError::Sql(value)
    }
}

/// Conexión con el almacén local (se abre sobre la ruta que se le indique).
pub struct Store {
    conn: Connection,
    media_dir: Option<std::path::PathBuf>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  updated_at INTEGER NOT NULL DEFAULT 0,
  data TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS media (
  id TEXT PRIMARY KEY,
  mime TEXT NOT NULL DEFAULT '',
  meta TEXT NOT NULL,
  bytes BLOB,
  file_path TEXT DEFAULT ''
);
";

impl Store {
    /// Abre (creándolo si hace falta) el almacén de la ruta indicada.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StoreError> {
        let path_ref = path.as_ref();
        let media_dir = if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::CorruptMedia(e.to_string()))?;
            let m_dir = parent.join("media");
            let _ = std::fs::create_dir_all(&m_dir);
            Some(m_dir)
        } else {
            None
        };
        let conn = Connection::open(path_ref)?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        conn.execute_batch(SCHEMA)?;
        // Asegurar columna file_path para bases de datos existentes
        let _ = conn.execute("ALTER TABLE media ADD COLUMN file_path TEXT DEFAULT ''", []);
        Ok(Self { conn, media_dir })
    }

    /// Abre un almacén en memoria (para pruebas y vistas previas).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn, media_dir: None })
    }

    /// Establece la carpeta para almacenar los medios en disco.
    pub fn set_media_dir(&mut self, dir: std::path::PathBuf) {
        let _ = std::fs::create_dir_all(&dir);
        self.media_dir = Some(dir);
    }

    /// Carpeta donde se almacenan los medios físicos en disco.
    pub fn media_dir(&self) -> Option<&std::path::Path> {
        self.media_dir.as_deref()
    }

    /* ------------------------------- Proyectos ------------------------------ */

    /// Guarda un proyecto (inserta o sobrescribe), como `db.saveProject`.
    pub fn save_project(&mut self, project: &Project) -> Result<(), StoreError> {
        let data = serde_json::to_string(project).map_err(|e| StoreError::CorruptMedia(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO projects (id, updated_at, data) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET updated_at = excluded.updated_at, data = excluded.data",
            params![project.id, project.updated_at, data],
        )?;
        Ok(())
    }

    /// Lista los proyectos por `updatedAt` descendente, como `db.listProjects`.
    pub fn list_projects(&self) -> Result<Vec<Project>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM projects ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut projects = Vec::new();
        for row in rows {
            let data = row?;
            if let Ok(project) = serde_json::from_str::<Project>(&data) {
                projects.push(project);
            }
        }
        Ok(projects)
    }

    /// Recupera un proyecto por id, como `db.getProject`.
    pub fn get_project(&self, id: &str) -> Result<Option<Project>, StoreError> {
        let data: Option<String> = self
            .conn
            .query_row("SELECT data FROM projects WHERE id = ?1", params![id], |row| row.get(0))
            .optional()?;
        match data {
            Some(data) => Ok(Some(serde_json::from_str(&data).map_err(|e| StoreError::CorruptMedia(e.to_string()))?)),
            None => Ok(None),
        }
    }

    /// Borra un proyecto por id, como `db.deleteProject`.
    pub fn delete_project(&mut self, id: &str) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    /* --------------------------------- Medios ------------------------------- */

    /// Guarda un medio (metadatos + bytes en disco / base de datos), como `db.saveMedia`.
    pub fn save_media(&mut self, record: &MediaRecord) -> Result<(), StoreError> {
        let meta = serde_json::to_string(&record.meta).map_err(|e| StoreError::CorruptMedia(e.to_string()))?;
        let mut file_path_str = String::new();

        if let Some(media_dir) = &self.media_dir {
            let ext = match record.mime.as_str() {
                "video/mp4" => "mp4",
                "video/webm" => "webm",
                "image/png" => "png",
                "image/jpeg" => "jpg",
                "audio/wav" => "wav",
                "audio/mp3" | "audio/mpeg" => "mp3",
                _ => "bin",
            };
            let disk_file = media_dir.join(format!("{}.{}", record.meta.id, ext));
            if !record.bytes.is_empty() {
                let _ = std::fs::write(&disk_file, &record.bytes);
                file_path_str = disk_file.to_string_lossy().to_string();
            }
        }

        self.conn.execute(
            "INSERT INTO media (id, mime, meta, bytes, file_path) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET mime = excluded.mime, meta = excluded.meta, bytes = excluded.bytes, file_path = excluded.file_path",
            params![record.meta.id, record.mime, meta, record.bytes, file_path_str],
        )?;
        Ok(())
    }

    /// Recupera un medio por id, leyendo del archivo en disco o del blob de reserva en SQLite.
    pub fn get_media(&self, id: &str) -> Result<Option<MediaRecord>, StoreError> {
        let row: Option<(String, String, Option<Vec<u8>>, Option<String>)> = self
            .conn
            .query_row(
                "SELECT mime, meta, bytes, file_path FROM media WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some((mime, meta, bytes, file_path)) => {
                let meta = serde_json::from_str(&meta).map_err(|e| StoreError::CorruptMedia(e.to_string()))?;
                let data = if let Some(path) = file_path {
                    if !path.is_empty() && std::path::Path::new(&path).is_file() {
                        std::fs::read(&path).unwrap_or_else(|_| bytes.unwrap_or_default())
                    } else {
                        bytes.unwrap_or_default()
                    }
                } else {
                    bytes.unwrap_or_default()
                };
                Ok(Some(MediaRecord { meta, mime, bytes: data }))
            }
        }
    }

    /// Borra un medio por id de la base de datos y de disco.
    pub fn delete_media(&mut self, id: &str) -> Result<(), StoreError> {
        let file_path: Option<String> = self
            .conn
            .query_row("SELECT file_path FROM media WHERE id = ?1", params![id], |row| row.get(0))
            .optional()?;
        if let Some(path) = file_path {
            if !path.is_empty() {
                let _ = std::fs::remove_file(&path);
            }
        }
        self.conn.execute("DELETE FROM media WHERE id = ?1", params![id])?;
        Ok(())
    }

    /* ------------------------- Archivos de proyecto ------------------------- */

    /// Importa un `.opencup.json`: guarda los proyectos y los medios con bytes.
    ///
    /// Reproduce `importProjectFile` de la app JS, incluido el aviso de medios
    /// que venían sin archivo (`missing`).
    pub fn import_project_file(&mut self, text: &str) -> Result<crate::ImportReport, StoreError> {
        let file = ProjectFile::parse(text).map_err(|e| StoreError::CorruptMedia(e.to_string()))?;
        let (metas, _) = file.split_media();
        let mut missing = 0;
        for entry in &file.media {
            if entry.meta.id.is_empty() {
                continue;
            }
            let Some(data) = entry.data.as_deref() else {
                missing += 1;
                continue;
            };
            let fallback = entry.mime.as_deref().unwrap_or("");
            match decode_data_url(data, fallback) {
                Ok((mime, bytes)) => {
                    self.save_media(&MediaRecord { meta: entry.meta.clone(), mime, bytes })?;
                }
                Err(_) => missing += 1,
            }
        }
        let mut project = file.project.clone();
        project.media = metas;
        project.updated_at = cupcraft_core::now_millis();
        self.save_project(&project)?;
        let mut report = file.import_report();
        report.missing = missing;
        Ok(report)
    }

    /// Serializa un proyecto como archivo `.opencup.json`, incrustando los
    /// bytes de sus medios mientras quepan en el límite (como la app JS).
    pub fn export_project_file(&mut self, project_id: &str) -> Result<ProjectFile, StoreError> {
        let mut project = self
            .get_project(project_id)?
            .ok_or_else(|| StoreError::NotFound(project_id.to_string()))?;
        let mut embedded = 0u64;
        let mut entries = Vec::new();
        for meta in project.media.clone() {
            let record = self.get_media(&meta.id)?;
            let mut entry = crate::MediaEntry { meta: meta.clone(), mime: None, data: None };
            if let Some(record) = record {
                let size = record.bytes.len() as u64;
                if size > 0 && embedded + size <= crate::PROJECT_FILE_EMBED_LIMIT {
                    embedded += size;
                    entry.mime = Some(record.mime.clone());
                    entry.data = Some(record.data_url());
                }
            }
            entries.push(entry);
        }
        project.media.clear();
        Ok(ProjectFile::new(project, entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cupcraft_core::ClipKind;

    fn sample_project() -> Project {
        let mut p = Project::new("Proyecto de prueba");
        p.id = "prj_store".into();
        p.updated_at = 1_700_000_000_000;
        p.media.push(Media {
            id: "m1".into(),
            kind: Some("image".into()),
            name: Some("foto.png".into()),
            duration: None,
            thumb: None,
            extra: Default::default(),
        });
        p.add_media_clip(None, ClipKind::Image, "foto.png", 5.0, None, Some("m1".into()));
        p
    }

    #[test]
    fn save_and_get_project_round_trip() {
        let mut store = Store::open_in_memory().unwrap();
        let p = sample_project();
        store.save_project(&p).unwrap();
        let loaded = store.get_project("prj_store").unwrap().unwrap();
        assert_eq!(loaded.name, "Proyecto de prueba");
        assert_eq!(loaded.tracks[1].clips.len(), 1);
        assert_eq!(loaded.tracks[1].clips[0].media_id.as_deref(), Some("m1"));
        assert!(store.get_project("no-existe").unwrap().is_none());
    }

    #[test]
    fn list_projects_orders_by_updated_at_desc() {
        let mut store = Store::open_in_memory().unwrap();
        let mut a = sample_project();
        a.id = "a".into();
        a.updated_at = 100;
        let mut b = sample_project();
        b.id = "b".into();
        b.updated_at = 300;
        let mut c = sample_project();
        c.id = "c".into();
        c.updated_at = 200;
        store.save_project(&a).unwrap();
        store.save_project(&b).unwrap();
        store.save_project(&c).unwrap();
        // Sobrescribir no duplica.
        store.save_project(&b).unwrap();
        let ids: Vec<String> = store.list_projects().unwrap().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, vec!["b".to_string(), "c".to_string(), "a".to_string()]);
    }

    #[test]
    fn media_put_and_delete_like_indexeddb() {
        let mut store = Store::open_in_memory().unwrap();
        let p = sample_project();
        store
            .save_media(&MediaRecord { meta: p.media[0].clone(), mime: "image/png".into(), bytes: vec![9, 8, 7] })
            .unwrap();
        let got = store.get_media("m1").unwrap().unwrap();
        assert_eq!(got.mime, "image/png");
        assert_eq!(got.bytes, vec![9, 8, 7]);
        assert_eq!(got.data_url(), "data:image/png;base64,CQgH");
        store.delete_media("m1").unwrap();
        assert!(store.get_media("m1").unwrap().is_none());
        assert!(store.get_media("otro").unwrap().is_none());
    }

    #[test]
    fn export_embeds_media_under_the_limit() {
        let mut store = Store::open_in_memory().unwrap();
        let p = sample_project();
        store.save_project(&p).unwrap();
        store
            .save_media(&MediaRecord { meta: p.media[0].clone(), mime: "image/png".into(), bytes: vec![1, 2, 3] })
            .unwrap();
        let file = store.export_project_file("prj_store").unwrap();
        assert_eq!(file.kind, crate::PROJECT_FILE_KIND);
        assert_eq!(file.media.len(), 1);
        assert!(file.media[0].data.is_some(), "los bytes caben en el límite");
        assert!(file.project.media.is_empty(), "el proyecto exportado no lleva medios incrustados");
    }

    #[test]
    fn export_reports_missing_project() {
        let store = Store::open_in_memory().unwrap();
        let mut store = store;
        assert!(matches!(
            store.export_project_file("nope"),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn import_project_file_restores_project_and_media() {
        let mut store = Store::open_in_memory().unwrap();
        let p = sample_project();
        store.save_project(&p).unwrap();
        store
            .save_media(&MediaRecord { meta: p.media[0].clone(), mime: "image/png".into(), bytes: vec![1, 2, 3, 4] })
            .unwrap();
        let text = store.export_project_file("prj_store").unwrap().to_text();

        let mut other = Store::open_in_memory().unwrap();
        let report = other.import_project_file(&text).unwrap();
        assert_eq!(report.id, "prj_store");
        assert_eq!(report.media, 1);
        assert_eq!(report.missing, 0);
        let restored = other.get_project("prj_store").unwrap().unwrap();
        assert_eq!(restored.tracks[1].clips.len(), 1);
        assert_eq!(restored.media.len(), 1, "los metadatos vuelven al proyecto");
        let media = other.get_media("m1").unwrap().unwrap();
        assert_eq!(media.bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn import_counts_media_without_file() {
        let mut store = Store::open_in_memory().unwrap();
        let mut file = {
            let p = sample_project();
            store.save_project(&p).unwrap();
            store
                .save_media(&MediaRecord { meta: p.media[0].clone(), mime: "image/png".into(), bytes: vec![1, 2] })
                .unwrap();
            store.export_project_file("prj_store").unwrap()
        };
        file.media.push(crate::MediaEntry {
            meta: Media {
                id: "m2".into(),
                kind: Some("video".into()),
                name: Some("clip.mp4".into()),
                duration: Some(2.0),
                thumb: None,
                extra: Default::default(),
            },
            mime: None,
            data: None,
        });
        let report = store.import_project_file(&file.to_text()).unwrap();
        assert_eq!(report.media, 2);
        assert_eq!(report.missing, 1);
    }

    #[test]
    fn import_rejects_foreign_files() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(store.import_project_file("{}", ).is_err());
        assert!(store.import_project_file(r#"{"kind":"otra-cosa","project":{"tracks":[]}}"#).is_err());
    }

    #[test]
    fn disk_backed_media_persists_files_and_cleans_up() {
        let tmp = std::env::temp_dir().join(format!("cupcraft_test_{}", cupcraft_core::now_millis()));
        let db_path = tmp.join("cupcraft.db");
        let mut store = Store::open(&db_path).unwrap();

        let media_id = "med_test_disk";
        let record = MediaRecord {
            meta: Media {
                id: media_id.into(),
                kind: Some("video".into()),
                name: Some("clip.mp4".into()),
                duration: Some(10.0),
                thumb: None,
                extra: Default::default(),
            },
            mime: "video/mp4".into(),
            bytes: vec![0, 1, 2, 3, 4, 5],
        };
        store.save_media(&record).unwrap();

        // El medio debe haberse escrito en disco en tmp/media/med_test_disk.mp4
        let expected_file = tmp.join("media").join("med_test_disk.mp4");
        assert!(expected_file.is_file(), "El archivo físico debe existir en disco");

        // Leerlo de vuelta
        let loaded = store.get_media(media_id).unwrap().unwrap();
        assert_eq!(loaded.bytes, vec![0, 1, 2, 3, 4, 5]);

        // Al borrar el medio, también debe eliminarse el archivo de disco
        store.delete_media(media_id).unwrap();
        assert!(!expected_file.exists(), "El archivo debe eliminarse de disco al borrar el medio");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
