# Modelo de Datos de cupcraft (Fase 0)

Este documento especifica el esquema de datos exacto y tipado para Rust, garantizando compatibilidad total con los archivos `.opencup.json`, el historial de deshacer y el formato de base de datos SQLite.

---

## 1. Esquema de Entidades Principales

### `Project` (Proyecto)
```rust
pub struct Project {
    /// Identificador único con prefijo: "proj_..."
    pub id: String,
    /// Nombre visible del proyecto
    pub name: String,
    /// Timestamp Unix de creación en milisegundos
    pub created_at: i64,
    /// Timestamp Unix de última modificación en milisegundos
    pub updated_at: i64,
    /// Relación de aspecto: "16:9", "9:16", "1:1", "4:3", "21:9", "4:5"
    pub ratio: String,
    /// Resolución nominal: "480p", "720p", "1080p", "4K"
    pub resolution: String,
    /// Fotogramas por segundo del proyecto (24, 30, 60)
    pub fps: u32,
    /// Metadatos de medios registrados en el proyecto
    pub media: Vec<Media>,
    /// Data URL o ruta local a la miniatura del proyecto (JPEG)
    pub thumb: Option<String>,
    /// Lista ordenada de pistas
    pub tracks: Vec<Track>,
    /// Lista de marcas de tiempo en segundos para navegación rápida
    pub markers: Vec<f64>,
}
```

### `Track` (Pista)
```rust
pub enum TrackType {
    Video,
    Audio,
    Text,
    Overlay,
}

pub struct Track {
    /// Identificador único con prefijo: "track_..."
    pub id: String,
    /// Tipo funcional de la pista
    pub kind: TrackType,
    /// Nombre visible ("Video 1", "Audio 1", "Texto 1", etc.)
    pub name: String,
    /// Estado de visibilidad (oculto no se dibuja en el reproductor)
    pub hidden: bool,
    /// Estado de silencio (silenciado no se mezcla en el audio)
    pub muted: bool,
    /// Ganancia de volumen de la pista (0.0 a 2.0, por defecto 1.0)
    pub volume: f64,
    /// Bloqueo contra edición accidental
    pub locked: bool,
    /// Lista de clips situados en esta pista
    pub clips: Vec<Clip>,
}
```

### `Clip` (Clip de Línea de Tiempo)
```rust
pub enum ClipKind {
    Video,
    Image,
    Audio,
    Text,
    Sticker,
    Effect,
}

pub struct Clip {
    /// Identificador único: "clip_..."
    pub id: String,
    /// Referencia al medio en disco/almacén (None para texto o stickers puros)
    pub media_id: Option<String>,
    /// Tipo de contenido del clip
    pub kind: ClipKind,
    /// Nombre visible en la interfaz
    pub name: String,
    /// Posición de inicio en la línea de tiempo en segundos
    pub start: f64,
    /// Duración visible del clip en segundos
    pub duration: f64,
    /// Desplazamiento de entrada en el medio original en segundos (in-point)
    pub in_point: f64,
    /// Multiplicador de velocidad (0.25 a 4.0, por defecto 1.0)
    pub speed: f64,
    /// Nivel de volumen (0.0 a 2.0, por defecto 1.0)
    pub volume: f64,
    /// Nivel de opacidad (0.0 a 1.0, por defecto 1.0)
    pub opacity: f64,
    /// Factor de escala uniforme (por defecto 1.0)
    pub scale: f64,
    /// Desplazamiento horizontal relativo en porcentaje (-100 a +100)
    pub x: f64,
    /// Desplazamiento vertical relativo en porcentaje (-100 a +100)
    pub y: f64,
    /// Ángulo de rotación en grados (0 a 360)
    pub rotate: f64,
    /// Inversión horizontal (espejo horizontal)
    pub flip_h: bool,
    /// Inversión vertical (espejo vertical)
    pub flip_v: bool,
    /// Reproducción invertida
    pub reverse: bool,
    /// Filtro de color aplicado (clave de catálogo o "none")
    pub filter: String,
    /// Ajustes manuales de colorimetría
    pub adjust: Adjust,
    /// Duración del fundido de entrada de opacidad/volumen en segundos
    pub fade_in: f64,
    /// Duración del fundido de salida de opacidad/volumen en segundos
    pub fade_out: f64,
    /// Efecto visual aplicado (clave de EFFECTS o "none")
    pub effect: String,
    /// Intensidad del efecto (0.0 a 1.0)
    pub effect_strength: f64,
    /// Transición aplicada en el punto de entrada del clip
    pub transition: Option<String>,
    /// Duración de la transición en segundos (típico 0.6s)
    pub transition_duration: f64,
    /// Almacén de curvas de fotogramas clave
    pub keyframes: KeyframeStore,
    /// Configuración específica de texto (si kind == Text)
    pub text: Option<TextProps>,
    /// Configuración específica de sticker (si kind == Sticker)
    pub sticker: Option<StickerProps>,
}
```

