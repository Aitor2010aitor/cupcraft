//! Servicios nativos que sustituyen a las APIs del navegador de la versión web:
//!
//! * `media`   — importación, caché en disco (`%LOCALAPPDATA%\openCUP\media`),
//!   sondeo con ffprobe y decodificación de audio (sustituye a
//!   `<input type=file>`, `URL.createObjectURL` y Web Audio).
//! * `video`   — decodificación de fotogramas con FFmpeg en un hilo de fondo
//!   (sustituye a los elementos `<video>`/`<img>` del MediaPool).
//! * `compose` — compositor CPU de fotogramas para exportar (sustituye al
//!   `<canvas>` 2D + `MediaRecorder` de `export.js`).
//! * `raster`  — rasterizado de texto/stickers con ab_glyph y fuentes de Windows.
//! * `export`  — hilo de exportación con progreso y cancelación.

pub mod ai_local;
pub mod compose;
pub mod preview_comp;
pub mod export;
pub mod gif_anim;
pub mod gpu;
pub mod matte;
pub mod media;
pub mod raster;
pub mod updater;
pub mod video;
pub mod whisper;
