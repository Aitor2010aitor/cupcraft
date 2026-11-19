# Análisis de la base de código actual (Fase 0)

Este documento detalla qué hace cada archivo del proyecto heredado, qué datos gestiona, de quién depende y qué funciones/APIs expone, para servir como especificación completa de la reconstrucción nativa en Rust.

---

## 1. Núcleo y Estado

### `src/store.js`
- **Responsabilidad**: Gestor de estado reactivo global del editor, bus de eventos interno, gestión de pistas y clips, historial de 80 pasos para deshacer/rehacer, y serialización del proyecto.
- **Estado que maneja**:
  - `state.view`: `'home' | 'editor'`
  - `state.booted`: Booleano de arranque completado.
  - `state.projects`: Lista de metadatos de proyectos guardados.
  - `state.project`: Objeto del proyecto actualmente abierto (pistas, clips, medios, resolución, fps, ratio).
  - `state.media`: `Map<string, MediaRecord>` con URLs de blobs y metadatos en memoria.
  - `state.selection`: ID del clip seleccionado o `null`.
  - `state.panel`: Panel activo (`'media' | 'audio' | 'text' | 'stickers' | 'effects' | 'transitions' | 'captions' | 'filters' | 'adjust' | 'templates' | 'avatar'`).
  - `state.playing`: Booleano de reproducción activa.
  - `state.playhead`: Posición actual del cabezal de lectura en segundos.
  - `state.zoom`: Nivel de zoom de la línea de tiempo (30 a 260 px/s, por defecto 78).
  - `state.snap`: Imantación magnética activada/desactivada.
  - `state.history` / `state.future`: Pilas de instantáneas JSON del proyecto (tope de 80 pasos).
- **Dependencias**: `src/db.js`, `src/utils.js`, `src/filter-line.js`, `src/library.js`.
- **Funciones expuestas**:
  - `on(reason, fn)`, `emit(reason, data)`: Bus de eventos.
  - `initStore()`, `refreshProjects()`: Carga inicial de proyectos desde base de datos.
  - `createProject(name)`, `newProject(name)`, `openProject(id)`, `deleteProject(id)`, `renameProject(id, name)`: CRUD de proyectos.
  - `exportProjectFile()`, `importProjectFile(file)`: Exportación e importación de archivos `.opencup.json` con medios incrustados en Base64 hasta 256 MB.
  - `importFiles(fileList, options)`, `importExternalMedia(file, options)`: Análisis de medios (video, audio, imagen), cálculo de thumbnail y peaks de audio, guardado en almacén.
  - `addMediaToTimeline(mediaId, options)`, `addTextClip(content, options)`, `addSubtitleClips(items, options)`, `clearSubtitleClips()`, `addStickerClip(content, options)`, `addEffectClip(effect, options)`: Inserción de clips en pistas respetando colisiones con `resolveStart()`.
  - `updateClip(clipId, patch, options)`, `moveClip(clipId, options)`, `trimClip(clipId, edge, deltaSeconds)`: Modificación geométrica y de atributos de clips.
  - `selectClip(clipId)`, `deleteSelected()`, `duplicateSelected()`: Gestión de selección.
  - `splitAtPlayhead(customTime)`, `trimLeftAtPlayhead()`, `trimRightAtPlayhead()`: División y recorte de clips bajo el cabezal.
  - `rotateSelectedClip()`, `flipSelectedClip()`, `reverseSelectedClip()`: Transformaciones rápidas (rotar 90°, volteo horizontal, marcha atrás).
  - `freezeFrameAtPlayhead()`: Congelado de fotograma estilo CapCut (clip de 3s a velocidad 0.0001 insertado en el cabezal).
  - `separateAudioFromSelected()`: Extracción de audio de un clip de vídeo a una nueva pista de audio silenciando el vídeo original.
  - `toggleMarkerAtPlayhead()`: Añadir o quitar marcadores de tiempo en `project.markers`.
  - `undo()`, `redo()`, `mutate()`, `pushHistory()`, `snapshot()`: Motor de deshacer/rehacer.
  - `play()`, `pause()`, `togglePlay()`, `seek(time)`, `stepFrame(direction)`, `skipToEdge(edge)`: Transporte y control de reproducción con `requestAnimationFrame`.
  - `setZoom(value)`, `toggleSnap()`, `setPanel(panel)`: Parámetros de interfaz.