### `Adjust` (Ajustes de Colorimétria)
```rust
pub struct Adjust {
    pub brightness: f64,   // 0 a 200, neutro 100
    pub contrast: f64,     // 0 a 200, neutro 100
    pub saturate: f64,     // 0 a 200, neutro 100
    pub grayscale: f64,    // 0 a 100, neutro 0
    pub sepia: f64,        // 0 a 100, neutro 0
    pub hue: f64,          // -180 a +180 grados, neutro 0
}
```

### `TextProps` (Propiedades de Texto y Subtítulos)
```rust
pub struct TextProps {
    pub content: String,
    pub font: String,
    pub size: f64,
    pub weight: u32,
    pub italic: bool,
    pub color: String,             // Formato Hex "#RRGGBB"
    pub align: String,             // "left", "center", "right"
    pub letter_spacing: f64,
    pub line_spacing: f64,
    pub text_case: String,         // "none", "upper", "lower", "capitalize"
    pub stroke_color: String,
    pub stroke_width: f64,
    pub glow: bool,
    pub glow_color: String,
    pub glow_blur: f64,
    pub shadow: bool,
    pub shadow_color: String,
    pub shadow_blur: f64,
    pub shadow_distance: f64,
    pub shadow_angle: f64,
    pub background: String,        // "transparent" o Hex "#RRGGBB"
    pub background_alpha: f64,
    pub background_radius: f64,
    pub background_padding: f64,
    pub curve: f64,                // Deformación en arco (-100 a +100)
    pub animation_in: Option<String>,
    pub animation_out: Option<String>,
    pub animation_loop: Option<String>,
    pub words: Vec<WordTiming>,    // Para subtítulos estilo karaoke
}

pub struct WordTiming {
    pub word: String,
    pub start: f64,
    pub end: f64,
}
```

### `StickerProps` (Propiedades de Sticker)
```rust
pub struct StickerProps {
    pub content: String,           // Emoji o identificador de recurso
    pub src: Option<String>,       // Ruta o URL de imagen local
    pub size: f64,                 // Tamaño base en píxeles virtuales
    pub background: String,
    pub shape: String,             // "none", "round", "square"
    pub color: String,
    pub rotate: f64,
}
```

### `KeyframeStore` (Curvas de Animación)
```rust
pub struct KeyframePoint {
    pub t: f64, // Tiempo absoluto en segundos
    pub v: f64, // Valor interpolado
}

pub struct KeyframeStore {
    pub points: HashMap<String, Vec<KeyframePoint>>,
}
// Propiedades animables soportadas:
// "x", "y", "scale", "scaleX", "scaleY", "rotate", "opacity", "text.opacity"
```

### `Media` (Ficha de Medio en Biblioteca)
```rust
pub struct Media {
    pub id: String,
    pub name: String,
    pub kind: String,          // "video", "audio", "image"
    pub mime: String,
    pub size_bytes: u64,
    pub duration: f64,         // Duración total en segundos (0 para imágenes)
    pub width: u32,
    pub height: u32,
    pub file_path: PathBuf,    // Ubicación real en disco
    pub thumb_path: Option<PathBuf>,
    pub audio_peaks: Vec<f32>, // 220 muestras normalizadas para forma de onda
}
```
