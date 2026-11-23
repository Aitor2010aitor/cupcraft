# Documentación Completa de CUPCRAFT

> Documento generado para revisión por Claude. Describe la arquitectura, lenguaje, funciones, herramientas y estado actual del proyecto.

---

## 1. Visión General

**CUPCRAFT** es un editor de video de escritorio inspirado en CapCut, construido con:
- **Frontend**: HTML/CSS/JavaScript (ES Modules) renderizado en WebView2.
- **Backend nativo**: Rust (crates) que sirve la UI y gestiona medios, audio, renderizado y persistencia.
- **Ejecutable**: `dist\cupcraft-rust\cupcraft.exe` (instancia única, mutex `cupcraft.Desktop.SingleInstance`).

La app sirve la UI en `http://127.0.0.1:8787` y un puente de IA en `http://127.0.0.1:8790`.

---

## 2. Estructura de Carpetas

```
cupcraft/
├── src/                    # Interfaz web (JS/CSS/HTML)
│   ├── panels.js           # Panel "Detalles" completo (pestañas y controles)
│   ├── renderer.js         # Dibujado del canvas (máscara, chroma key, blur, filtros, voz)
│   ├── store.js            # Estado del proyecto, clips, historial, importación
│   ├── timeline.js         # Línea de tiempo, drag, seek
│   ├── styles.css          # Estilos de pestañas, máscaras, curvas, voz, etc.
│   ├── icons.js            # Iconos SVG
│   ├── library.js          # Filtros, efectos, transiciones, categorías
│   ├── capcut-panels.js    # Catálogo real de CapCut (efectos, animaciones)
│   ├── capcut-catalog.js   # Catálogo CapCut completo
│   ├── capcut.js           # Integración con CapCut
│   ├── capcut-text-style.js# Estilos de texto de CapCut
│   ├── auto-edit.js        # Edición automática
│   ├── ai-bridge.js        # Puente de IA
│   ├── db.js               # Base de datos IndexedDB
│   ├── modal.js            # Sistema de modales
│   ├── utils.js            # Utilidades (toast, escapeHtml, etc.)
│   ├── keyframes.js        # Fotogramas clave
│   ├── gl-post.js          # Shaders WebGL (runner)
│   ├── filter-line.js      # Filtros de línea
│   ├── text-catalog.js     # Catálogo de texto
│   ├── sticker-animation.js# Animación de stickers
│   ├── sticker-image.js    # Imágenes de stickers
│   ├── export.js           # Exportación de video
│   ├── home.js             # Pantalla de inicio
│   ├── editor.js           # Editor principal
│   ├── main.js             # Punto de entrada
│   ├── layout-resize.js    # Redimensionado de paneles
│   ├── desktop-frame.js    # Marco de escritorio (menú contextual, etc.)
│   ├── dialogs.js          # Diálogos
│   ├── templates.js        # Plantillas
│   ├── webm.js             # WebM
│   ├── kokoro-tts.js       # TTS Kokoro
│   ├── tts-abierto.js      # TTS abierto
│   └── log.js              # Registro de la app
├── dist/cupcraft-rust/     # App nativa empaquetada
│   ├── cupcraft.exe        # Ejecutable
│   └── src/                # Copia sincronizada de src/ (la que usa el exe)
├── rust/crates/            # Código Rust
│   ├── cupcraft-app/       # App principal
│   ├── cupcraft-audio/     # Audio
│   ├── cupcraft-bridge/    # Puente
│   ├── cupcraft-capcut/    # CapCut
│   ├── cupcraft-core/      # Núcleo
│   ├── cupcraft-desktop/   # Desktop (main.rs, instancia única)
│   ├── cupcraft-media/     # Medios
│   ├── cupcraft-persist/   # Persistencia
│   ├── cupcraft-render/    # Renderizado
│   ├── cupcraft-server/    # Servidor HTTP
│   └── cupcraft-tts/       # TTS
├── assets/                 # LUTs, previews, iconos
├── tools/                  # Scripts de build
├── docs/                   # Documentación
├── index.html              # Página principal
└── dist/                   # Distribución
```

---

## 3. Lenguaje y Tecnologías

| Capa | Tecnología |
|---|---|
| Frontend | JavaScript (ES Modules), HTML5, CSS3 |
| Renderizado | Canvas 2D, WebGL (shaders) |
| Audio | WebAudio API |
| Backend | Rust (crates modulares) |
| Empaquetado | WebView2 (Microsoft Edge) + servidor HTTP local |
| Persistencia | IndexedDB (frontend), SQLite (Rust) |
| IA | Puente HTTP local (puerto 8790) |

