//! Idioma de la interfaz.
//!
//! `tr("texto en español")` devuelve la traducción del idioma activo si existe;
//! si no, devuelve el original. Así lo que aún no está traducido sigue viéndose
//! (en español) en vez de romperse.
//!
//! Idiomas incluidos: archivos `lang/<código>.txt` con una traducción por línea,
//! en el mismo orden que `lang/es.txt` (las claves). Idiomas propios: plantilla
//! JSON que se exporta desde Ajustes, se traduce a mano o con una IA y se vuelve
//! a importar; se guarda en `<carpeta de datos>/langs/<código>.json`.
//! La elección se guarda en `lang.txt` dentro de la carpeta de datos.

use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::RwLock;

/// Claves: los textos originales en español, uno por línea.
const KEYS: &str = include_str!("../lang/es.txt");

/// (código, nombre en su idioma, traducciones alineadas con `KEYS`).
const BUILTIN: &[(&str, &str, &str)] = &[
    ("es", "Español", KEYS),
    ("en", "English", include_str!("../lang/en.txt")),
    ("fr", "Français", include_str!("../lang/fr.txt")),
    ("de", "Deutsch", include_str!("../lang/de.txt")),
    ("pt-BR", "Português (Brasil)", include_str!("../lang/pt-BR.txt")),
    ("ru", "Русский", include_str!("../lang/ru.txt")),
    ("ja", "日本語", include_str!("../lang/ja.txt")),
    ("zh-CN", "简体中文", include_str!("../lang/zh-CN.txt")),
];

#[derive(Clone, Debug, PartialEq)]
pub struct LangInfo {
    pub code: String,
    pub name: String,
    pub builtin: bool,
}

struct State {
    code: String,
    map: Option<HashMap<String, &'static str>>,
}

static ACTIVE: RwLock<State> = RwLock::new(State { code: String::new(), map: None });

fn langs_dir() -> std::path::PathBuf {
    crate::app::app_dir().join("langs")
}

/// Código del idioma activo (`es` si no se ha elegido otro).
pub fn current_code() -> String {
    ACTIVE.read().map(|s| if s.code.is_empty() { "es".to_string() } else { s.code.clone() }).unwrap_or_else(|_| "es".into())
}

pub fn is_english() -> bool {
    current_code() == "en"
}

/// Idiomas disponibles: los incluidos y los importados por el usuario.
pub fn languages() -> Vec<LangInfo> {
    let mut out: Vec<LangInfo> = BUILTIN.iter().map(|(c, n, _)| LangInfo { code: (*c).into(), name: (*n).into(), builtin: true }).collect();
    if let Ok(rd) = std::fs::read_dir(langs_dir()) {
        let mut users: Vec<LangInfo> = rd
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("json") {
                    return None;
                }
                let code = p.file_stem()?.to_str()?.to_string();
                let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
                let name = v.get("name").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).unwrap_or(&code).to_string();
                Some(LangInfo { code, name, builtin: false })
            })
            .collect();
        users.sort_by_key(|a| a.name.to_lowercase());
        out.extend(users);
    }
    out
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn builtin_map(code: &str) -> Option<HashMap<String, &'static str>> {
    let (_, _, pack) = BUILTIN.iter().find(|(c, _, _)| *c == code)?;
    if code == "es" {
        return None;
    }
    Some(KEYS.lines().zip(pack.lines()).filter(|(_, v)| !v.trim().is_empty()).map(|(k, v)| (k.to_string(), leak(v))).collect())
}

fn user_map(code: &str) -> Option<HashMap<String, &'static str>> {
    let text = std::fs::read_to_string(langs_dir().join(format!("{code}.json"))).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    Some(strings_of(&v)?.into_iter().map(|(k, v)| (k, leak(&v))).collect())
}

/// Pares (clave, traducción) no vacíos de una plantilla.
fn strings_of(v: &Value) -> Option<Vec<(String, String)>> {
    let o = v.get("strings")?.as_object()?;
    Some(o.iter().filter_map(|(k, v)| v.as_str().filter(|s| !s.trim().is_empty()).map(|s| (k.clone(), s.to_string()))).collect())
}

fn activate(code: &str) -> bool {
    let builtin = BUILTIN.iter().any(|(c, _, _)| *c == code);
    let map = if builtin { builtin_map(code) } else { user_map(code) };
    if map.is_none() && !builtin {
        return false;
    }
    if let Ok(mut s) = ACTIVE.write() {
        s.code = code.to_string();
        s.map = map;
    }
    true
}

