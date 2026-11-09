//! Núcleo de cupcraft: modelo de proyecto, operaciones de línea de tiempo,
//! historial de deshacer/rehacer y la línea propia de filtros.
//!
//! Este crate no toca disco ni red: es la parte del programa que se puede
//! probar sin abrir la interfaz. La persistencia vive en `cupcraft-persist`.

pub mod filters;
pub mod history;
pub mod model;
pub mod timeline;

pub use filters::{by_key as filter_by_key, Filter, FILTERS};
pub use history::{Editor, History, HISTORY_LIMIT};
pub use model::{new_id, now_millis, Adjust, Clip, ClipKind, Media, Project, Track, TrackType};
pub use timeline::{make_clip, resolve_start, TrimEdge};
