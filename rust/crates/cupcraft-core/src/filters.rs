//! Línea propia de filtros de cupcraft (puerto de `src/filter-line.js`).
//!
//! Todo el catálogo es nuestro: mezclas de color en CSS con valores propios y,
//! en las categorías `Textura` y `Firma`, los shaders del motor GL. No hay
//! ninguna referencia al catálogo de CapCut.

/// Categorías de la línea, en el orden del panel.
pub const CATEGORIES: [&str; 12] = [
    "Esenciales",
    "Cálido",
    "Frío",
    "Blanco y negro",
    "Cine",
    "Analógico",
    "Retrato",
    "Paisaje",
    "Neón",
    "Clima",
    "Textura",
    "Firma",
];

/// Una entrada de la línea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    /// Clave del filtro (se guarda en `clip.filter`).
    pub key: &'static str,
    /// Nombre visible en la tarjeta.
    pub label: &'static str,
    /// Categoría del panel.
    pub category: &'static str,
    /// Mezcla CSS: vista previa de la tarjeta y reserva sin WebGL.
    pub css: &'static str,
    /// Clave del shader GL, si el filtro se pinta con el motor.
    pub gl: Option<&'static str>,
}

/// Constructor constante.
const fn f(
    key: &'static str,
    label: &'static str,
    category: &'static str,
    css: &'static str,
    gl: Option<&'static str>,
) -> Filter {
    Filter { key, label, category, css, gl }
}

/// Filtros con shader GL.
const fn gl(key: &'static str, label: &'static str, category: &'static str, css: &'static str, shader: &'static str) -> Filter {
    Filter { key, label, category, css, gl: Some(shader) }
}

