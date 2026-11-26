# Architecture

cupcraft is a Rust workspace (`rust/`). The `cupcraft-app` crate builds the final native executable (egui/eframe on top of wgpu); everything else is a library crate with a single responsibility.

```
rust/crates/
├── cupcraft-core      Project model, tracks, clips, history (80 steps), keyframes, filters
├── cupcraft-persist   SQLite store (rusqlite), project files, legacy migration
├── cupcraft-render    wgpu pipeline: layer compositing, WGSL shaders, transitions
├── cupcraft-media     FFmpeg probing, decoding, waveforms and export (H.264 / H.265 / VP9)
├── cupcraft-audio     Real-time multitrack mixer (cpal + symphonia)
├── cupcraft-capcut    Read-only CapCut draft reader and converter
├── cupcraft-tts       Local text-to-speech (Piper)
├── cupcraft-bridge    Local HTTP bridge (127.0.0.1 + token) for AI agents
└── cupcraft-app       The application: UI, panels, timeline, services, auto-updater
```

> [!NOTE]
> `cupcraft-desktop`, `cupcraft-server` and `cupcraft-ui` belong to earlier prototypes (WebView2 and Slint). They are kept for reference and are not part of the release build.

## Application layout (`cupcraft-app/src`)

| Module | Purpose |
|---|---|
| `app.rs` | Global state (`CupApp`), jobs, project lifecycle, close/save flow |
| `home.rs`, `editor.rs` | Home screen and editor shell (custom title bar) |
| `timeline.rs`, `panels.rs`, `inspector*.rs` | Timeline and side panels |
| `dialogs.rs` | Modal dialogs (new project, export, settings, save on close…) |
| `fx_*.rs`, `clip_anim.rs`, `trans_preview.rs` | Effects, animations and previews |
| `services/` | Media import, video decoding, CPU/GPU compositing, export, Whisper, background matting |
| `services/updater.rs` | Self-update from GitHub Releases |

## Project lifecycle

- Every edit is autosaved to the SQLite store.
- When a project is opened, a snapshot is kept. Closing the app asks *"Do you want to save the project?"*:
  - **Yes** → asks for a name, saves and exits.
  - **No** → a project created in this session is deleted; an existing one is restored to the snapshot taken when it was opened.
  - **Cancel** → back to the editor.

## Data folders

`%LOCALAPPDATA%\openCUP` (Windows), `~/Library/Application Support/openCUP` (macOS) or `~/.local/share/openCUP` (Linux).
