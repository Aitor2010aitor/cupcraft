//! cupcraft-media: Integración con FFmpeg, decodificación de vídeo, thumbnails y exportación acelerada.

pub mod ffmpeg;
pub mod probe;
pub mod extractor;
pub mod exporter;

pub use ffmpeg::{FfmpegLocation, HwAccel};
pub use probe::{probe_media, MediaInfo, ProbeError};
pub use extractor::{extract_frame, extract_filmstrip};
pub use exporter::{VideoEncoderSession, ExportFormat, mux_audio_video};

#[cfg(test)]
mod tests;