/// Cambia el idioma y lo guarda. `false` si el código no existe.
pub fn set_language(code: &str) -> bool {
    if !activate(code) {
        return false;
    }
    let _ = std::fs::create_dir_all(crate::app::app_dir());
    let _ = std::fs::write(crate::app::app_dir().join("lang.txt"), code);
    true
}

/// Lee el idioma guardado (se llama al arrancar).
pub fn load() {
    let code = std::fs::read_to_string(crate::app::app_dir().join("lang.txt")).map(|s| s.trim().to_string()).unwrap_or_default();
    if code.is_empty() || !activate(&code) {
        activate("es");
    }
}

/// Traduce `es` al idioma activo.
pub fn tr(es: &str) -> Cow<'_, str> {
    if let Ok(s) = ACTIVE.read() {
        if let Some(t) = s.map.as_ref().and_then(|m| m.get(es)) {
            return Cow::Borrowed(*t);
        }
    }
    Cow::Borrowed(es)
}

fn quote(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// Plantilla JSON para traducir: cada texto de la app (clave = original en
/// español, en el orden en que se definieron) con el inglés como punto de partida.
pub fn export_template() -> String {
    let en = BUILTIN.iter().find(|(c, _, _)| *c == "en").map(|(_, _, p)| *p).unwrap_or("");
    let help = "Translate ONLY the values inside \"strings\" into your language: the key on the left is the original Spanish text and must NOT be changed, and the value on the right starts in English. Also set \"code\" (for example \"it\", \"ko\", \"tr\") and \"name\" (the language name written in that language). Keep emoji and symbols (↺ ✨ 🎙 …), punctuation such as … and any placeholders untouched. Leave a value empty to keep the Spanish original. Then import the file from Settings > App language > Import language. / Traduce SOLO los valores de \"strings\"; no cambies las claves. Rellena tambien \"code\" y \"name\".";
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"_instructions\": {},\n", quote(help)));
    out.push_str("  \"code\": \"xx\",\n  \"name\": \"Language name\",\n  \"strings\": {\n");
    let pairs: Vec<(&str, &str)> = KEYS.lines().zip(en.lines()).collect();
    for (i, (k, v)) in pairs.iter().enumerate() {
        out.push_str(&format!("    {}: {}{}\n", quote(k), quote(v), if i + 1 < pairs.len() { "," } else { "" }));
    }
    out.push_str("  }\n}\n");
    out
}

/// Importa una plantilla traducida, la guarda y la activa.
pub fn import_template(text: &str) -> Result<LangInfo, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let pairs = strings_of(&v).ok_or("falta \"strings\"")?;
    if pairs.is_empty() {
        return Err("no hay traducciones".into());
    }
    let raw_code = v.get("code").and_then(Value::as_str).unwrap_or("").trim();
    let mut code: String = raw_code.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(16).collect();
    if code.is_empty() || code.eq_ignore_ascii_case("xx") {
        return Err("falta el campo \"code\" (por ejemplo \"it\")".into());
    }
    if BUILTIN.iter().any(|(c, _, _)| c.eq_ignore_ascii_case(&code)) {
        code.push_str("-custom");
    }
    let name = v.get("name").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty() && *s != "Language name").unwrap_or(&code).to_string();
    let strings: serde_json::Map<String, Value> = pairs.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
    let file = json!({ "code": code, "name": name, "strings": strings });
    std::fs::create_dir_all(langs_dir()).map_err(|e| e.to_string())?;
    std::fs::write(langs_dir().join(format!("{code}.json")), serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if !set_language(&code) {
        return Err("no se pudo activar".into());
    }
    Ok(LangInfo { code, name, builtin: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_paquetes_estan_alineados_con_las_claves() {
        let n = KEYS.lines().count();
        for (code, _, pack) in BUILTIN {
            assert_eq!(pack.lines().count(), n, "{code} no tiene el mismo numero de lineas que es.txt");
        }
    }

    #[test]
    fn traduce_y_vuelve_al_original() {
        assert!(activate("en"));
        assert_eq!(tr("Cancelar"), "Cancel");
        assert_eq!(tr("texto que no existe"), "texto que no existe");
        assert!(activate("es"));
        assert_eq!(tr("Cancelar"), "Cancelar");
    }

    #[test]
    fn la_plantilla_exportada_es_json_valido_y_completa() {
        let v: Value = serde_json::from_str(&export_template()).unwrap();
        assert_eq!(v["strings"].as_object().unwrap().len(), KEYS.lines().count());
        assert_eq!(v["strings"]["Cancelar"], "Cancel");
    }
}
