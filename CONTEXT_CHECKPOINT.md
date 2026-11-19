# Context Checkpoint — cupcraft

Branch: `version.04` · Last commit: `fd511c7` · Remote: https://github.com/Aitor2010aitor/cupcraft.git

## Objetivo
Reverse-engineer openCUP → CapCut feature-for-feature. Desktop app + GitHub releases. Reescritura completa en **Rust**, UI-first. Visible name **cupcraft**; `openCUP` solo técnico.

## Estado de Fases
0 Cimientos ✅, 1 Núcleo ✅, 2 Medios (ffmpeg/symphonia) ⏳, 3 Render (wgpu/WGSL) ⏳, 4 UI (en progreso), 5 Paneles, 6 Audio/TTS, 7 Exportación, 8 Puente IA, 9 Paridad tests.

## Cambios recientes realizados
- ✅ **Previsualización visual interactiva en Hover**: Al pasar el cursor por encima de cualquier tarjeta de transición en el panel, se reproduce una simulación animada fluida a 60 FPS mostrando en vivo cómo actúa la transición (fades, rebotes, zooms, golpes, fuegos, relámpagos, glitches, barridos, etc.).
- ✅ **10 Nuevas Transiciones de Drift (CutWire Studios)**:
  - Voronoi Shatter (`drift-voronoi`), Plasma Burn (`drift-plasma`), Liquid Smudge (`drift-liquid`), Matrix Rain (`drift-matrix`), Ink Bleed (`drift-ink`), Analog Interference (`drift-interference`), Honeycomb Hexagon (`drift-honeycomb`), Bubbling Dissolve (`drift-bubbles`), VHS Scanline (`drift-vhs`), Cross Zoom Swirl (`drift-swirl`).
  - Integradas con categoría propia "Drift", renderizado Canvas2D y miniaturas animadas.
- ✅ **20 Nuevas Transiciones Estilo CapCut**:
  - **Impacto y golpes**: Golpe Slam (`slam-hit`), Sacudida Punch (`shake-punch`), Terremoto (`earthquake`).
  - **Elementos y energía**: Llamarada Fuego (`fire-burst`), Rayo Eléctrico (`lightning-flash`), Humo Volumétrico (`smoke-cloud`).
  - **Tendencia y Viral**: Glitch RGB Split (`glitch-rgb`), Pulso Cromático (`rgb-pulse`), Mosaico Pixel (`pixelate`).
  - **Movimiento rápido**: Latigazo ← (`whip-left`), Latigazo → (`whip-right`).
  - **Zoom dinámico**: Zoom Snap Rebote (`zoom-snap`), Giro 360° Golpe (`spin-slam`).
  - **Distorsión**: Vórtice Espiral (`swirl-warp`), Caleidoscopio (`kaleido`).
  - **Luz y Flashes**: Destello Anamórfico (`lens-flare`), Estroboscopio (`strobe`).
  - **Formas y Cortinilla**: Iris Corazón (`heart-iris`), Iris Estrella (`star-iris`), Pliegue Página (`page-curl`).
  - Renderizado Canvas2D de alto rendimiento y miniaturas CSS en `src/styles.css` y `src/renderer.js`.
- ✅ **TTS-Abierto (Piper TTS neuronal 100% local y offline)**:
  - Motor Piper TTS integrado en `tools/piper/` con modelos en español (`davefx`, `sharvard`, `carlfm`).
  - Endpoints en Rust `/api/tts/voices` y `/api/tts/generate`.
  - Panel de texto conectado en `src/panels.js` y `src/tts-abierto.js`.
- ✅ **Barra de título única con versión**: Eliminado el marco redundante y configurado el nombre dinámico `cupcraft v0.1.0`.
- ✅ **Ciclo de vida de proyecto como CapCut**: Al cerrar o salir del editor, se retorna automáticamente a la pantalla de inicio sin cerrar el servidor local.
- ✅ **Zoom y Scroll por rueda**: Soporte fluido de `wheel` en la barra de tiempo.
- ✅ **Playhead mejorado**: Cabezal amarillo dorado visible (#ffdd00) con indicación visual clara.
- ✅ **Compilado y Desplegado**: Binario Rust release actualizado y ejecutándose en PID activo.

## Próximos Pasos
1. Miniaturas de fotogramas (filmstrip) en los clips de la línea de tiempo.
2. Implementación de efectos tipo 48 en el renderizador visual.
3. Análisis con Ghidra/REA para stickers PAG y tracking.