---

## 4. Panel "Detalles" — Estructura CapCut

### Pestañas principales (DETAIL_TABS)
1. **Video** → Básico, Eliminar fondo, Máscara, Retoque
2. **Audio** → Básico, Modificador de voz
3. **Velocidad** → Estándar, Curva, Efectos de velocidad
4. **Animación** → Entrada, Salida, Combinado
5. **Ajustar** → Básico, HSL, Curvas, Círculo cromático, Máscara
6. **Estilizado de IA** → Rostro, Cuerpo, Cabello, Párrafos, Párrafos corporales

### Funciones por pestaña

#### Video → Básico
- Transformación (escala, posición, rotación)
- Estabilización, Mejorar calidad, Reducir ruido, Flujo óptico
- Eliminación de IA (zona editable: X/Y/ancho/alto/difuminado)
- Ampliación de IA, Remix de IA, Contacto visual, Sincronización labial
- Adición de iluminación, Automarcado, Eliminar parásitos, Movimiento de IA, Seguimiento de cámara, Movimiento difuminado
- Lienzo (relación de aspecto, resolución) + "Aplicar a todo"

#### Video → Eliminar fondo
- Eliminación automática (switch, IA)
- Trazo (estilos, color, tamaño, espaciado)
- Eliminación personalizada (pincel, pincel rápido, lazo, borrador)
- Chroma key (selector de color, intensidad, sombra, borde de plumas, limpiar borde)
- Recortar con máscara

#### Video → Máscara
- 9 formas: Ninguna, Lineal, Espejo, Círculo, Rectángulo, Redondeado, Corazón, Estrella, Triángulo
- Feather, Escala, Rotación, Posición X/Y, Invertir

#### Video → Retoque
- Suavizar piel, Blanquear, Sonrojo (funciona en preview/export)
- Adelgazar, Ojos, Dientes (bloqueados IA)

#### Audio → Básico
- Volumen en dB, aparición/desaparición progresiva
- Normalizar (PRO), Mejorar voz, Traductor, Reducir ruido, Aislar voz, Completar canal (bloqueados IA)

#### Audio → Modificador de voz
- ~90 nombres de CapCut (grave, agudo, eco, robot, radio, etc.)
- Cadena WebAudio local (EQ + delay/eco)

#### Velocidad
- Estándar: velocidad×, preservar tono
- Curva: PRO (bloqueado)
- Efectos de velocidad: Flash, Movimiento, Desvanecer, Zoom retro, Alcohol

#### Animación
- Entrada/Salida/Combinado: catálogos CapCut reales filtrados por `item.cats`
- Cada tile ejecuta `capcutEffectToGL` → `effectClips` GL y/o mapeo local

#### Ajustar → Básico
- Intensidad, Combinación de colores (PRO), Corrección, LUT
- Exposición, temperatura, tinte, saturación, claridad, contraste, luminar, sombra, blancos, negros, luminosidad
- Enfoque, partículas, desvanecer, viñeta

#### Ajustar → HSL
- 8 familias de color + Hue/Sat/Light

#### Ajustar → Curvas
- Selector de canal (Luma/Rojo/Verde/Azul)
- Puntos de control interactivos (añadir clic, arrastrar, borrar doble clic)
- Trazado suave (Catmull-Rom)

#### Ajustar → Círculo cromático
- Sombras, medios, tinte, desplazamiento con disco arrastrable

#### Estilizado de IA
- Rostro, Cuerpo, Cabello, Párrafos, Párrafos corporales
- Sliders 0-100 guardados en `clip.aiStyle.<grupo>.<parámetro>`

---

## 5. Funciones del Renderer (renderer.js)

### Funciones principales
- `drawMediaLayer(ctx, layer, width, height, pool, options)` — dibuja un clip en el canvas
- `clipFilter(clip)` — genera la cadena de filtros CSS del clip
- `adjustCss(adjust)` — ajustes de color (exposición, claridad, etc.)
- `hslCss(hsl)` — filtros HSL
- `beautyCss(beauty)` — retoque (suavizado, blanqueo, sonrojo)
- `curveCss(clip)` — brillo/contraste derivados de la curva
- `maskActive(clip)` / `applyMask(ctx, clip, width, height)` — recorte con máscara
- `traceMaskPath(ctx, clip, width, height)` — traza la forma de máscara
- `drawVignette(ctx, clip, width, height)` — viñeta radial
- `drawAiRemove(ctx, clip, width, height)` — blur de zona de eliminación de IA
- `applyChromaKeyPixels(ctx, clip, width, height)` — eliminación de color por píxel
- `drawMediaSource(ctx, clip, media, width, height, pool)` — dibuja el medio (video/imagen)
- `drawMediaSourceChroma(ctx, clip, media, width, height, pool, filter)` — wrapper con chroma key

