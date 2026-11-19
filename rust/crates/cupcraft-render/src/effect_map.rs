/// Mapa de efectos: nombre interno → archivo de shader WGSL.
/// Mantiene la equivalencia con `SHADERS` y `CAPCUT_SHADER_MAP` de gl-post.js.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectId {
    // Post-proceso básico
    RadialBlur,
    DirectionalBlur,
    RgbSplit,
    Chromatic,
    Vhs,
    ZoomPulse,
    Pixelate,
    Mirror,
    Vignette,
    Grain,
    Posterize,
    Wave,
    Bloom,
    // CapCut sin LUT
    BrumaBorgona,
    AtardecerEnsueno,
    BrilloBrumoso,
    PixelNoir,
    CarinoRetro,
    SuenoCaqui,
    JengibreSuave,
    LienzoEmbrujado,
    // CapCut con LUT (512×512 PNG)
    CapcutLut,
    // Curva tonal
    Curve,
}

#[derive(Debug, Clone)]
pub enum FilterKind {
    /// Solo ajustes CSS-equivalentes en WGSL (brightness, contrast, saturate, hue, grayscale, sepia)
    ColorAdjust,
    /// Shader de post-proceso completo con `PostUniforms`
    Shader(EffectId),
    /// LUT 512×512 con shader dedicado
    Lut(EffectId),
}

/// Convierte el nombre de efecto del JSON legado al EffectId correspondiente.
pub fn effect_id_from_name(name: &str) -> Option<EffectId> {
    match name {
        "radial-blur"        => Some(EffectId::RadialBlur),
        "directional-blur"   => Some(EffectId::DirectionalBlur),
        "rgb-split"          => Some(EffectId::RgbSplit),
        "chromatic"          => Some(EffectId::Chromatic),
        "vhs"                => Some(EffectId::Vhs),
        "zoom-pulse"         => Some(EffectId::ZoomPulse),
        "pixelate"           => Some(EffectId::Pixelate),
        "mirror"             => Some(EffectId::Mirror),
        "vignette"           => Some(EffectId::Vignette),
        "grain"              => Some(EffectId::Grain),
        "posterize"          => Some(EffectId::Posterize),
        "wave"               => Some(EffectId::Wave),
        "bloom"              => Some(EffectId::Bloom),
        "bruma-borgona"      => Some(EffectId::BrumaBorgona),
        "atardecer-ensueno"  => Some(EffectId::AtardecerEnsueno),
        "brillo-brumoso"     => Some(EffectId::BrilloBrumoso),
        "pixel-noir"         => Some(EffectId::PixelNoir),
        "carino-retro"       => Some(EffectId::CarinoRetro),
        "sueno-caqui"        => Some(EffectId::SuenoCaqui),
        "jengibre-suave"     => Some(EffectId::JengibreSuave),
        "lienzo-embrujado"   => Some(EffectId::LienzoEmbrujado),
        "capcut-lut"         => Some(EffectId::CapcutLut),
        "curve"              => Some(EffectId::Curve),
        _ => None,
    }
}

/// Convierte el ID de filtro CapCut (u64 como string) al EffectId.
pub fn effect_id_from_capcut(capcut_id: &str) -> Option<EffectId> {
    match capcut_id {
        "7696747334054448391" => Some(EffectId::BrumaBorgona),
        "7694537677269486868" => Some(EffectId::AtardecerEnsueno),
        "7683823805017443602" => Some(EffectId::BrilloBrumoso),
        "7680864395202727186" => Some(EffectId::PixelNoir),
        "7543902649372904765" => Some(EffectId::CarinoRetro),
        "7582494191255702837" => Some(EffectId::SuenoCaqui),
        "7571029701003808005" => Some(EffectId::JengibreSuave),
        "7543902426269519157" => Some(EffectId::LienzoEmbrujado),
        _ => None,
    }
}

/// Devuelve el archivo WGSL correspondiente al EffectId (embedded con include_str!).
pub fn shader_source(id: &EffectId) -> &'static str {
    match id {
        EffectId::RadialBlur        => include_str!("shaders/radial_blur.wgsl"),
        EffectId::DirectionalBlur   => include_str!("shaders/directional_blur.wgsl"),
        EffectId::RgbSplit          => include_str!("shaders/rgb_split.wgsl"),
        EffectId::Chromatic         => include_str!("shaders/chromatic.wgsl"),
        EffectId::Vhs               => include_str!("shaders/vhs.wgsl"),
        EffectId::ZoomPulse         => include_str!("shaders/zoom_pulse.wgsl"),
        EffectId::Pixelate          => include_str!("shaders/pixelate.wgsl"),
        EffectId::Mirror            => include_str!("shaders/mirror.wgsl"),
        EffectId::Vignette          => include_str!("shaders/vignette.wgsl"),
        EffectId::Grain             => include_str!("shaders/grain.wgsl"),
        EffectId::Posterize         => include_str!("shaders/posterize.wgsl"),
        EffectId::Wave              => include_str!("shaders/wave.wgsl"),
        EffectId::Bloom             => include_str!("shaders/bloom.wgsl"),
        EffectId::BrumaBorgona      => include_str!("shaders/bruma_borgona.wgsl"),
        EffectId::AtardecerEnsueno  => include_str!("shaders/atardecer_ensueno.wgsl"),
        EffectId::BrilloBrumoso     => include_str!("shaders/brillo_brumoso.wgsl"),
        EffectId::PixelNoir         => include_str!("shaders/pixel_noir.wgsl"),
        EffectId::CarinoRetro       => include_str!("shaders/carino_retro.wgsl"),
        EffectId::SuenoCaqui        => include_str!("shaders/sueno_caqui.wgsl"),
        EffectId::JengibreSuave     => include_str!("shaders/jengibre_suave.wgsl"),
        EffectId::LienzoEmbrujado   => include_str!("shaders/lienzo_embrujado.wgsl"),
        EffectId::CapcutLut | EffectId::Curve => include_str!("shaders/capcut_lut.wgsl"),
    }
}
