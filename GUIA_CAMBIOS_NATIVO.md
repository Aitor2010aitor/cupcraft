# Guía de cambios: cupcraft nativo

Documento de trabajo para reconstruir cupcraft como aplicación nativa de Windows, con la misma interfaz que la versión web actual, y para elegir herramientas open source para eliminación de fondo y el panel de Detalles.

Estado: borrador. Se basa en `Documentación Completa de CUPCRAFT` (archivo de contexto que aportó el usuario) y en el código público de GitHub (rama `0.4.5`). **La versión de trabajo actual del usuario no está en GitHub**: sus módulos (`panels.js` con pestañas, `renderer.js`, `capcut-*`, `ai-bridge.js`, crates `audio`, `render`, `tts`, `bridge`, `capcut`) deben revisarse antes de dar nada por cerrado.

---

## 1. Reglas del proyecto

- Aplicación nativa de escritorio para Windows. Sin navegador, sin WebView2, sin servidor HTTP local, sin puertos (ni 8787 ni 8790).
- La interfaz debe verse igual que la versión web actual. La web sirve como referencia visual: se capturan pantallas y se comparan.
- Stack propuesto: Rust, Slint (interfaz), wgpu/WGSL (efectos y shaders), FFmpeg (vídeo y audio), SQLite (proyectos), ONNX Runtime (modelos de IA locales).
- Puente IA: tubería con nombre de Windows, sin puerto.
- Licencia del proyecto: MIT (según `Cargo.toml`). Cualquier dependencia debe ser compatible con distribuir la app. Ver sección 6.
- No subir cambios a GitHub sin pedirlo.

---

## 2. Inventario de funciones (según el documento de contexto)

### 2.1 Panel Detalles, por pestañas

| Pestaña | Subsecciones | Estado declarado |
|---|---|---|
| Video | Básico, Eliminar fondo, Máscara, Retoque | Mixto (ver 2.2) |
| Audio | Básico, Modificador de voz | Funciona en parte; Normalizar, Mejorar voz, Reducir ruido, Aislar voz: bloqueados |
| Velocidad | Estándar, Curva, Efectos de velocidad | Estándar funciona; Curva bloqueada (PRO) |
| Animación | Entrada, Salida, Combinado | Funciona (mapeo a efectos GL locales) |
| Ajustar | Básico, HSL, Curvas, Círculo cromático, Máscara | Básico, Curvas y HSL funcionan; Combinación de colores bloqueada |
| Estilizado de IA | Rostro, Cuerpo, Cabello, Párrafos, Párrafos corporales | Sin efecto real; requieren IA en la nube |

### 2.2 Video → Eliminar fondo (detalle)

| Función | Estado declarado | Necesita |
|---|---|---|
| Eliminación automática (switch) | Solo guarda un marcador | Segmentación local (sección 3) |
| Trazo (estilo, color, tamaño, espaciado) | Solo UI | Contorno de la máscara |
| Eliminación personalizada (pincel, pincel rápido, lazo, borrador) | Solo UI | Editor de máscara propio |
| Chroma key (color, intensidad, sombra, pluma, limpiar borde) | Funciona; sombra y pluma son aproximaciones | Despill y refinado de bordes |
| Recortar con máscara | Funciona | Ya implementado |

Otras funciones de Video → Básico que no tienen efecto real: Estabilización, Mejorar calidad, Reducir ruido, Flujo óptico, Ampliación IA, Remix IA, Contacto visual, Sincronización labial, Movimiento IA, Seguimiento de cámara. Los que dependen de IA en la nube quedan bloqueados.

---

## 3. Eliminación de fondo local: opciones open source

Cada herramienta está marcada con su licencia. **Verificado** significa que consulté el repositorio. **Revisar** significa que hay que comprobarla antes de usarla.

### 3.1 Tabla de decisión

| Herramienta | Qué hace | Licencia | Estado | Recomendación |
|---|---|---|---|---|
| **BackgroundMattingV2** (github.com/PeterL1n/BackgroundMattingV2) | Matting de vídeo en tiempo real. Necesita una foto del fondo sin persona | MIT | Verificado | Buena opción para vídeo con cámara fija. Requiere foto de fondo |
| **RobustVideoMatting** (github.com/PeterL1n/RobustVideoMatting) | Matting de vídeo sin foto de fondo | **GPL-3.0** | Verificado | **Evitar** salvo que el proyecto pase a GPL. Es copyleft |
| **rembg** (github.com/danielgatis/rembg) | Quitar fondo por imagen, con varios modelos | MIT (código) | Verificado | El modelo por defecto (`bria-rmbg`) requiere acuerdo de pago para uso comercial. **No usarlo como modelo por defecto**. Revisar licencias de `u2net` e `isnet` |
| **U²-Net** (github.com/xuebinqin/U-2-Net) | Detección de objeto principal, base de rembg | Revisar | Pendiente | Candidato para recorte de persona. Confirmar licencia de los pesos |
| **MediaPipe Selfie Segmentation** | Segmentación de persona, muy rápido | Apache-2.0 | Revisar | Candidato principal para recorte automático en tiempo real |
| **MODNet** | Matting de retrato | Revisar | Pendiente | Alternativa para bordes finos (pelo) |
| **Segment Anything / MobileSAM** | Máscara a partir de clics (equivale a un lazo o pincel inteligente) | Revisar | Pendiente | Candidato para la herramienta de eliminación personalizada |