### `src/db.js`
- **Responsabilidad**: Capa de persistencia local mediante IndexedDB (`openCUP-db`, versión 1).
- **Almacenes**:
  - `projects`: Clave primaria `id`. Guarda el JSON serializado de cada proyecto.
  - `media`: Clave primaria `id`. Guarda `{ id, blob, meta }`.
- **Funciones expuestas**: `listProjects()`, `getProject(id)`, `saveProject(project)`, `deleteProject(id)`, `saveMedia(record)`, `getMedia(id)`, `deleteMedia(id)`.

---

## 2. Renderizado, Gráficos y Efectos

### `src/renderer.js`
- **Responsabilidad**: Motor de composición multipista en Canvas2D y puente a shaders WebGL.
- **Capacidades**:
  - Orden de capas: `audio` (-1) < `video`/`overlay` (1) < `sticker` (2) < `text` (3).
  - Cálculo de opacidad y volumen interpolando keyframes (`keyframes.js`) y aplicando fundidos de entrada (`fadeIn`) y salida (`fadeOut`).
  - Animaciones de entrada y transiciones: cálculo de posición, rotación y escala para más de 40 transiciones (básicas, movimiento, zoom, cortinilla, 20 estilo CapCut como slam-hit, earthquake, y 10 de CutWire Drift).
  - Renderizado de clips de texto con cálculo de saltos de línea (`textLayout`), caja de colisión (`clipLayerBox`), contornos (`stroke`), resplandor (`glow`), curvas circulares y alineación.
  - Renderizado de stickers procedurales (emojis o texturas) con fondo redondeado/cuadrado y transformaciones.
  - Superposiciones procedurales: partículas (estrellas, corazones, confeti, fuegos artificiales, burbujas, polvo), marcos (polaroid, film, tape, neón, flores, papel rasgado) y efectos de golpe/terremoto/fuego.
  - Soporte de caché de vídeo y captura del último frame válido (`captureFrame`) para evitar parpadeos negros durante el salto (`seek`).

### `src/gl-post.js`
- **Responsabilidad**: Runner WebGL2 para efectos de post-proceso sobre el fotograma compuesto.
- **Shaders integrados (WGSL target)**:
  - Shaders utilitarios y de textura: `radial-blur`, `directional-blur`, `rgb-split`, `chromatic`, `vhs`, `zoom-pulse`, `pixelate`, `mirror`, `vignette`, `grain`, `posterize`, `wave`, `bloom`.
  - Shaders firma portados de CapCut: `bruma-borgona`, `atardecer-ensueno`, `brillo-brumoso`, `pixel-noir`, `carino-retro`, `sueno-caqui`, `jengibre-suave`, `lienzo-embrujado`.
  - Intérprete de LUT 3D (`capcut-lut`): Textura 512x512 con cubo 64x64x64 repartido en 8x8 celdas.
  - Curva tonal sigmoidea (`curve`).

### `src/filter-line.js`
- **Responsabilidad**: Catálogo de 70+ filtros clasificados en 12 categorías (`Esenciales`, `Cálido`, `Frío`, `Blanco y negro`, `Cine`, `Analógico`, `Retrato`, `Paisaje`, `Neón`, `Clima`, `Textura`, `Firma`). Define equivalencias en CSS y mapeo a shaders GL cuando procede.

### `src/keyframes.js`
- **Responsabilidad**: Motor de interpolación lineal para curvas de fotogramas clave en propiedades de clip (`x`, `y`, `scale`, `scaleX`, `scaleY`, `rotate`, `opacity`, `text.opacity`).

### `src/text-catalog.js`
- **Responsabilidad**: Presets de estilos de texto (20 estilos como Impacto, Neón, Oro, Cine, Retro), estilos de burbuja/fondo (9 variantes), efectos (contorno, resplandor, 3D, arco) y animaciones de texto (21 animaciones clasificadas en entrada, salida, bucle y subtítulos karaoke/salto).

### `src/library.js`
- **Responsabilidad**: Catálogo maestro de efectos visuales (más de 100 efectos agrupados en 19 categorías), stickers (19 categorías temáticas), transiciones (básicas, movimiento, zoom, cortinilla, impacto, elementos, tendencia, distorsión, luz, formas y Drift) y presets de ajuste de color.

