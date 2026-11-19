//! cupcraft-render: Pipeline wgpu para composición de fotogramas y post-proceso GPU.
//!
//! # Responsabilidades
//! - Compositar capas de clip (vídeo, imagen, texto, sticker) en un único fotograma.
//! - Aplicar filtros de post-proceso (shaders WGSL: radial-blur, vhs, bloom, etc.).
//! - Aplicar filtros CapCut con LUT 512×512 o shaders sin LUT (bruma-borgona, etc.).
//! - Renderizar transiciones entre fotogramas.
//! - Devolver el fotograma como imagen RGBA en CPU (para previsualización o exportación).

pub mod uniforms;
pub mod pipeline;
pub mod effect_map;
pub mod transition;
pub mod renderer;

pub use pipeline::{PostPipeline, LayerPipeline, TransitionPipeline};
pub use effect_map::{EffectId, FilterKind, effect_id_from_name};
pub use transition::TransitionKind;
pub use renderer::GpuRenderer;

#[cfg(test)]
mod tests;

