//! Catálogos de texto y ajustes, cargados desde el JSON generado a partir de
//! `src/text-catalog.js` y `src/library.js`.

use serde_json::Value;
use std::sync::OnceLock;

static DATA: OnceLock<Value> = OnceLock::new();

fn data() -> &'static Value {
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("text_styles_data.json")).expect("text_styles_data.json")
    })
}

fn arr(key: &str) -> &'static [Value] {
    data().get(key).and_then(|v| v.as_array()).map(|v| v.as_slice()).unwrap_or(&[])
}

pub fn text_styles() -> &'static [Value] { arr("text_styles") }
pub fn text_bubbles() -> &'static [Value] { arr("text_bubbles") }
pub fn text_effects() -> &'static [Value] { arr("text_effects") }
pub fn text_animations() -> &'static [Value] { arr("text_animations") }
pub fn text_fonts() -> &'static [Value] { arr("text_fonts") }
pub fn text_cases() -> &'static [Value] { arr("text_cases") }
pub fn text_aligns() -> &'static [Value] { arr("text_aligns") }
pub fn text_weights() -> &'static [Value] { arr("text_weights") }
pub fn adjust_presets() -> &'static [Value] { arr("adjust_presets") }
pub fn adjust_categories() -> &'static [Value] { arr("adjust_categories") }
pub fn caption_styles() -> &'static [Value] { arr("caption_styles") }
pub fn avatar_presets() -> &'static [Value] { arr("avatar_presets") }