---

## 3. Interfaz de Usuario y Vistas

### `src/home.js`
- **Responsabilidad**: Pantalla de bienvenida y gestión de biblioteca.
- **Componentes**: Barra superior con marca y botón de importar/crear; barra lateral izquierda con navegación (Proyectos, Plantillas, Papelera); carrusel de tarjetas de inspiración (Showcases); cuadrícula de proyectos locales con tarjeta de vista previa, duración, resolución y menú de acciones (renombrar, exportar archivo, eliminar); modales para crear proyecto e importar borradores de CapCut.

### `src/editor.js`
- **Responsabilidad**: Estructura principal del entorno de edición.
- **Componentes**: Barra de título (nombre de proyecto editable, guardado, exportar, ventana); área de trabajo de tres columnas con divisores redimensionables (`.v-split`); columna izquierda con selector de herramientas y panel activo; columna central con vista previa y reproductor; columna derecha con panel de detalles/propiedades; área inferior con la línea de tiempo completa.

### `src/timeline.js`
- **Responsabilidad**: Renderizado e interacción de la línea de tiempo.
- **Componentes**: Regla de tiempos con marcas de segundo y submúltiplos adaptables al zoom; cabezal de lectura arrastrable; cabeceras de pista (bloquear, silenciar, ocultar, añadir pista); filas de pistas con clips arrastrables; tiradores laterales para recorte izquierdo/derecho; indicadores de fotogramas clave y filtros aplicados; atajos de teclado para división (`S`), recorte (`Q`/`E`), borrado (`Supr`/`Backspace`), reproducción (`Espacio`) y zoom (`Ctrl + Rueda`).

### `src/panels.js`
- **Responsabilidad**: Implementación de los 11 paneles de herramientas de la columna izquierda (Medios, Audio, Texto, Stickers, Efectos, Transiciones, Subtítulos, Filtros, Ajustar, Plantillas, Avatares, y TTS-Abierto con Piper).

### `src/preview.js`
- **Responsabilidad**: Controlador del reproductor de vista previa en tiempo real. Gestiona la sincronización de vídeo y audio mediante `MediaPool`, interpolación de tiempo, dibujo de tiradores de selección sobre el canvas y redimensionado adaptativo manteniendo la relación de aspecto.

### `src/styles.css`
- **Responsabilidad**: Definición exhaustiva de estilos y tokens visuales (tema oscuro, paleta de colores, tipografías, radios de borde, alturas de pistas y cabeceras).

---

## 4. Integraciones y Servicios

### `src/capcut.js`
- **Responsabilidad**: Lector y conversor de borradores nativos de CapCut (`draft_content.json`).
- **Mapeo**: Descubre borradores en `%LOCALAPPDATA%\CapCut\User Data\Projects\com.lveditor.draft`, extrae medios reales referenciados en `material_path()`, mapea pistas de audio, vídeo, texto y stickers, y los reconstruye dentro del modelo de cupcraft sin modificar los archivos originales de CapCut.

### `src/ai-bridge.js`
- **Responsabilidad**: Puente de control remoto para agentes de Inteligencia Artificial mediante API REST/JSON. Expone más de 30 acciones atómicas (`getState`, `newProject`, `openProject`, `seek`, `play`, `pause`, `addText`, `addSticker`, `applyFilter`, `applyEffect`, `applyTransition`, `updateClip`, `split`, `deleteClip`, `duplicateClip`, `undo`, `redo`, `screenshot`, `autoAnalyze`, `autoPlan`, `autoApply`).

### `src/tts-abierto.js`
- **Responsabilidad**: Interfaz cliente para el generador de voz local Piper TTS con modelos en español (`davefx`, `sharvard`, `carlfm`), control de velocidad y tono, e inserción directa del audio resultante en la línea de tiempo.

### `src/export.js` y `src/webm.js`
- **Responsabilidad**: Exportación de vídeo en la versión web mediante Canvas Capture Stream y MediaRecorder (actualmente limitado a WebM en tiempo real, objetivo clave a sustituir por codificación nativa FFmpeg ultrarrápida a MP4/H.264/H.265).
