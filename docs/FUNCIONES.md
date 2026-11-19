# Mapeo de Funciones: JavaScript a Rust (Fase 0)

Esta tabla mapea cada función, módulo y subsistema de la aplicación web anterior (`src/*.js`) a su crate y módulo correspondiente en la arquitectura nativa en Rust.

---

## 1. Núcleo, Estado y Operaciones de Timeline

| Función JS original (`src/store.js`) | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| `createProject(name)` | `cupcraft-core::model::Project::new` | Constructor de proyecto con 4 pistas por defecto. |
| `projectDuration()` | `cupcraft-core::model::Project::duration` | Cálculo del instante de fin máximo entre todas las pistas. |
| `timelineLength()` | `cupcraft-core::model::Project::timeline_length` | Longitud visible total de la línea de tiempo. |
| `addTrack(type, name)` | `cupcraft-core::model::Project::add_track` | Añadir nueva pista al proyecto. |
| `findClip(clipId)` | `cupcraft-core::timeline::find_clip` | Búsqueda por ID devolviendo referencia mutable a pista y clip. |
| `selectedClip()` | `cupcraft-core::timeline::selected_clip` | Obtiene el clip seleccionado actualmente en el editor. |
| `resolveStart(track, id, des, dur)` | `cupcraft-core::timeline::resolve_start` | Algoritmo anti-solapamiento magnético en 6 pasadas. |
| `addMediaToTimeline(mediaId, opts)` | `cupcraft-core::timeline::add_media_to_timeline` | Inserta clip en pista adecuada resolviendo colisiones. |
| `addTextClip(content, opts)` | `cupcraft-core::timeline::add_text_clip` | Inserta clip de texto con estilo y tipografía predeterminados. |
| `addSubtitleClips(items, opts)` | `cupcraft-core::timeline::add_subtitle_clips` | Inserción en lote de subtítulos con temporización de palabras. |
| `clearSubtitleClips()` | `cupcraft-core::timeline::clear_subtitle_clips` | Elimina todos los clips de texto de la pista de subtítulos. |
| `addStickerClip(content, opts)` | `cupcraft-core::timeline::add_sticker_clip` | Añade sticker procedural a la pista de superposición. |
| `addEffectClip(effect, opts)` | `cupcraft-core::timeline::add_effect_clip` | Añade efecto visual con duración a la pista de efectos. |
| `updateClip(clipId, patch, opts)` | `cupcraft-core::timeline::update_clip` | Modifica propiedades de clip registrando paso en el historial. |
| `moveClip(clipId, { start, trackId })` | `cupcraft-core::timeline::move_clip` | Traslado de clip entre pistas compatibles o en el tiempo. |
| `trimClip(clipId, edge, delta)` | `cupcraft-core::timeline::trim_clip` | Recorte por el borde izquierdo (in/start) o derecho (duration). |
| `selectClip(clipId)` | `cupcraft-core::history::Editor::select_clip` | Asigna selección activa y notifica a la interfaz. |
| `deleteSelected()` | `cupcraft-core::timeline::delete_selected` | Elimina el clip seleccionado y limpia la selección. |
| `duplicateSelected()` | `cupcraft-core::timeline::duplicate_selected` | Clona clip seleccionado y lo sitúa inmediatamente después. |
| `splitAtPlayhead(customTime)` | `cupcraft-core::timeline::split_at_playhead` | Divide el clip seleccionado o todos los clips bajo el cabezal. |
| `trimLeftAtPlayhead()` | `cupcraft-core::timeline::trim_left_at_playhead` | Recorta la parte izquierda del clip hasta el cabezal (atajo Q). |
| `trimRightAtPlayhead()` | `cupcraft-core::timeline::trim_right_at_playhead` | Recorta la parte derecha del clip desde el cabezal (atajo E). |
| `rotateSelectedClip()` | `cupcraft-core::timeline::rotate_selected` | Gira el clip 90 grados acumulativos (módulo 360). |
| `flipSelectedClip()` | `cupcraft-core::timeline::flip_selected` | Alterna el volteo horizontal (`flip_h = !flip_h`). |
| `reverseSelectedClip()` | `cupcraft-core::timeline::reverse_selected` | Alterna reproducción marcha atrás (`reverse = !reverse`). |
| `freezeFrameAtPlayhead()` | `cupcraft-core::timeline::freeze_frame` | Inserta clip congelado de 3s (velocidad 0.0001) y desplaza el resto. |
| `separateAudioFromSelected()` | `cupcraft-core::timeline::separate_audio` | Crea clip en pista de audio y silencia el vídeo original (`volume = 0`). |
| `toggleMarkerAtPlayhead()` | `cupcraft-core::timeline::toggle_marker` | Añade o quita un marcador en el cabezal dentro de `project.markers`. |
| `undo()` / `redo()` | `cupcraft-core::history::History::undo / redo` | Gestión de la pila de instantáneas JSON (límite 80). |
| `exportProjectFile()` | `cupcraft-persist::project_file::export` | Genera archivo JSON `.opencup.json` con medios incrustados. |
| `importProjectFile(file)` | `cupcraft-persist::project_file::import` | Parsea archivo `.opencup.json` y almacena medios en SQLite. |

---

## 2. Persistencia y Almacenamiento Local