/// Los 82 filtros de la línea, en el mismo orden que los presenta el panel.
pub const FILTERS: [Filter; 82] = [
    // --- Esenciales ---
    f("vivid", "Vívido", "Esenciales", "saturate(1.45) contrast(1.08)", None),
    f("amber", "Ámbar", "Esenciales", "sepia(0.3) saturate(1.2) brightness(1.03)", None),
    f("crystal", "Cristal", "Esenciales", "saturate(0.95) contrast(1.06) brightness(1.06)", None),
    f("sand", "Arena", "Esenciales", "sepia(0.24) saturate(1.05) brightness(1.04) contrast(0.98)", None),
    f("smoke", "Humo", "Esenciales", "saturate(0.7) contrast(1.05) brightness(0.98)", None),
    f("dream", "Ensueño", "Esenciales", "saturate(1.05) brightness(1.12) contrast(0.9) blur(0.3px)", None),
    f("pastel", "Pastel", "Esenciales", "saturate(0.9) brightness(1.1) contrast(0.88)", None),
    // --- Cálido ---
    f("warm", "Cálido", "Cálido", "sepia(0.28) saturate(1.15) hue-rotate(-8deg)", None),
    f("sunset", "Atardecer", "Cálido", "sepia(0.22) saturate(1.4) hue-rotate(-16deg) brightness(1.04)", None),
    f("golden", "Dorado", "Cálido", "sepia(0.38) saturate(1.25) brightness(1.08)", None),
    f("copper", "Cobre", "Cálido", "sepia(0.42) saturate(1.12) contrast(1.06)", None),
    f("honey", "Miel", "Cálido", "sepia(0.45) saturate(1.2) brightness(1.06)", None),
    f("cinnamon", "Canela", "Cálido", "sepia(0.36) hue-rotate(-6deg) contrast(1.05)", None),
    f("dusk", "Poniente", "Cálido", "sepia(0.3) saturate(1.5) hue-rotate(-22deg) brightness(1.02)", None),
    // --- Frío ---
    f("cool", "Frío", "Frío", "saturate(1.1) hue-rotate(12deg) brightness(1.02)", None),
    f("arctic", "Ártico", "Frío", "saturate(0.9) hue-rotate(16deg) brightness(1.1) contrast(0.98)", None),
    f("ocean", "Océano", "Frío", "saturate(1.25) hue-rotate(178deg) brightness(0.98)", None),
    f("teal", "Turquesa", "Frío", "saturate(1.2) hue-rotate(140deg) brightness(1.02)", None),
    f("mint", "Menta", "Frío", "saturate(0.95) hue-rotate(96deg) brightness(1.06)", None),
    f("blizzard", "Ventisca", "Frío", "saturate(0.85) hue-rotate(20deg) brightness(1.1) contrast(1.02)", None),
    f("blueHaze", "Niebla azul", "Frío", "saturate(1.1) hue-rotate(190deg) brightness(1.05) contrast(0.95)", None),
    f("coldShadow", "Sombra fría", "Frío", "hue-rotate(16deg) contrast(1.1) brightness(0.96)", None),
    // --- Blanco y negro ---
    f("mono", "B/N", "Blanco y negro", "grayscale(1) contrast(1.05)", None),
    f("noir", "Noir", "Blanco y negro", "grayscale(1) contrast(1.35) brightness(0.9)", None),
    f("silver", "Plata", "Blanco y negro", "grayscale(1) contrast(0.94) brightness(1.12)", None),
    f("charcoal", "Carbón", "Blanco y negro", "grayscale(1) contrast(1.5) brightness(0.8)", None),
    f("graphite", "Grafito", "Blanco y negro", "grayscale(1) contrast(1.2) brightness(1.02)", None),
    f("deepBlack", "Negro profundo", "Blanco y negro", "grayscale(1) contrast(1.6) brightness(0.85)", None),
    f("snow", "Nieve", "Blanco y negro", "grayscale(1) brightness(1.18) contrast(0.95)", None),
    // --- Cine ---
    f("film", "Película", "Cine", "sepia(0.18) saturate(0.9) contrast(1.12) brightness(0.96)", None),
    f("matte", "Mate", "Cine", "saturate(0.82) contrast(0.88) brightness(1.06)", None),
    f("fade", "Desvanecido", "Cine", "opacity(0.92) saturate(0.75) brightness(1.08)", None),
    f("darkRoom", "Sala oscura", "Cine", "contrast(1.2) saturate(0.9) brightness(0.92)", None),
    f("trailer", "Tráiler", "Cine", "contrast(1.3) saturate(1.15) brightness(0.95)", None),
    f("bleach", "Bleach", "Cine", "contrast(1.15) saturate(0.6) brightness(1.06)", None),
    f("seventies", "Setenta", "Cine", "sepia(0.2) contrast(1.1) saturate(1.1) hue-rotate(-8deg)", None),
    f("anamorphic", "Anamórfico", "Cine", "contrast(1.12) saturate(1.1) hue-rotate(6deg) brightness(1.02)", None),
    // --- Analógico ---
    f("vintage", "Vintage", "Analógico", "sepia(0.5) contrast(0.92) brightness(0.95)", None),
    f("retro", "Retro", "Analógico", "sepia(0.32) saturate(0.85) contrast(0.9) brightness(1.05)", None),
    f("polaroid", "Polaroid", "Analógico", "sepia(0.3) saturate(1.1) contrast(0.92) brightness(1.06)", None),
    f("eightMm", "Ocho milímetros", "Analógico", "sepia(0.4) contrast(1.08) saturate(0.85) brightness(1.02)", None),
    f("reel", "Carrete", "Analógico", "sepia(0.26) contrast(1.05) saturate(0.95)", None),
    f("develop", "Revelado", "Analógico", "contrast(1.1) saturate(1.05) brightness(0.98) sepia(0.12)", None),
    f("underexposed", "Subexpuesto", "Analógico", "brightness(0.9) contrast(1.08) saturate(0.9)", None),
    // --- Retrato ---
    f("rose", "Rosa", "Retrato", "sepia(0.16) saturate(1.2) hue-rotate(324deg) brightness(1.03)", None),
    f("lavender", "Lavanda", "Retrato", "saturate(0.95) hue-rotate(250deg) brightness(1.06)", None),
    f("softSkin", "Piel suave", "Retrato", "saturate(1.08) brightness(1.06) contrast(0.96)", None),
    f("peach", "Melocotón", "Retrato", "sepia(0.2) saturate(1.25) hue-rotate(-10deg) brightness(1.04)", None),
    f("ruby", "Rubí", "Retrato", "saturate(1.3) hue-rotate(-14deg) contrast(1.04)", None),
    f("warmPortrait", "Retrato cálido", "Retrato", "sepia(0.24) saturate(1.12) brightness(1.03)", None),
    // --- Paisaje ---
    f("forest", "Bosque", "Paisaje", "saturate(1.15) hue-rotate(28deg) brightness(0.98)", None),
    f("valley", "Valle", "Paisaje", "saturate(1.2) hue-rotate(12deg) contrast(1.05)", None),
    f("summit", "Cumbre", "Paisaje", "saturate(1.15) brightness(1.06) contrast(1.08)", None),
    f("deepForest", "Bosque profundo", "Paisaje", "saturate(1.25) hue-rotate(30deg) brightness(0.94) contrast(1.06)", None),
    f("desert", "Desierto", "Paisaje", "sepia(0.34) saturate(1.2) brightness(1.04)", None),
    // --- Neón ---
    f("cyber", "Cyber", "Neón", "saturate(1.7) hue-rotate(288deg) contrast(1.18)", None),
    f("infrared", "Infrarrojo", "Neón", "saturate(2) hue-rotate(230deg) contrast(1.1)", None),
    f("magenta", "Magenta", "Neón", "saturate(1.6) hue-rotate(300deg) contrast(1.12)", None),
    f("acid", "Ácido", "Neón", "saturate(1.7) hue-rotate(70deg) contrast(1.1)", None),
    f("plasma", "Plasma", "Neón", "saturate(1.55) hue-rotate(265deg) contrast(1.14) brightness(1.02)", None),
    // --- Clima ---
    f("mist", "Bruma", "Clima", "saturate(0.9) brightness(1.08) contrast(0.94) blur(0.3px)", None),
    f("storm", "Tormenta", "Clima", "saturate(0.8) hue-rotate(196deg) contrast(1.14) brightness(0.9)", None),
    f("drizzle", "Lluvia fina", "Clima", "saturate(0.85) brightness(0.97) contrast(1.05) blur(0.2px)", None),
    f("winterSun", "Sol de invierno", "Clima", "brightness(1.1) saturate(0.92) hue-rotate(10deg) contrast(1.02)", None),
    f("frost", "Escarcha", "Clima", "saturate(0.8) hue-rotate(14deg) brightness(1.12) contrast(0.98)", None),
    // --- Textura (GL) ---
    gl("vignette", "Viñeta", "Textura", "contrast(1.06) brightness(0.96)", "vignette"),
    gl("grain", "Grano", "Textura", "contrast(1.08) saturate(0.96)", "grain"),
    gl("chroma", "Croma", "Textura", "contrast(1.08) saturate(1.12)", "chromatic"),
    gl("vhs", "VHS", "Textura", "saturate(1.18) contrast(1.06) hue-rotate(-4deg)", "vhs"),
    gl("bloom", "Resplandor", "Textura", "brightness(1.1) contrast(0.94) saturate(1.06)", "bloom"),
    gl("rgbSplit", "División RGB", "Textura", "contrast(1.1) saturate(1.15)", "rgb-split"),
    gl("radialBlur", "Desenfoque radial", "Textura", "blur(0.5px) contrast(1.04)", "radial-blur"),
    gl("directionalBlur", "Desenfoque direccional", "Textura", "blur(0.4px)", "directional-blur"),
    gl("posterize", "Posterizar", "Textura", "contrast(1.3) saturate(1.35)", "posterize"),
    // --- Firma (GL portado) ---
    gl("burgundyHaze", "Bruma borgoña", "Firma", "sepia(0.35) hue-rotate(-18deg) saturate(1.3) contrast(1.05)", "bruma-borgona"),
    gl("dreamSunset", "Atardecer de ensueño", "Firma", "sepia(0.3) saturate(1.45) hue-rotate(-14deg) brightness(1.05)", "atardecer-ensueno"),
    gl("mistyGlow", "Brillo brumoso", "Firma", "brightness(1.14) contrast(0.92) saturate(1.1)", "brillo-brumoso"),
    gl("pixelNoir", "Píxel noir", "Firma", "grayscale(1) contrast(1.35) brightness(0.95)", "pixel-noir"),
    gl("retroLove", "Cariño retro", "Firma", "sepia(0.34) saturate(1.1) contrast(1.06) brightness(1.02)", "carino-retro"),
    gl("khakiDream", "Sueño caqui", "Firma", "sepia(0.42) saturate(0.95) brightness(1.06) contrast(0.96)", "sueno-caqui"),
    gl("gingerSoft", "Jengibre suave", "Firma", "sepia(0.3) saturate(1.2) hue-rotate(-8deg) brightness(1.04)", "jengibre-suave"),
    gl("hauntedCanvas", "Lienzo embrujado", "Firma", "saturate(1.25) hue-rotate(255deg) contrast(1.12) brightness(0.96)", "lienzo-embrujado"),
];