### Cadena de voz (WebAudio)
- `buildVoiceChain(ctx, clip)` — construye la cadena de audio
- `applyVoice(clip)` — aplica el modificador de voz seleccionado

---

## 6. Store (store.js)

### Funciones clave
- `updateClip(clipId, patch, opts)` — modifica un clip
- `updateClipsMany(entries)` — modifica múltiples clips (una sola entrada de historial)
- `addMediaToTimeline(mediaId, opts)` — añade clip a la línea de tiempo
- `importFiles(fileList, opts)` — importa archivos multimedia
- `analyzeFile(file)` — analiza un archivo (video/imagen/audio)
- `newProject(name)` — crea un proyecto nuevo
- `snapshot()` / `undo()` / `redo()` — historial

---

## 7. Panels (panels.js)

### Funciones de construcción de UI
- `switchRow(label, key, value, opts)` — fila de interruptor
- `lockedRow(label, badge, note)` — fila bloqueada (PRO/IA)
- `detailRange(label, prop, value, min, max, step, factor, readout, unit)` — slider
- `maskGrid(clip)` / `maskControls(clip)` — rejilla y controles de máscara
- `voiceGrid(clip)` — rejilla de modificadores de voz
- `speedTiles(clip)` / `speedFxTiles(clip)` — tiles de velocidad
- `animationTab(clip, sub)` — pestaña de animación
- `hslTab(clip)` — pestaña HSL
- `curvesTab(clip)` — pestaña de curvas (con puntos interactivos)
- `wheelTab(clip)` — círculo cromático
- `adjustBasicTab(clip)` — ajustes básicos de color
- `retouchTab(clip)` — retoque
- `aiTab(clip, sub)` — estilizado de IA
- `detailTabBody(clip, tab, sub)` — cuerpo de la pestaña activa
- `cutoutTab(clip)` — pestaña de eliminar fondo

### Manejadores de eventos
- Click: pestañas, sub-pestañas, máscaras, switches, voz, velocidad, animación, HSL, curvas, ruedas, apply-all, etc.
- Input: sliders, campos de texto, volumen en dB
- Change: filtros, efectos, transiciones, stickers
- Pointer: arrastre de puntos de curva
- Dblclick: borrar punto de curva

### Utilidades
- `readNested(source, key)` — lee propiedades anidadas
- `setNestedPatch(clip, key, value)` — crea parche para propiedades anidadas
- `curvePathFromPoints(points)` — genera path SVG suave (Catmull-Rom)
- `applyRange(input)` — aplica un slider a la propiedad del clip

---

## 8. Iconos (icons.js)

Iconos SVG disponibles, incluyendo el nuevo `mask`:
`play`, `pause`, `stop`, `skip`, `rewind`, `forward`, `volume`, `mute`, `settings`, `export`, `import`, `trash`, `copy`, `paste`, `cut`, `undo`, `redo`, `zoom`, `fit`, `fullscreen`, `close`, `minimize`, `maximize`, `menu`, `search`, `filter`, `effect`, `transition`, `text`, `sticker`, `audio`, `video`, `image`, `mask`, `rotate`, `flip`, `crop`, `speed`, `animation`, `adjust`, `ai`, `voice`, `curve`, `wheel`, `hsl`, `color`, `palette`, `eyedropper`, `wand`, `edit`, `trash`, etc.

---

## 9. Estilos CSS (styles.css)

Secciones principales:
- Layout del editor (paneles, timeline, preview)
- Pestañas de detalles (`.detail-tab`, `.detail-subtab`)
- Máscaras (`.mask-grid`, `.mask-tile`)
- Voz (`.voice-grid`, `.voice-chip`)
- Curvas (`.curve-box`, `.curve-dot`, `.curve-line`)
- HSL (`.hsl-grid`, `.hsl-swatch`)
- Ruedas (`.wheel-row`, `.wheel-disc`)
- Tiles de animación (`.fx-tile`, `.fx-grid`)
- Switches (`.switch-row`, `.switch`)
- Texto (`.tx-group`, `.tx-chip`, `.tx-range`)
- Registro (`.log-row`, `.log-list`)

