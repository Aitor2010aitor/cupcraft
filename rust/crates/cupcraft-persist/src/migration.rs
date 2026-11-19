//! Migrador de proyectos antiguos desde IndexedDB (WebView2 / Chromium) a SQLite nativo.
//!
//! Lee los almacenes de WebView2 sin necesitar el navegador abierto y traslada los
//! proyectos y archivos de medios al nuevo formato a prueba de cierres bruscos.

use crate::store::{MediaRecord, Store, StoreError};
use crate::v8_deserializer::parse_v8;
use cupcraft_core::Project;
use std::path::{Path, PathBuf};

/// Informe del proceso de migración de datos heredados.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MigrationReport {
    pub projects_migrated: usize,
    pub media_migrated: usize,
    pub bytes_migrated: u64,
    pub roots_scanned: Vec<PathBuf>,
}

/// Detecta posibles rutas de IndexedDB de la aplicación anterior en Windows.
pub fn detect_legacy_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let local_app_data = match std::env::var("LOCALAPPDATA") {
        Ok(v) => PathBuf::from(v),
        Err(_) => return roots,
    };

    let candidates = [
        local_app_data.join("cupcraft").join("WebView2").join("EBWebView").join("Default").join("IndexedDB"),
        local_app_data.join("openCUP").join("WebView2").join("EBWebView").join("Default").join("IndexedDB"),
    ];

    for candidate in candidates {
        if candidate.is_dir() {
            roots.push(candidate);
        }
    }
    roots
}

/// Migra los datos de una carpeta raíz de IndexedDB hacia el `Store` nativo.
pub fn migrate_from_indexeddb(legacy_indexeddb_root: &Path, store: &mut Store) -> Result<MigrationReport, StoreError> {
    let mut report = MigrationReport {
        roots_scanned: vec![legacy_indexeddb_root.to_path_buf()],
        ..Default::default()
    };

    let Ok(entries) = std::fs::read_dir(legacy_indexeddb_root) else {
        return Ok(report);
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Examinar carpetas .blob de IndexedDB (ej. http_127.0.0.1_8787.indexeddb.blob)
        if name.ends_with(".indexeddb.blob") && path.is_dir() {
            migrate_blob_dir(&path, store, &mut report)?;
        }
    }

    Ok(report)
}

fn migrate_blob_dir(blob_dir: &Path, store: &mut Store, report: &mut MigrationReport) -> Result<(), StoreError> {
    // Recorrer recursivamente todos los archivos dentro del almacén de blobs
    let mut stack = vec![blob_dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };

                // Comprobar si es un objeto V8 (Proyecto o Medio serializado)
                if bytes.starts_with(&[0xFF, 0x11]) || bytes.windows(2).any(|w| w == [0xFF, 0x10]) {
                    if let Some(val) = parse_v8(&bytes) {
                        // Es un objeto JSON válido
                        if let Ok(project) = serde_json::from_value::<Project>(val.clone()) {
                            if !project.id.is_empty() && !project.name.is_empty() {
                                store.save_project(&project)?;
                                report.projects_migrated += 1;
                                continue;
                            }
                        }
                    }
                }

                // Si es un archivo multimedia binario (MP4, QuickTime, WebM, PNG, JPEG, etc.)
                let mime = detect_mime(&bytes);
                if !mime.is_empty() {
                    let file_id = path.file_name().and_then(|n| n.to_str()).unwrap_or("blob");
                    let media_id = format!("med_{file_id}");
                    let len = bytes.len() as u64;
                    let meta = cupcraft_core::Media {
                        id: media_id.clone(),
                        kind: Some(if mime.starts_with("video") { "video" } else if mime.starts_with("audio") { "audio" } else { "image" }.to_string()),
                        name: Some(format!("Media_{file_id}")),
                        duration: None,
                        thumb: None,
                        extra: Default::default(),
                    };
                    store.save_media(&MediaRecord {
                        meta,
                        mime: mime.to_string(),
                        bytes,
                    })?;
                    report.media_migrated += 1;
                    report.bytes_migrated += len;
                }
            }
        }
    }
    Ok(())
}

fn detect_mime(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return "video/mp4";
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return "video/webm";
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return "image/png";
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }
    if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WAVE" {
        return "audio/wav";
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_legacy_roots() {
        let roots = detect_legacy_roots();
        for r in roots {
            assert!(r.exists());
        }
    }

    #[test]
    fn test_detect_mime() {
        let mp4_header = [0, 0, 0, 0x20, b'f', b't', b'y', b'p', b'i', b's', b'o', b'4'];
        assert_eq!(detect_mime(&mp4_header), "video/mp4");
        let png_header = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_mime(&png_header), "image/png");
    }

    #[test]
    fn test_migration_pipeline() {
        let tmp = std::env::temp_dir().join(format!("cupcraft_mig_{}", cupcraft_core::now_millis()));
        let fake_idb = tmp.join("Default").join("IndexedDB");
        let blob_dir = fake_idb.join("http_127.0.0.1_8787.indexeddb.blob").join("1").join("00");
        std::fs::create_dir_all(&blob_dir).unwrap();

        // Crear medio simulado
        let mut fake_mp4 = vec![0, 0, 0, 0x20, b'f', b't', b'y', b'p', b'i', b's', b'o', b'4'];
        fake_mp4.extend_from_slice(&[1, 2, 3, 4]);
        std::fs::write(blob_dir.join("2"), fake_mp4).unwrap();

        let mut store = Store::open_in_memory().unwrap();
        let report = migrate_from_indexeddb(&fake_idb, &mut store).unwrap();
        assert_eq!(report.media_migrated, 1);
        assert!(report.bytes_migrated > 0);

        let loaded = store.get_media("med_2").unwrap().unwrap();
        assert_eq!(loaded.mime, "video/mp4");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