| Función JS original (`src/db.js`) | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| `db.listProjects()` | `cupcraft-persist::store::Store::list_projects` | Consulta en tabla `projects` ordenada por `updated_at DESC`. |
| `db.getProject(id)` | `cupcraft-persist::store::Store::get_project` | Lee el JSON de un proyecto por su ID. |
| `db.saveProject(project)` | `cupcraft-persist::store::Store::save_project` | Inserta o actualiza un registro en la tabla `projects`. |
| `db.deleteProject(id)` | `cupcraft-persist::store::Store::delete_project` | Borra el proyecto y elimina sus medios asociados en cascada. |
| `db.saveMedia(record)` | `cupcraft-persist::store::Store::save_media` | Guarda los metadatos en SQLite y el archivo binario en disco. |
| `db.getMedia(id)` | `cupcraft-persist::store::Store::get_media` | Obtiene los metadatos y la ruta al archivo en disco. |
| `db.deleteMedia(id)` | `cupcraft-persist::store::Store::delete_media` | Elimina el registro de la tabla `media` y su archivo físico. |

---

## 3. Composición, Render y Shaders en GPU

| Subsistema JS original | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| `activeLayers(project, time)` | `cupcraft-render::compositor::active_layers` | Filtra clips activos y los ordena por capa. |
| `clipAlpha(clip, local)` | `cupcraft-render::compositor::clip_alpha` | Evalúa opacidad combinando keyframes, fadeIn y fadeOut. |
| `clipGain(clip, local)` | `cupcraft-audio::mixer::clip_gain` | Evalúa ganancia de audio combinando volumen y fundidos. |
| `textLayout(text, w, h)` | `cupcraft-render::text::layout` | Cálculo de fuentes, saltos de línea y métricas de caja. |
| `clipLayerBox(clip, w, h)` | `cupcraft-render::text::layer_box` | Dimensiones y posición para tiradores y detección de clics. |
| `transitionState(clip, local)` | `cupcraft-render::transitions::state` | Progreso y tipo de la transición activa en el punto de entrada. |
| Shaders GL en `gl-post.js` (13) | `cupcraft-render::shaders::*.wgsl` | Shaders nativos WGSL ejecutados sobre pipeline de render en `wgpu`. |
| Shaders firma CapCut (8) | `cupcraft-render::shaders::signature::*.wgsl` | Implementación WGSL de los 8 shaders procedurales (Bruma borgoña, etc.). |
| Intérprete LUT 3D (`capcut-lut`) | `cupcraft-render::shaders::lut.wgsl` | Shader WGSL que mapea el cubo 64^3 sobre una textura 2D de 512x512. |
| Canvas2D Particle Overlays | `cupcraft-render::particles::Emitter` | Sistema de partículas en GPU (confeti, estrellas, fuegos artificiales). |
| Canvas2D Frame Fx | `cupcraft-render::frames::draw_frame` | Marcos procedurales dibujados con primitivas vectoriales en wgpu. |

---

## 4. Medios, Audio, Exportación y FFmpeg

| Subsistema JS original | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| `analyzeFile(file)` (`store.js`) | `cupcraft-media::probe::analyze` | Extracción de duración, códec, dimensiones y forma de onda con FFmpeg. |
| `computeAudioInfo(blob)` | `cupcraft-audio::waveform::compute_peaks` | Decodificación rápida con Symphonia y generación de 220 peaks. |
| `MediaPool` (`renderer.js`) | `cupcraft-media::cache::FrameCache` | Caché de decodificación de fotogramas de vídeo para reproducción continua. |
| Audio playback con WebAudio | `cupcraft-audio::player::AudioPlayer` | Stream de salida de audio de baja latencia con `cpal` + `symphonia`. |
| `export.js` / `webm.js` | `cupcraft-media::export::Exporter` | Exportador de vídeo nativo a MP4 (H.264/H.265) y WebM con aceleración GPU. |

---

## 5. Importador de CapCut

| Función JS original (`src/capcut.js`) | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| `listCapCutProjects()` | `cupcraft-capcut::scanner::list_drafts` | Escaneo del directorio de borradores de CapCut en AppData. |
| Parseo de `draft_content.json` | `cupcraft-capcut::parser::parse_draft` | Deserialización de pistas, clips, curvas y materiales de CapCut. |
| `material_path()` resolution | `cupcraft-capcut::resolver::find_media_path` | Búsqueda robusta del archivo original en disco con fallbacks. |
| `importCapCutProject(id)` | `cupcraft-capcut::importer::import_to_cupcraft` | Conversión completa del borrador a proyecto nativo de cupcraft. |

---

## 6. Puente de Inteligencia Artificial (AI Bridge)

| Endpoint / Acción JS (`src/ai-bridge.js`) | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| Servidor HTTP en `127.0.0.1` | `cupcraft-bridge::server::start` | Servidor web nativo ligero con `axum` y autenticación por token. |
| `getState` | `cupcraft-bridge::handlers::get_state` | Serializa el estado completo del editor a JSON. |
| `newProject`, `openProject`, `save` | `cupcraft-bridge::handlers::project_*` | Despacho directo al gestor de proyectos de `cupcraft-core`. |
| `seek`, `play`, `pause`, `togglePlay` | `cupcraft-bridge::handlers::playback_*` | Control de transporte de reproducción. |
| `addText`, `addSticker`, `split`, etc. | `cupcraft-bridge::handlers::dispatch_command` | Ejecución atómica de comandos del editor. |
| `screenshot` | `cupcraft-bridge::handlers::screenshot` | Renderiza el frame actual a memoria y lo devuelve como JPEG en Base64. |

---

## 7. Voces Locales (TTS-Abierto)

| Módulo JS original (`src/tts-abierto.js`) | Crate y módulo Rust destino | Equivalente / Descripción |
|---|---|---|
| Listado de voces | `cupcraft-tts::voices::list_voices` | Detección de modelos ONNX de Piper en `tools/piper/`. |
| Generación de audio | `cupcraft-tts::synthesizer::generate` | Invocación del binario `piper.exe` pasando texto por stdin a WAV temporal. |
