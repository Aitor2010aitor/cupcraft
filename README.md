<div align="center">

<img src="rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**A fast, local, native video editor for your desktop. Written 100% in Rust.**

[![English](https://img.shields.io/badge/English-0a84ff?style=for-the-badge)](README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](docs/i18n/README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](docs/i18n/README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-2d2d2d?style=for-the-badge)](docs/i18n/README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-2d2d2d?style=for-the-badge)](docs/i18n/README.pt-BR.md)
[![Deutsch](https://img.shields.io/badge/Deutsch-2d2d2d?style=for-the-badge)](docs/i18n/README.de.md)
[![日本語](https://img.shields.io/badge/%E6%97%A5%E6%9C%AC%E8%AA%9E-2d2d2d?style=for-the-badge)](docs/i18n/README.ja.md)
[![Русский](https://img.shields.io/badge/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9-2d2d2d?style=for-the-badge)](docs/i18n/README.ru.md)

[![Latest release](https://img.shields.io/github/v/release/Aitor2010aitor/cupcraft?style=flat-square&color=00c1cd)](https://github.com/Aitor2010aitor/cupcraft/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-egui%20%2B%20wgpu-orange?style=flat-square)
![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey?style=flat-square)

</div>

cupcraft is a desktop video editor inspired by the workflow and dark interface of CapCut. It is a single native executable: no Electron, no WebView, no Node.js, no server. Everything runs on your machine.

> [!NOTE]
> cupcraft is an original implementation. It does not include binaries, logos, fonts or any proprietary assets from CapCut, and it is not affiliated with or endorsed by ByteDance.

## ✨ Features

- **Multitrack timeline** — video, audio, text, stickers, effects and filters. Split, trim, duplicate, snapping, keyframes and 80-step undo/redo.
- **GPU compositing** — `wgpu` shaders for effects, LUT filters, adjustments and transitions, with a CPU fallback.
- **Text and stickers** — text styles, system and imported fonts, animated stickers (GIF / WebP) and clip animations.
- **Audio** — real-time multitrack mixing with fades, waveforms and local text-to-speech.
- **Local AI tools** — background removal (ONNX) and automatic subtitles (Whisper), run on your computer.
- **Export** — MP4 (H.264 / H.265) and WebM (VP9) through FFmpeg, using hardware encoders (NVENC, QSV, AMF) when available.
- **CapCut import** — reads CapCut drafts (read-only) and converts them into cupcraft projects.
- **Safe projects** — projects are autosaved to a local SQLite database, and closing the app asks whether you want to save.
- **Self-updating** — new versions published on GitHub are offered inside the app.
- **Private** — no account, no cloud, no telemetry. The only network traffic is the update check and downloads you start yourself.

## 🆕 What's new in v0.4.12.2

- **Complete translations** — panels, dialogs, inspector, timeline tooltips, toasts and effect/transition names follow the chosen language (English fallback for anything a pack does not cover).

## 🆕 What's new in v0.4.12

- **Keyframe diamonds** — Scale, Position X/Y, Rotation and Opacity now have a ◆ button: click it to add a keyframe at the playhead and animate the property over time. The preview and dragging on the stage follow the keyframes.
- **App language** — a language selector in Settings with Spanish, English, French, German, Portuguese (Brazil), Russian, Japanese and Chinese.
- **Your own language** — export a JSON template with every app text, translate it (by hand or with an AI) and import it back to switch the whole app.
- **More complete translations**: panels, dialogs, inspector, timeline tooltips, toasts and effect/transition names now follow the chosen language (English fallback for any text a pack does not cover).

## 🆕 What's new in v0.4.11.1

- **AI subtitle generator window** — the new **Generate subtitles with AI…** button opens a window where you pick the clip, the voice language and a karaoke effect (each with an animated preview), choose whether to replace the current subtitles, and see if the Whisper model is ready.
- **Whisper download at the top** — the engine/model download button now sits at the very top of the Subtitles panel.
- **Voice panel fixed** — you can always type the text, pick a voice style and add automatic karaoke subtitles. If the Piper engine is missing, one button downloads it with a Spanish voice.
- **More karaoke effects and any color** — color, jump, big word, color box, box + jump, underline and progressive, each with an animated preview, plus a color picker.
- **Fix: some Piper voices failed** with a phoneme error — Amy, Joe and Ald now work.
- **More voices** — Spain Spanish voices and three Loquendo-style presets (an approximation: the real Loquendo voices are commercial and can't be bundled).
- **App language** — switch the interface between Spanish and English in Settings; your choice is remembered.
- **More accurate subtitles** — Whisper Base model, cleaner audio and tighter timing. Text to speech is now labelled Beta.

## 📦 Download

Get the latest build from the [**Releases page**](https://github.com/Aitor2010aitor/cupcraft/releases/latest).

| Platform | File | Notes |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | Installer, no administrator rights needed. |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | Portable executable. |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.deb` | Installer for Debian / Ubuntu / Linux Mint (double-click, or `sudo apt install ./cupcraft-linux-x86_64.deb`). Adds cupcraft to the app menu. |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | Portable: extract and run `./cupcraft`. |

> [!WARNING]
> The macOS and Linux builds are **experimental**. Some tools (Piper voices and CapCut draft import) still rely on Windows components and are not available there yet.

> [!IMPORTANT]
> The binaries are not code-signed yet. On Windows, SmartScreen may show a warning (**More info → Run anyway**). On macOS, remove the quarantine flag once:
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

Exporting needs [FFmpeg](https://ffmpeg.org/download.html) available on your `PATH` (or in the cupcraft data folder).

## 🔄 Updates

When you publish a new release, cupcraft detects it on startup and shows a card with **Update now / Later**. The download is verified with SHA-256, the executable is replaced, your project is saved and the app restarts.

> [!TIP]
> It works without the GitHub API and without tokens: the app only reads public files from the latest release. Details in [docs/RELEASING.md](docs/RELEASING.md).

## 🛠️ Build from source

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

The binary is created at `rust/target/release/cupcraft-app`. On Windows you can also double-click `compilar.bat`.

> [!NOTE]
> Linux needs the development packages `libasound2-dev`, `libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libssl-dev` and `pkg-config`.

## 🗂️ Where is my data?

| OS | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 Architecture

A Rust workspace split into small crates (`cupcraft-core`, `-persist`, `-render`, `-media`, `-audio`, `-capcut`, `-tts`, `-bridge` and the `-app` binary). See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## 🤝 Credits

| | Role |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | Creator and maintainer |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | Co-author — AI pair programmer |

## 📄 License

[MIT](LICENSE) © 2026 AITOR2010
