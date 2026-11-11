# Context Checkpoint — cupcraft (pre-format PC)

Branch: `version.04` · Last commit: `fd511c7` (Rust UI completa compilando) · Remote: https://github.com/Aitor2010aitor/cupcraft.git

## Objetivo
Reverse-engineer openCUP → CapCut feature-for-feature. Desktop app + GitHub releases. Reescritura completa en **Rust** (9 fases), UI-first (egui/eframe). Visible name **cupcraft**; `openCUP` solo técnico.

## Decisiones clave
- C++ Win32+WebView2 = v0.4.x (branch version.04 tip anterior a2b7824 era Fase0+1).
- Rust rewrite: UI en egui/eframe 0.37 (timeline pintado a mano), diálogos rfd 0.15.
- Filtros: solo panel Filtros; línea propia 82 filtros/12 cat/17 GL; applyFilter = exactamente un filtro por clip.
- Commits autor: `AITOR2010 <194286048+Aitor2010aitor@users.noreply.github.com>`.
- No blanket `git add -A`; stage paths explícitos.
- No editar repo con PowerShell Set-Content (BOM). Usar write/edit/Node.

## Fases
0 Cimientos ✅, 1 Núcleo ✅, 2 Medios (ffmpeg/symphonia) ⏳, 3 Render (wgpu/WGSL) ⏳, 4 UI (en progreso), 5 Paneles, 6 Audio/TTS, 7 Exportación, 8 Puente IA, 9 Paridad tests.

## Workspace Rust
`rust/` (Cargo workspace). Crates:
- `cupcraft-core`: model, timeline, history, filters (42 tests green).
- `cupcraft-persist`: store (SQLite), project_file (.opencup.json).
- `cupcraft-app`: egui/eframe UI (compila, warnings).

## Archivos UI escritos
`main.rs`, `theme.rs`, `format.rs`, `paint.rs`, `text_styles.rs`, `text_styles_data.json`, `app.rs`, `home.rs`, `editor.rs`, `panels.rs`, `timeline.rs`.

## API gotchas egui 0.37
- `App::ui(&mut self, ui: &mut Ui, frame)` (no update).
- Paneles: `egui::Panel::top/bottom/left/right(id)` (no TopBottomPanel/SidePanel). CentralPanel::default().
- `Panel::show(ui, ...)` toma `&mut Ui`.
- `FontFamily::Name(Arc<str>)`.
- `ctx.egui_wants_keyboard_input()`.
- `rfd::FileDialog::pick_file/save_file/pick_files` sync.

## Core ops
`add_media_clip`, `add_text_clip`, `apply_filter`, `update_clip`, `move_clip`, `trim_clip`, `split_at`, `duplicate_clip`, `remove_clip`, `Editor::{open,project,project_mut,selection,select,mutate,mutate_silent,undo,redo,can_undo,can_redo}`.

## Almacén
`%LOCALAPPDATA%\openCUP\store.sqlite`, fallback `open_in_memory()`.

## Tests JS (sin tocar src/ durante Rust)
thumb-check 21/21, filter-bar-test 49/49, run-all-tests 12/12, glfilter-paint 18/18.

## Próximos pasos
- Profundizar paneles (media import, text style apply, filter cards con scene).
- Fase 2 medios (ffmpeg/symphonia), Fase 3 render (wgpu/WGSL).
- Fase 6 audio/TTS, Fase 7 export, Fase 8 IA bridge.
- Fase 9 paridad de tests (JS→Rust).
- Ejecutar `cargo test` completo y verificar binary visualmente (sin screenshot tool en este entorno: usar PowerShell System.Drawing).

## Restauración tras formateo
1. Instalar Rust (rustup), cargo, MSVC Build Tools, Windows SDK, Inno Setup 6.7.4.
2. `git clone https://github.com/Aitor2010aitor/cupcraft.git && cd cupcraft && git checkout version.04`
3. `cd rust && cargo build -p cupcraft-app`
4. Binario: `rust/target/debug/cupcraft-app.exe`
5. Node para tests JS: `cd .. && npm test` (si hay package.json).
6. No redistribuir binarios CapCut. Solo assets originales openCUP.
