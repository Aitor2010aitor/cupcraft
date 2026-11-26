//! Persistencia de cupcraft: el archivo `.opencup.json` y el almacén local
//! (proyectos y medios) que en la app JS vive en IndexedDB (`src/db.js`).
//!
//! El almacén usa SQLite para que la biblioteca sea consultable y no pierda
//! datos si la app se cierra a mitad de un guardado.

pub mod migration;
pub mod pack;
pub mod project_file;
pub mod store;
pub mod v8_deserializer;

pub use migration::{detect_legacy_roots, migrate_from_indexeddb, MigrationReport};
pub use pack::{read_zip, write_zip, PackError, CUPCRAFT_EXT, ZIP_MAGIC};
pub use project_file::{
    decode_data_url, encode_data_url, export_file_name, export_file_name_cupcraft, ImportReport, MediaEntry, ProjectFile,
    ProjectFileError, PROJECT_FILE_EMBED_LIMIT, PROJECT_FILE_KIND, PROJECT_FILE_VERSION,
};
pub use store::{MediaRecord, Store};
// Modelos reexportados para que los consumidores no dependan de dos crates.
pub use cupcraft_core::{Clip, ClipKind, Media, Project, Track, TrackType};