### 3.2 Recomendación por función

| Función de la app | Herramienta sugerida | Motivo |
|---|---|---|
| Eliminación automática | MediaPipe Selfie Segmentation, o U²-Net si MediaPipe no da la calidad | Rápido, local, sin nube |
| Matting de pelo y bordes | MODNet o BackgroundMattingV2 | Bordes más finos que una segmentación binaria |
| Pincel, lazo y borrador | Propio (Rust) + SAM/MobileSAM para clics | El pincel es edición de máscara, no necesita IA |
| Chroma key y despill | Propio (WGSL) | Algoritmo simple y ya existe en la versión web |
| Trazo del recorte | Propio: distancia al borde + contorno | Sin dependencias |
| Refinado de máscara | Propio: filtro guiado y pluma | Sin dependencias |

### 3.3 Cómo ejecutar los modelos en Rust

- Exportar el modelo a **ONNX** y ejecutarlo con **ONNX Runtime**. En Rust, el crate `ort` (bindings de ONNX Runtime) con el proveedor DirectML para usar la GPU de Windows. **Revisar licencia del crate.**
- Alternativa sin dependencias nativas: `tract` (Rust puro, ONNX). Más lento, pero más sencillo de distribuir. **Revisar.**
- Los pesos del modelo se guardan junto a la app, no dentro del binario, y se comprueba su licencia antes de incluirlos.

---

## 4. Otras herramientas para el panel Detalles

| Área | Opción | Licencia | Notas |
|---|---|---|---|
| Decodificación de audio | Symphonia (Rust) | Apache-2.0 / MIT | Ya está en `cupcraft-media` |
| Reproducción de audio | CPAL (Rust) | Apache-2.0 | Ya citada en el documento |
| Cadena de voz (EQ, eco, delay) | Propio sobre CPAL, o Rubber Band (GPL, revisar) | Revisar | WebAudio no existe fuera del navegador |
| Velocidad con tono preservado | FFmpeg `atempo` | LGPL/GPL (revisar build) | Más simple que implementarlo |
| Shaders de ajuste, HSL, curvas | WGSL con wgpu | MIT / Apache-2.0 | Sustituye a `gl-post.js` |
| Máscaras (9 formas) | Propio en WGSL | Propio | Ya existe en la versión web |
| Estabilización, flujo óptico | OpenCV (`opencv` crate) | Apache-2.0 | Pesado; posponer |
| Super-resolución (Mejorar calidad) | Real-ESRGAN en ONNX | BSD-3 | Revisar pesos |
| Voz TTS local | Piper (ya integrado) o Kokoro | Apache-2.0 (Kokoro, según el documento) | Revisar |
| Transcripción | Whisper (whisper-tiny) | MIT | Ya lo usa `auto-edit.js` |
| Retoque facial, pose, cabello | MediaPipe Face Mesh / Pose / Hair | Apache-2.0 | Fase posterior |

---

## 5. Orden de trabajo sugerido

1. Recibir la versión actual del usuario (zip o rama subida). Sin ella, no se toca la interfaz.
2. Levantar la web actual para capturar las pantallas de referencia: inicio, editor, panel Detalles en cada pestaña, y la zona Eliminar fondo.
3. Escribir las especificaciones de cada pantalla con esa versión, no con la de GitHub.
4. Reconstruir la interfaz en Slint pantalla a pantalla. Comparar cada una con su captura.
5. Implementar el motor de máscaras: primero chroma key y máscaras (ya existen), después segmentación automática con un solo modelo.
6. Probar con vídeos reales en Windows (no solo en Linux).
7. Pruebas automáticas de: decodificación, máscara, chroma key, exportación.

---

## 6. Derechos y licencias

- Los recursos derivados de CapCut (152 MB de LUTs en `assets/luts`, catálogos de efectos) requieren revisar derechos antes de distribuir. Pendiente de decisión del usuario. No es asesoramiento jurídico: conviene consultar a un abogado.
- Antes de incluir cualquier modelo, comprobar su licencia y la de sus pesos. Los modelos suelen tener licencia distinta del código.
- No usar RVM ni rembg con `bria-rmbg` en una versión distribuida sin revisar antes sus condiciones.
- Falta un archivo LICENSE en el repositorio.

---

## 7. Decisiones pendientes para el usuario

1. Enviar la versión actual (zip o rama) para poder reconstruir la interfaz.
2. Confirmar si el proyecto seguirá en MIT o pasará a otra licencia (condiciona el uso de RVM).
3. Decidir qué hacer con los recursos de CapCut.
4. Elegir el modelo de segmentación principal tras una prueba con vídeos reales.

---

## 8. Fuentes

- BackgroundMattingV2: https://github.com/PeterL1n/BackgroundMattingV2 (MIT, verificado)
- RobustVideoMatting: https://github.com/PeterL1n/RobustVideoMatting (GPL-3.0, verificado)
- rembg: https://github.com/danielgatis/rembg (MIT; modelo por defecto con licencia de pago, verificado)
- Resto de licencias: pendientes de verificar antes de usar.