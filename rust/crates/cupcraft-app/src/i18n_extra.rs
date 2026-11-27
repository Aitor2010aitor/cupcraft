//! Respaldo en inglés de los textos que no están en `lang/*.txt`.

pub fn all() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    crate::i18n_panels::DICT
        .iter()
        .chain(crate::i18n_catalog::DICT.iter())
        .chain(crate::i18n_ui1::DICT.iter())
        .chain(crate::i18n_ui2::DICT.iter())
}
