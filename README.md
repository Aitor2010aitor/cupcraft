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

## 🆕 What's new in v0.4.11

- **Linux: subtitles (Whisper) now install** — pressing install shows the system password prompt (`pkexec`) to add what is missing (compiler, cmake and FFmpeg), then builds the Whisper engine on your PC. It takes a few minutes the first time.
- **Linux: FFmpeg is detected** — cupcraft now finds `ffmpeg` on your system (`/usr/bin`, `PATH`), so exporting works after `sudo apt install ffmpeg`.

## What's new in v0.4.10.1

- **"What's new" after updating** — when you press Update and cupcraft restarts, a window shows the version you just installed and what changed. Close it with the X and carry on.

## What's new in v0.4.10

- **Linux installer (.deb)** — a package for Debian, Ubuntu and Linux Mint: double-click it and cupcraft is installed system-wide, with its icon in the app menu.
- **Updates on Linux ask for permission** — when cupcraft is installed in `/usr/bin`, the in-app updater shows the system password prompt (`pkexec`) to replace the program. If you cancel, nothing changes.
- **Fix: update notice that never went away on Linux** — after updating, the app relaunched the old version, so the "new version available" card kept coming back. It now relaunches the new one.

## What's new in v0.4.9

- **TikTok-style karaoke subtitles** — the whole line is visible and the word being spoken is highlighted (5 colors, plus an optional "pop" jump). Works with Whisper word timings, manual subtitles, SRT and generated voice.
- **Voice styles for text to speech** — squirrel, young, deep, giant, robot, radio and echo, applied locally with FFmpeg.
- **More voices** — download extra Piper voices (Spanish and English) from the Voice panel, only when you click the button.
- **Auto captions for generated voice** — text to speech can add karaoke subtitles in the same step.

## What's new in v0.4.8.1

- **Right-click menu on clips** — Copy, Cut, Paste, Duplicate, Delete, Split at the playhead and Extract audio, right on the timeline.
- **Move clips to any layer** — drag any clip (video, audio, text, stickers…) up or down to any track. Dragging above the top track creates a new layer.
- **Layer order** — the track at the top of the timeline is now drawn on top, as in CapCut.
- **Bigger toolbar icons** — icons are scaled up so they fit their buttons better.

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
