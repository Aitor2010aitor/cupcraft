//! Fuentes instaladas en Windows.
//!
//! cupcraft no instala nada: solo enumera las fuentes que ya están en el
//! equipo. Se leen las dos carpetas estándar de Windows (fuentes del sistema y
//! fuentes instaladas por el usuario) y el nombre real de la familia se saca
//! de la tabla `name` del propio archivo, porque el nombre del archivo no
//! siempre coincide con el nombre que usa Windows.
//!
//! `GET /api/system-fonts` devuelve la lista y
//! `GET /api/system-fonts/file?name=...` sirve un archivo concreto, siempre
//! que esté dentro de una carpeta de fuentes y tenga una extensión válida.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use ttf_parser::{name_id, Face, PlatformId};

/// Una fuente detectada en el sistema.
#[derive(Clone)]
pub struct SystemFont {
    pub family: String,
    pub file: Option<String>,
    pub path: PathBuf,
    pub source: &'static str,
    regular: bool,
}

#[derive(Serialize)]
pub struct FontEntry {
    pub family: String,
    pub label: String,
    pub file: Option<String>,
    pub source: &'static str,
}

fn font_dirs() -> Vec<(PathBuf, &'static str)> {
    let mut dirs = Vec::new();
    let windir = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("WINDIR"))
        .unwrap_or_else(|_| "C:\\Windows".to_string());
    dirs.push((PathBuf::from(windir).join("Fonts"), "windows"));
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push((
            PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
            "user",
        ));
    }
    dirs
}

fn is_font_file(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(extension.as_str(), "ttf" | "otf" | "ttc" | "otc")
}

fn pick_name(face: &Face, id: u16) -> Option<String> {
    let mut best: Option<(u8, String)> = None;
    for name in face.names() {
        if name.name_id != id {
            continue;
        }
        let Some(value) = name.to_string() else {
            continue;
        };
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        let score = match (name.platform_id, name.language_id) {
            (PlatformId::Windows, 0x0409) => 3,
            (PlatformId::Windows, _) => 2,
            _ => 1,
        };
        if best.as_ref().map(|(current, _)| score > *current).unwrap_or(true) {
            best = Some((score, value));
        }
    }
    best.map(|(_, value)| value)
}

fn face_names(data: &[u8], index: u32) -> Option<(String, String)> {
    let face = Face::parse(data, index).ok()?;
    let family = pick_name(&face, name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| pick_name(&face, name_id::FAMILY))?;
    if family.is_empty() {
        return None;
    }
    let subfamily = pick_name(&face, name_id::TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| pick_name(&face, name_id::SUBFAMILY))
        .unwrap_or_default();
    Some((family, subfamily))
}

fn is_regular(subfamily: &str) -> bool {
    let value = subfamily.trim().to_ascii_lowercase();
    value.is_empty()
        || value == "regular"
        || value == "normal"
        || value == "book"
        || value == "roman"
        || value == "standard"
        || value == "plain"
}

fn scan() -> Vec<SystemFont> {
    let mut found: BTreeMap<String, SystemFont> = BTreeMap::new();
    for (dir, source) in font_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && is_font_file(path))
            .collect();
        paths.sort();
        for path in paths {
            let Some(file_name) = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_string())
            else {
                continue;
            };
            let Ok(data) = std::fs::read(&path) else {
                continue;
            };
            let count = ttf_parser::fonts_in_collection(&data)
                .unwrap_or(1)
                .min(64);
            for index in 0..count {
                let Some((family, subfamily)) = face_names(&data, index) else {
                    continue;
                };
                let key = family.to_lowercase();
                let regular = is_regular(&subfamily);
                // Si ya existe la familia, solo se sustituye cuando la nueva
                // cara es la regular (evita quedarse con "Arial Bold").
                if let Some(existing) = found.get(&key) {
                    if existing.regular || !regular {
                        continue;
                    }
                }
                found.insert(
                    key,
                    SystemFont {
                        family: family.clone(),
                        file: if index == 0 {
                            Some(file_name.clone())
                        } else {
                            // En una colección .ttc el navegador no puede
                            // elegir la cara por índice desde una URL; esa
                            // familia se sigue usando por su nombre nativo.
                            None
                        },
                        path: path.clone(),
                        source,
                        regular,
                    },
                );
            }
        }
    }
    let mut list: Vec<SystemFont> = found.into_values().collect();
    list.sort_by(|a, b| {
        a.family
            .to_lowercase()
            .cmp(&b.family.to_lowercase())
    });
    list
}

static CACHE: OnceLock<Mutex<Option<Vec<SystemFont>>>> = OnceLock::new();

/// Lista de fuentes detectadas. `refresh` ignora la caché en memoria.
pub fn list(refresh: bool) -> Vec<SystemFont> {
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if !refresh {
        if let Ok(guard) = cache.lock() {
            if let Some(fonts) = guard.as_ref() {
                return fonts.clone();
            }
        }
    }
    let fonts = scan();
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(fonts.clone());
    }
    fonts
}

pub fn entries(fonts: &[SystemFont]) -> Vec<FontEntry> {
    fonts
        .iter()
        .map(|font| FontEntry {
            family: font.family.clone(),
            label: font.family.clone(),
            file: font.file.clone(),
            source: font.source,
        })
        .collect()
}

/// Resuelve el archivo de una fuente a partir de su nombre, sin permitir
/// rutas ni archivos fuera de las carpetas de fuentes.
pub fn find_file(name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains(':')
    {
        return None;
    }
    let candidate = PathBuf::from(name);
    if !is_font_file(&candidate) {
        return None;
    }
    for (dir, _) in font_dirs() {
        let path = dir.join(name);
        let Ok(canonical) = std::fs::canonicalize(&path) else {
            continue;
        };
        let Ok(root) = std::fs::canonicalize(&dir) else {
            continue;
        };
        if canonical.starts_with(&root) && canonical.is_file() {
            return Some(path);
        }
    }
    None
}