/// Busca un filtro por su clave.
pub fn by_key(key: &str) -> Option<&'static Filter> {
    FILTERS.iter().find(|filter| filter.key == key)
}

/// ¿Existe la clave en la línea? (útil para validar `clip.filter`.)
pub fn is_valid(key: &str) -> bool {
    key == "none" || by_key(key).is_some()
}

/// Filtra por categoría y/o texto de búsqueda, como el panel.
pub fn search<'a>(category: Option<&str>, query: Option<&str>) -> Vec<&'a Filter> {
    let query = query.map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty());
    FILTERS
        .iter()
        .filter(|filter| match category {
            None | Some("Todos") => true,
            Some(cat) => filter.category == cat,
        })
        .filter(|filter| match &query {
            None => true,
            Some(q) => filter.label.to_lowercase().contains(q.as_str()),
        })
        .collect()
}

/// Clave de la línea que corresponde a un clip (para resaltar la tarjeta).
///
/// Los filtros GL viven en `clip.glFilter` y los de color en `clip.filter`;
/// los LUT heredados de proyectos antiguos no tienen equivalente propio.
pub fn line_key_for_clip(
    filter: &str,
    gl_filter: Option<&str>,
    lut: Option<&str>,
) -> Option<&'static str> {
    if let Some(shader) = gl_filter {
        return FILTERS.iter().find(|f| f.gl == Some(shader)).map(|f| f.key);
    }
    if lut.is_some() {
        return None;
    }
    if filter.is_empty() || filter == "none" {
        return None;
    }
    by_key(filter).map(|f| f.key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_82_filters_in_12_categories() {
        assert_eq!(FILTERS.len(), 82);
        assert_eq!(CATEGORIES.len(), 12);
    }

    #[test]
    fn every_filter_belongs_to_a_listed_category() {
        for filter in FILTERS {
            assert!(
                CATEGORIES.contains(&filter.category),
                "{} tiene categoría desconocida {}",
                filter.key,
                filter.category
            );
            assert!(!filter.label.is_empty(), "{} sin nombre", filter.key);
            assert!(!filter.css.is_empty(), "{} sin css", filter.key);
        }
    }

    #[test]
    fn keys_and_labels_are_unique() {
        for (i, a) in FILTERS.iter().enumerate() {
            for b in FILTERS.iter().skip(i + 1) {
                assert_ne!(a.key, b.key, "clave repetida: {}", a.key);
                assert_ne!(a.label, b.label, "nombre repetido: {}", a.label);
            }
        }
    }

    #[test]
    fn seventeen_filters_use_the_gl_engine() {
        let count = FILTERS.iter().filter(|f| f.gl.is_some()).count();
        assert_eq!(count, 17);
    }

    #[test]
    fn texture_and_signature_categories_are_all_gl() {
        for filter in FILTERS.iter().filter(|f| f.category == "Textura" || f.category == "Firma") {
            assert!(filter.gl.is_some(), "{} debería ser GL", filter.key);
        }
    }

    #[test]
    fn the_js_keys_are_all_here() {
        // Claves que la app JS usa en proyectos existentes.
        for key in ["vivid", "warm", "mint", "mono", "noir", "vintage", "cyber", "forest", "burgundyHaze"] {
            assert!(is_valid(key), "falta {key}");
        }
        assert!(is_valid("none"));
        assert!(!is_valid("calabaza"));
    }

    #[test]
    fn search_filters_by_category_and_text() {
        assert_eq!(search(Some("Firma"), None).len(), 8);
        assert_eq!(search(Some("Todos"), None).len(), 82);
        assert_eq!(search(None, Some("bor")).len(), 1); // Bruma borgoña
        assert_eq!(search(None, Some("BRUMA")).len(), 2); // "Bruma" (Clima) y "Bruma borgoña"
        assert_eq!(search(None, Some("nOIr")).len(), 2); // case-insensitive: "Noir" y "Píxel noir"
        assert!(search(Some("Neón"), Some("bruma")).is_empty());
    }

    #[test]
    fn line_key_resolves_gl_and_color_clips() {
        assert_eq!(line_key_for_clip("none", Some("bruma-borgona"), None), Some("burgundyHaze"));
        assert_eq!(line_key_for_clip("vivid", None, None), Some("vivid"));
        assert_eq!(line_key_for_clip("none", None, Some("assets/luts/x.png")), None);
        assert_eq!(line_key_for_clip("none", None, None), None);
        // Un shader que no esté en la línea no resuelve.
        assert_eq!(line_key_for_clip("none", Some("capcut-lut"), None), None);
    }
}