---

## 10. Estado de Funciones

### ✅ Funciona (efecto real en canvas/exportación)
- Máscara (9 formas + invertir)
- Viñeta
- Ajustes de color (exposición, temperatura, tinte, saturación, claridad, contraste, etc.)
- HSL (aproximación global)
- Retoque (suavizar, blanquear, sonrojo)
- Chroma key (eliminación de color por píxel)
- Eliminación de IA (blur de zona)
- Modificador de voz (cadena WebAudio)
- Animaciones (mapeo a efectos GL locales)
- Curvas (edición + brillo/contraste)
- Eliminación de ruido (blur leve)
- Estabilización, flujo óptico, ampliación, movimiento IA, seguimiento de cámara, contacto visual, sincronización labial, remix (efectos CSS aproximados)
- Aplicar a todo

### ⚠️ Solo UI/valores guardados (sin efecto real)
- Trazo (borde de recorte)
- Eliminación personalizada (pincel/lazo/borrador)
- Normalizar intensidad (PRO)
- Combinación de colores (PRO)
- Curva de velocidad (PRO)

### ❌ Bloqueados (requieren IA en la nube)
- Recorte automático (segmentación)
- Mejorar voz, Traductor, Reducir ruido, Aislar voz, Completar canal (audio IA)
- Adelgazar, Ojos, Dientes (retoque facial IA)
- Rostro, Cuerpo, Cabello, Párrafos (estilizado IA avanzado)

---

## 11. Bugs Conocidos

1. **Botones de barra de título** (Minimizar/Maximizar/Cerrar): no funcionan en la app empaquetada por falta de `window.ipc` en WebView2.
2. **Imágenes de trazo** (`assets/strokes/*.png`): no existen, se ocultan con `onerror`.
3. **Importación de medios**: a veces se queda en "Finalizando…" (problema con `pendingFiles`).
4. **Curvas**: el tono no se aplica por canal individual (solo Luma global).
5. **Chroma key**: el selector de color y los sliders funcionan, pero la sombra/borde de plumas son aproximaciones.

---

## 12. Flujo de Desarrollo

1. Editar archivos en `src/`.
2. Copiar a `dist\cupcraft-rust\src\`: `Copy-Item "src\*" "dist\cupcraft-rust\src\" -Force`
3. Reiniciar app: matar PID actual → `Start-Process 'dist\cupcraft-rust\cupcraft.exe'`
4. Verificar con navegador en `http://127.0.0.1:8787/`
5. Verificar sintaxis: `node --check` (copiar a `.mjs` primero)

---

## 13. Comandos Útiles

```powershell
# Ver procesos
Get-Process cupcraft

# Matar y reiniciar
Get-Process cupcraft | Stop-Process -Force
Start-Process 'dist\cupcraft-rust\cupcraft.exe' -WorkingDirectory 'C:\Users\AITOR\Desktop\cupcraft'

# Verificar sintaxis
Copy-Item "src\panels.js" "$env:TEMP\cc-panels.mjs" -Force
node --check "$env:TEMP\cc-panels.mjs"

# Ver log de la app
Get-Content "$env:LOCALAPPDATA\cupcraft\cupcraft.log" -Tail 20
```

---

## 14. Herramientas y Librerías Open-Source Utilizadas

### 14.1 Ya integradas en el proyecto

