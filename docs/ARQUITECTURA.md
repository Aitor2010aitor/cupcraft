# Arquitectura de cupcraft (App Nativa de Escritorio)

Este documento describe la arquitectura modular del proyecto, la distribución de responsabilidades en crates de Rust y el estado de avance de la reconstrucción desde cero.

---

## 1. Visión General

cupcraft es un editor de vídeo nativo para Windows y plataformas de escritorio construido íntegramente en **Rust**, con renderizado acelerado por GPU (**wgpu + WGSL**) e interfaz gráfica nativa (**egui/eframe**). 

No utiliza tecnologías web (ni HTML, ni CSS, ni JavaScript, ni Electron, ni WebView2, ni Node.js).

---

## 2. Mapa de Crates del Workspace

```
rust/crates/
├── cupcraft-core/     # Lógica pura, modelo de datos, timeline, historial (80 pasos), keyframes y filtros
├── cupcraft-persist/  # Almacenamiento SQLite (rusqlite), lectura y escritura de proyectos y medios
├── cupcraft-render/   # Pipeline gráfico wgpu, composición de capas, shaders WGSL y transiciones
├── cupcraft-media/    # Decodificación y codificación con FFmpeg, MP4 (H.264/H.265), formas de onda
├── cupcraft-audio/    # Reproducción y mezcla de audio con cpal + symphonia
├── cupcraft-capcut/   # Lector de borradores de CapCut (solo lectura, draft_content.json)
├── cupcraft-bridge/   # Servidor HTTP axum con token local para integración con agentes de IA
├── cupcraft-tts/      # Síntesis de voz local mediante Piper TTS en español
├── cupcraft-ui/       # Interfaz de usuario en egui/eframe (paneles, timeline dibujada, transportes)
└── cupcraft-app/      # Binario final unificado (.exe)
```

---

## 3. Estado de Avance por Fases

| Fase | Descripción | Estado |
|---|---|---|
| **Fase 0** | Lectura y análisis completo de la base JS vieja (`ANALISIS.md`, `MODELO.md`, `FUNCIONES.md`, `UI-TOKENS.md`, `DUDAS.md`). | **Completada** |
| **Fase 0.5** | Copias de seguridad del proyecto (`.zip` en Escritorio) y datos de AppData (`cupcraft-datos-backup`), migración del código viejo a `legacy/`. | **Completada** |
| **Fase 1** | Núcleo ampliado en Rust (`cupcraft-core`): modelo completo, keyframes, congelado de fotograma, separación de audio, stickers, efectos y subtítulos. 69 tests unitarios pasando. | **Completada** |
| **Fase 2** | Persistencia nativa (`cupcraft-persist`) con SQLite WAL, medios en disco y migración transparente de proyectos viejos de IndexedDB/WebView2. | **Completada** |
| **Fase 3** | Motor de composición en GPU con `wgpu 30` y shaders WGSL (`cupcraft-render`): 13 efectos clásicos, 8 efectos firma CapCut, shader LUT 512×512, transiciones (67 tipos) y composición de capas. | **Completada** |
| **Fase 4** | Integración de audio y vídeo con FFmpeg, cpal y symphonia (`cupcraft-media` y `cupcraft-audio`): decodificación nativa (MP3, WAV, AAC, OGG, FLAC), mezcla multipista con fundidos, motor CPAL en tiempo real, extracción de fotogramas/tiras de película y exportación acelerada (H.264, H.265, WebM con NVENC/QSV/AMF). | **Completada** |
| **Fase 5** | Interfaz gráfica nativa con `egui/eframe` (`cupcraft-ui`): diseño oscuro con tokens exactos, timeline multipista fluida, previsualización, paneles y atajos de teclado. | Siguiente |
| **Fase 6** | Integraciones accesorias (CapCut, AI Bridge, Piper TTS). | Pendiente |
| **Fase 7** | Empaquetado final de un solo `.exe` nativo y pruebas de paridad. | Pendiente |
