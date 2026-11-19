# Dudas y Decisiones para Confirmar con el Usuario (Fase 0)

Durante la lectura y análisis detallado del código fuente antiguo se han identificado los siguientes puntos para tu revisión y confirmación antes de iniciar la Fase 1:

---

### 1. Migración de Proyectos Existentes desde WebView2
- **Situación actual**: En la versión web anterior, los proyectos y archivos de medios se almacenaban dentro de la base de datos IndexedDB del perfil de WebView2 (`%LOCALAPPDATA%\cupcraft\WebView2`).
- **Propuesta**: Al arrancar por primera vez la app nativa, detectar si existen proyectos en esa carpeta de WebView2 y ofrecer una opción de migración automática transparente (extrayendo los medios a `%LOCALAPPDATA%\cupcraft\media\` e insertando los registros en la base de datos SQLite `projects.db`).
- **Pregunta**: ¿Te parece bien que se detecte y ofrezca la migración al inicio, o prefieres un botón manual en la pantalla de bienvenida?

---

### 2. Formato de Stickers Propietarios `.pag` de CapCut
- **Situación actual**: CapCut empaqueta sus stickers animados en formato propietario `.pag` (Portable Animated Graphics de Tencent). La versión web anterior usaba stickers procedurales (emojis enriquecidos, formas vectoriales e imágenes PNG/WebP) porque `.pag` en web requería WebAssembly de gran tamaño.
- **Propuesta**: Mantener el sistema completo de stickers actual (19 categorías, emojis enriquecidos y stickers de imagen) y dejar la integración del SDK nativo `libpag` de C++ en la lista de características futuras para no retrasar el lanzamiento del ejecutable nativo.

---

### 3. Aceleración por Hardware en la Exportación (FFmpeg)
- **Situación actual**: La versión web anterior solo podía exportar en WebM en tiempo real a través del navegador.
- **Propuesta**: En la versión nativa con FFmpeg, soportar exportación ultrarrápida a **MP4 (H.264 / H.265)** y WebM. Detectaremos automáticamente la GPU disponible:
  - NVIDIA: `h264_nvenc` / `hevc_nvenc`
  - Intel: `h264_qsv` / `hevc_qsv`
  - AMD: `h264_amf` / `hevc_amf`
  - Fallback por software: `libx264` / `libx265` de alta compatibilidad.

---

### 4. Vistas Previas Animadas de Transiciones en Hover
- **Situación actual**: En la web se renderizan animaciones CSS en el evento `:hover` sobre las tarjetas de transición para ver cómo se verá la transición antes de aplicarla.
- **Propuesta**: En `egui`, cuando el cursor del ratón se pose sobre una tarjeta de transición en el panel lateral, el shader o la rutina de dibujo ejecutará un ciclo de animación en bucle (0 a 1 segundo) en la miniatura de la tarjeta, reproduciendo exactamente la misma experiencia visual que en la web.