| Herramienta | Uso | Licencia | Dónde se usa |
|---|---|---|---|
| **WebAudio API** (nativa) | Modificador de voz (EQ, delay, eco, reverb) | Nativa del navegador | `renderer.js` → `buildVoiceChain`, `applyVoice` |
| **Canvas 2D API** (nativa) | Dibujado de clips, máscaras, chroma key, blur de zona | Nativa del navegador | `renderer.js` → `drawMediaLayer`, `applyMask`, `applyChromaKeyPixels` |
| **WebGL** (nativa) | Shaders de efectos (13+ firmas CapCut) | Nativa del navegador | `gl-post.js` |
| **IndexedDB** (nativa) | Almacenamiento de proyectos y medios en el navegador | Nativa del navegador | `db.js` |
| **Kokoro TTS** (kokoro-js) | Síntesis de voz neuronal en español/inglés (modelo ONNX q8, ~92 MB) | Apache-2.0 | `kokoro-tts.js` |
| **Whisper** (Xenova/whisper-tiny) | Transcripción de audio (automatic-speech-recognition) | MIT | `auto-edit.js` |
| **Symphonia** (Rust) | Decodificación de audio (MP3, WAV, AAC, etc.) | Apache-2.0/MIT | `cupcraft-audio` |
| **CPAL** (Rust) | Reproducción de audio en el sistema | Apache-2.0/MIT | `cupcraft-audio` |
| **WGPU** (Rust) | Renderizado GPU (WebGPU nativo) | MIT/Apache-2.0 | `cupcraft-render` |
| **Image crate** (Rust) | Procesamiento de imágenes (PNG, JPEG) | MIT | `cupcraft-render` |
| **Rusqlite** (Rust) | Base de datos SQLite embebida | MIT | `cupcraft-persist` |
| **Axum** (Rust) | Servidor HTTP para la UI | MIT | `cupcraft-bridge`, `cupcraft-server` |
| **Tao/WRY** (Rust) | Ventana nativa + WebView2 | MIT/Apache-2.0 | `cupcraft-desktop` |
| **TinyHTTP** (Rust) | Servidor HTTP ligero | MIT/Apache-2.0 | `cupcraft-server` |
| **Ureq** (Rust) | Cliente HTTP (descargas) | MIT/Apache-2.0 | `cupcraft-server` |
| **ONNX Runtime** (vía kokoro-js) | Inferencia de modelos neuronales en WASM | MIT | `kokoro-tts.js` |

### 14.2 Zona "Eliminar fondo" — Herramientas disponibles

#### Ya implementadas (funcionan)
| Herramienta | Tecnología | Descripción |
|---|---|---|
| **Máscara** (9 formas) | Canvas 2D `ctx.clip()` | Recorta con círculo, rectángulo, corazón, estrella, etc. + invertir |
| **Chroma key** | Canvas 2D `getImageData`/`putImageData` | Elimina un color específico píxel a píxel |
| **Eliminación de IA (zona)** | Canvas 2D `ctx.filter = blur()` + `drawImage` | Difumina una zona rectangular |
| **Eliminación automática** | Switch (sin backend) | Marcador guardado, sin segmentación real |

#### Herramientas open-source recomendadas para implementar las pendientes

| Función pendiente | Herramienta open-source recomendada | Licencia | Cómo se integraría |
|---|---|---|---|
| **Recorte automático (segmentación)** | **MediaPipe Selfie Segmentation** | Apache-2.0 | Modelo TFLite que segmenta persona/fondo en tiempo real. Se carga en WASM/JS. Output: máscara binaria por frame. |
| **Recorte automático (alternativa)** | **ONNX Runtime + U2-Net / MODNet** | MIT | Modelos de segmentación de retrato. Se ejecutan en WASM con ONNX. |
| **Recorte automático (alternativa 2)** | **TensorFlow.js + BodyPix** | Apache-2.0 | Segmentación de cuerpo/persona en el navegador. |
| **Chroma key mejorado** | **Canvas 2D + algoritmo de despill** | Nativo | Añadir despill (eliminar color derramado en bordes) y ajuste de tolerancia. |
| **Trazo (borde de recorte)** | **Canvas 2D stroke + máscara** | Nativo | Dibujar un borde alrededor de la máscara con `ctx.stroke()` sobre el contorno. |
| **Eliminación personalizada (pincel)** | **Canvas 2D + brush engine** | Nativo | Pintar una máscara con el ratón y aplicar `ctx.clip()` con la máscara pintada. |
| **Mejora de calidad (super-resolución)** | **ONNX Runtime + Real-ESRGAN** | BSD-3 | Modelo de super-resolución que escala la imagen. |
| **Estabilización** | **OpenCV.js** | Apache-2.0 | Detección de puntos de interés + transformación afín por frame. |
| **Denoise** | **Canvas 2D + filtro bilateral** | Nativo | Filtro bilateral (suavizado preservando bordes) implementado en JS. |
| **Flujo óptico** | **OpenCV.js (Farneback)** | Apache-2.0 | Cálculo de flujo óptico denso para efectos de movimiento. |
| **Face retouch (rostro)** | **MediaPipe Face Mesh** | Apache-2.0 | Detección de 468 puntos faciales para adelgazar, ojos, etc. |
| **Body slim (cuerpo)** | **MediaPipe Pose** | Apache-2.0 | Detección de pose para adelgazar cuerpo, alargar piernas, etc. |
| **Hair segmentation** | **MediaPipe Hair Segmentation** | Apache-2.0 | Segmentación de cabello para volumen/brillo/color. |
| **Text-to-speech** | **Kokoro TTS** (ya integrado) | Apache-2.0 | Ya se usa para TTS en español. |
| **Transcripción** | **Whisper** (ya integrado) | MIT | Ya se usa para transcripción de audio. |
| **Voice cloning** | **Coqui TTS** | MPL-2.0 | Síntesis de voz con clonación. |
| **Background music** | **MusicGen** (Meta) | MIT | Generación de música a partir de texto. |
| **Object removal** | **LaMa (Large Mask Inpainting)** | Apache-2.0 | Inpainting de alta calidad para eliminar objetos. |
| **Color grading** | **3D LUT** (nativo) | Nativo | Ya se soporta LUTs. Se pueden generar con herramientas como LUTCalc. |

