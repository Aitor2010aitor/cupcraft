# Context Checkpoint — cupcraft

Branch: `version.04` · Last commit: `fd511c7` · Remote: https://github.com/Aitor2010aitor/cupcraft.git

## Objetivo
Reverse-engineer openCUP → CapCut feature-for-feature. Desktop app + GitHub releases. Reescritura completa en **Rust**, UI-first. Visible name **cupcraft**; `openCUP` solo técnico.

## Estado de Fases
0 Cimientos ✅, 1 Núcleo ✅, 2 Medios (ffmpeg/symphonia) ⏳, 3 Render (wgpu/WGSL) ⏳, 4 UI (en progreso), 5 Paneles, 6 Audio/TTS, 7 Exportación, 8 Puente IA, 9 Paridad tests.

## Cambios recientes realizados
- ✅ **Barra de título única con versión**: Eliminado el marco redundante y configurado el nombre dinámico `cupcraft v0.1.0`.
- ✅ **Ciclo de vida de proyecto como CapCut**: Al cerrar o salir del editor, se retorna automáticamente a la pantalla de inicio sin cerrar el servidor local.
- ✅ **Zoom y Scroll por rueda**: Soporte fluido de `wheel` en la barra de tiempo.
- ✅ **Playhead mejorado**: Cabezal amarillo dorado visible (#ffdd00) con indicación visual clara.
- ✅ **Compilado y Desplegado**: Binario Rust release actualizado y ejecutándose en PID activo.

## Próximos Pasos
1. Miniaturas de fotogramas (filmstrip) en los clips de la línea de tiempo.
2. Implementación de efectos tipo 48 en el renderizador visual.
3. Análisis con Ghidra/REA para stickers PAG y tracking.
