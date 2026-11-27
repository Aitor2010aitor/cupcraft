//! Idioma de la interfaz (español / inglés).
//!
//! `tr("texto en español")` devuelve la traducción si el idioma activo es el
//! inglés y el texto está en el diccionario; si no, devuelve el original. Así
//! las cadenas sin traducir siguen viéndose (en español) en vez de romperse.
//! La elección se guarda en `lang.txt` dentro de la carpeta de datos.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn is_english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
    let _ = std::fs::create_dir_all(crate::app::app_dir());
    let _ = std::fs::write(crate::app::app_dir().join("lang.txt"), if on { "en" } else { "es" });
}

/// Lee el idioma guardado (se llama al arrancar).
pub fn load() {
    let on = std::fs::read_to_string(crate::app::app_dir().join("lang.txt")).map(|s| s.trim() == "en").unwrap_or(false);
    ENGLISH.store(on, Ordering::Relaxed);
}

/// Traduce `es` al idioma activo.
pub fn tr(es: &str) -> Cow<'_, str> {
    if !is_english() {
        return Cow::Borrowed(es);
    }
    DICT.iter().find(|(k, _)| *k == es).map(|(_, v)| Cow::Borrowed(*v)).unwrap_or(Cow::Borrowed(es))
}

const DICT: &[(&str, &str)] = &[
    ("Archivos", "Files"),
    ("Archivos multimedia", "Media files"),
    ("Filtros", "Filters"),
    ("Plantillas", "Templates"),
    ("Subtít.", "Subs"),
    ("Subtítulos", "Subtitles"),
    ("Transic.", "Trans."),
    ("Transiciones", "Transitions"),
    ("Ajuste", "Adjust"),
    ("Stickers", "Stickers"),
    ("Texto a voz (Beta)", "Text to speech (Beta)"),
    ("En pruebas: puede fallar o sonar raro.", "Testing: it may fail or sound odd."),
    ("+ Añadir texto predeterminado", "+ Add default text"),
    ("+ Importar", "+ Import"),
    ("+ Importar audio", "+ Import audio"),
    ("Abrir", "Open"),
    ("Abrir carpeta", "Open folder"),
    ("Abrir carpeta de datos", "Open data folder"),
    ("Abrir proyecto desde archivo", "Open project from file"),
    ("Adelgazar", "Slim"),
    ("Aislar voz", "Isolate voice"),
    ("Ajustar", "Adjust"),
    ("Ajuste manual", "Manual adjustment"),
    ("Ajustes", "Settings"),
    ("Almacenamiento", "Storage"),
    ("Animación", "Animation"),
    ("Aplicar a todo", "Apply to all"),
    ("Arrastra los clips para moverlos, usa los tiradores de los extremos para recortar y pulsa B para dividir en el cursor.", "Drag clips to move them, use the end handles to trim and press B to split at the cursor."),
    ("Atajos", "Shortcuts"),
    ("Atajos de teclado", "Keyboard shortcuts"),
    ("Atrás", "Back"),
    ("Audio", "Audio"),
    ("Audio del proyecto", "Project audio"),
    ("Añadir subtítulos", "Add subtitles"),
    ("Aún no hay audio. Importa música o efectos de sonido desde tu equipo.", "No audio yet. Import music or sound effects from your computer."),
    ("Biblioteca de efectos", "Effects library"),
    ("Buscar audio", "Search audio"),
    ("Buscar categorías de stickers", "Search sticker categories"),
    ("Buscar efectos", "Search effects"),
    ("Buscar en tus medios", "Search your media"),
    ("Buscar estilos", "Search styles"),
    ("Buscar filtros", "Search filters"),
    ("Buscar transiciones", "Search transitions"),
    ("Básico", "Basic"),
    ("Cambia el tono como en los vídeos de TikTok (ardilla, grave, robot…). Se aplica con FFmpeg, sin internet.", "Change the pitch like in TikTok videos (chipmunk, deep, robot…). Applied with FFmpeg, offline."),
    ("Cancelar", "Cancel"),
    ("Cerrar", "Close"),
    ("Color del resalte", "Highlight color"),
    ("Completar canal", "Fill channel"),
    ("Consejo: mantén Alt al arrastrar un clip para desactivar temporalmente el imán.", "Tip: hold Alt while dragging a clip to temporarily disable snapping."),
    ("Contenido", "Content"),
    ("Continuar", "Continue"),
    ("Copiar", "Copy"),
    ("Decidir más tarde", "Decide later"),
    ("Descargar .txt", "Download .txt"),
    ("Descargar motor y modelo", "Download engine and model"),
    ("Descartar", "Discard"),
    ("Dientes", "Teeth"),
    ("Efecto de los subtítulos", "Subtitle effect"),
    ("Efecto karaoke (estilo TikTok)", "Karaoke effect (TikTok style)"),
    ("Efectos", "Effects"),
    ("Efectos de video", "Video effects"),
    ("Efectos que se aplican al texto seleccionado.", "Effects applied to the selected text."),
    ("El efecto se añade como capa en el cursor y afecta a todo lo que hay debajo.", "The effect is added as a layer at the cursor and affects everything below it."),
    ("El filtro de voz se aplica al audio del clip en la vista previa y en la exportación.", "The voice filter is applied to the clip audio in the preview and in the export."),
    ("El párrafo se ve entero y la palabra que se dice cambia de color a medida que avanza.", "The whole paragraph is visible and the spoken word changes color as it goes."),
    ("Elige el color y, luego, ajusta la intensidad.", "Pick the color, then adjust the intensity."),
    ("Elige una burbuja: se aplica al texto seleccionado.", "Pick a bubble: it is applied to the selected text."),
    ("Entendido", "Got it"),
    ("Este efecto no tiene propiedades configurables.", "This effect has no configurable properties."),
    ("Estilo", "Style"),
    ("Estilo CapCut", "CapCut style"),
    ("Estilo de voz", "Voice style"),
    ("Estilo preestablecido", "Preset style"),
    ("Estándar", "Standard"),
    ("Exportar", "Export"),
    ("Exportar video", "Export video"),
    ("Generando…", "Generating…"),
    ("Genial", "Great"),
    ("Grabar voz en off", "Record voiceover"),
    ("Guardar", "Save"),
    ("Guardar como ajuste", "Save as preset"),
    ("Guardar proyecto como archivo", "Save project as file"),
    ("Guardar y salir", "Save and quit"),
    ("Hay que descargar el motor y el modelo una sola vez (~140 MB).", "The engine and model must be downloaded once (~140 MB)."),
    ("Hora en UTC.", "Time in UTC."),
    ("Idioma", "Language"),
    ("Idioma de la app", "App language"),
    ("Idioma de la voz", "Voice language"),
    ("Importar SRT/VTT", "Import SRT/VTT"),
    ("Integraciones", "Integrations"),
    ("La exportación anterior se canceló.", "The previous export was cancelled."),
    ("La grabación se inserta en el cursor como un clip de audio.", "The recording is inserted at the cursor as an audio clip."),
    ("Las curvas de velocidad marcan aceleraciones progresivas dentro del clip.", "Speed curves create gradual accelerations inside the clip."),
    ("Listo", "Done"),
    ("Local · sin cuenta", "Local · no account"),
    ("Los ajustes de color se aplican a clips de video o imagen.", "Color adjustments apply to video or image clips."),
    ("Los efectos se aplican a clips de video, imagen, texto o sticker.", "Effects apply to video, image, text or sticker clips."),
    ("Los proyectos y medios se guardan en %LOCALAPPDATA%\\openCUP. La exportación usa FFmpeg si está disponible en el equipo.", "Projects and media are stored in %LOCALAPPDATA%\\openCUP. Export uses FFmpeg if it is available on this computer."),
    ("Los proyectos, medios y ajustes se guardan en %LOCALAPPDATA%\\openCUP. No se necesitan permisos de administrador.", "Projects, media and settings are stored in %LOCALAPPDATA%\\openCUP. No administrator permissions are needed."),
    ("Mejorar voz", "Enhance voice"),
    ("Modo", "Mode"),
    ("Más voces", "More voices"),
    ("No hay entradas.", "No entries."),
    ("No hay notificaciones nuevas. cupcraft funciona sin conexión.", "No new notifications. cupcraft works offline."),
    ("No hay otros proyectos todavía. Crea uno nuevo desde el menú o vuelve al inicio.", "There are no other projects yet. Create a new one from the menu or go back home."),
    ("No hay voces instaladas. Descarga una en «Más voces», abajo.", "No voices installed. Download one in “More voices”, below."),
    ("Ojos", "Eyes"),
    ("Pistas", "Tracks"),
    ("Preferencias", "Preferences"),
    ("Procesando…", "Processing…"),
    ("Propiedades", "Properties"),
    ("Proyecto de CapCut", "CapCut project"),
    ("Proyectos", "Projects"),
    ("Proyectos secundarios", "Secondary projects"),
    ("Puente IA", "AI bridge"),
    ("Quitar filtro", "Remove filter"),
    ("Quitar fondo", "Remove background"),
    ("Quitar todos los subtítulos", "Remove all subtitles"),
    ("Quitar transición", "Remove transition"),
    ("Recortar con una máscara", "Crop with a mask"),
    ("Reducir el ruido", "Reduce noise"),
    ("Reportar en GitHub", "Report on GitHub"),
    ("Reportar problema", "Report a problem"),
    ("Restablecer", "Reset"),
    ("Retoque", "Retouch"),
    ("Se usa Piper, un motor local (~25 MB + una voz). No se envía nada a internet. Se descarga una sola vez.", "Piper is used, a local engine (~25 MB + one voice). Nothing is sent to the internet. Downloaded only once."),
    ("Selecciona un clip con audio para ajustar su volumen.", "Select a clip with audio to adjust its volume."),
    ("Selecciona un clip de video o imagen para ajustarlo.", "Select a video or image clip to adjust it."),
    ("Si eliges «No», el proyecto no se guarda y se pierden los cambios de esta sesión.", "If you choose “No”, the project is not saved and this session's changes are lost."),
    ("Si se desactiva, el audio mantiene su tono al cambiar la velocidad.", "When off, the audio keeps its pitch when the speed changes."),
    ("Sin animación", "No animation"),
    ("Subtítulos automáticos", "Automatic subtitles"),
    ("Subtítulos manuales", "Manual subtitles"),
    ("Sí, guardar", "Yes, save"),
    ("Tema", "Theme"),
    ("Texto", "Text"),
    ("Texto del clip", "Clip text"),
    ("Tipo de seguimiento", "Tracking type"),
    ("Tipografía", "Typography"),
    ("Todas las funciones de esta versión están disponibles sin suscripción. Tus archivos nunca salen de tu equipo.", "All features in this version are available without a subscription. Your files never leave your computer."),
    ("Todos", "All"),
    ("Traductor de videos", "Video translator"),
    ("Una línea por subtítulo. Se colocan uno tras otro desde el cursor.", "One line per subtitle. They are placed one after another from the cursor."),
    ("Vaciar", "Clear"),
    ("Ver", "View"),
    ("Video, imagen o audio desde tu equipo", "Video, image or audio from your computer"),
    ("Voces de la colección pública de Piper (Hugging Face). Solo se descargan cuando pulsas el botón.", "Voices from the public Piper collection (Hugging Face). They are only downloaded when you press the button."),
    ("Volver a buscar", "Search again"),
    ("Volver a intentarlo", "Try again"),
    ("Voz", "Voice"),
    ("Vídeo / Audio a transcribir", "Video / audio to transcribe"),
    ("Zona a eliminar", "Area to remove"),
    ("↺ Restablecer estilo de texto", "↺ Reset text style"),
    ("↺ Restablecer mezcla", "↺ Reset mix"),
    ("↺ Restablecer propiedades", "↺ Reset properties"),
    ("↺ Restablecer transformación", "↺ Reset transform"),
    ("↺ Sin animación", "↺ No animation"),
    ("⊘ Sin estilo", "⊘ No style"),
    ("✨ Generar subtítulos con IA…", "✨ Generate subtitles with AI…"),
    ("✨ Iniciar generación", "✨ Start generation"),
    ("🎙 Grabar con el micrófono", "🎙 Record with microphone"),
    ("🎤 Abrir Texto a voz completo", "🎤 Open full Text to speech"),
    ("🗑 Quitar", "🗑 Remove"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_espanol_devuelve_el_original() {
        ENGLISH.store(false, Ordering::Relaxed);
        assert_eq!(tr("Cancelar"), "Cancelar");
    }
}