### 14.3 Librerías de procesamiento de video (para exportación)

| Herramienta | Licencia | Uso |
|---|---|---|
| **FFmpeg** (vía Rust) | LGPL/GPL | Ya se usa en Rust para exportación de video. |
| **WebCodecs API** | Nativa del navegador | Codificación/decodificación de video en el navegador. |
| **MediaRecorder API** | Nativa del navegador | Grabación de canvas a video. |

### 14.4 Librerías de UI/interfaz

| Herramienta | Licencia | Uso |
|---|---|---|
| **Tao/WRY** (Rust) | MIT/Apache-2.0 | Ventana nativa + WebView2 (reemplaza Electron). |
| **Iconos SVG** (propios) | MIT | Iconos personalizados en `src/icons.js`. |
| **CSS custom** | MIT | Estilos propios, sin framework CSS. |

---

## 15. Resumen para Claude

### Lo que funciona (efecto real)
1. Máscara (9 formas + invertir) — Canvas 2D `ctx.clip()`
2. Chroma key (eliminación de color por píxel) — `getImageData`/`putImageData`
3. Eliminación de IA (zona) — blur de zona con `ctx.filter`
4. Viñeta, ajustes de color, HSL, retoque — CSS filters
5. Modificador de voz — WebAudio API
6. Animaciones — efectos GL locales
7. Curvas — edición + brillo/contraste
8. Denoise, estabilización, flujo óptico, etc. — efectos CSS aproximados

### Lo que falta (y herramientas open-source para implementarlo)
1. **Recorte automático** → MediaPipe Selfie Segmentation o U2-Net (ONNX)
2. **Trazo (borde de recorte)** → Canvas 2D stroke sobre contorno de máscara
3. **Eliminación personalizada (pincel)** → Canvas 2D brush engine + máscara pintada
4. **Mejora de calidad (super-resolución)** → Real-ESRGAN (ONNX)
5. **Estabilización real** → OpenCV.js (detección de puntos + transformación afín)
6. **Denoise real** → Filtro bilateral en Canvas 2D
7. **Face retouch real** → MediaPipe Face Mesh
8. **Body slim real** → MediaPipe Pose
9. **Hair segmentation** → MediaPipe Hair Segmentation
10. **Object removal real** → LaMa (Large Mask Inpainting)

### Arquitectura clave
- El frontend (JS) se comunica con el backend (Rust) vía HTTP local (puerto 8787).
- El renderizado se hace en Canvas 2D (frontend) y WebGL (shaders).
- El audio se procesa con WebAudio API (frontend) y Symphonia/CPAL (Rust).
- La persistencia usa IndexedDB (frontend) y SQLite (Rust).
- La IA usa un puente HTTP local (puerto 8790) y modelos ONNX en WASM.

### Archivos clave para modificar
- **UI/controles**: `src/panels.js` → funciones `cutoutTab`, `switchRow`, `detailRange`
- **Renderizado/efectos**: `src/renderer.js` → `applyChromaKeyPixels`, `drawAiRemove`, `applyMask`, `clipFilter`
- **Estado/propiedades**: `src/store.js` → `updateClip`, `setNestedPatch`
- **Estilos**: `src/styles.css`
- **Iconos**: `src/icons.js`
- **Sincronizar**: copiar `src/*` a `dist\cupcraft-rust\src\` y reiniciar `cupcraft.exe`

---

*Documento actualizado para revisión por Claude. Fecha: Noviembre 2026.*
