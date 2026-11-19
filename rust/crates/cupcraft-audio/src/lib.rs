//! cupcraft-audio: Motor de reproducción, decodificación y mezcla de audio en tiempo real.

pub mod buffer;
pub mod decoder;
pub mod waveform;
pub mod mixer;
pub mod engine;

pub use buffer::AudioBuffer;
pub use decoder::decode_audio_file;
pub use waveform::Waveform;
pub use mixer::AudioMixer;
pub use engine::AudioEngine;

#[cfg(test)]
mod tests;

