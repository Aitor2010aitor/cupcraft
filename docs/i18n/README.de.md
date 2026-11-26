<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**Ein schneller, lokaler, nativer Desktop-Videoeditor. Zu 100 % in Rust geschrieben.**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-2d2d2d?style=for-the-badge)](README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-2d2d2d?style=for-the-badge)](README.pt-BR.md)
[![Deutsch](https://img.shields.io/badge/Deutsch-0a84ff?style=for-the-badge)](README.de.md)
[![日本語](https://img.shields.io/badge/%E6%97%A5%E6%9C%AC%E8%AA%9E-2d2d2d?style=for-the-badge)](README.ja.md)
[![Русский](https://img.shields.io/badge/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9-2d2d2d?style=for-the-badge)](README.ru.md)

[![Latest release](https://img.shields.io/github/v/release/Aitor2010aitor/cupcraft?style=flat-square&color=00c1cd)](https://github.com/Aitor2010aitor/cupcraft/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](../../LICENSE)
![Rust](https://img.shields.io/badge/Rust-egui%20%2B%20wgpu-orange?style=flat-square)
![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey?style=flat-square)

</div>

<img width="1024" height="768" alt="cupcraft" src="https://github.com/user-attachments/assets/703ab96c-4646-4207-ae87-d8cc019c4994" />

<img width="1686" height="934" alt="cupcraft editor" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

cupcraft ist ein Desktop-Videoeditor, inspiriert vom Arbeitsablauf und der dunklen Oberfläche von CapCut. Er besteht aus einer einzigen nativen ausführbaren Datei: kein Electron, kein WebView, kein Node.js, kein Server. Alles läuft auf Ihrem Rechner.

> [!NOTE]
> cupcraft ist eine eigenständige Implementierung. Es enthält keine Binärdateien, Logos, Schriften oder proprietären Ressourcen von CapCut und steht in keiner Verbindung zu ByteDance.

## ✨ Funktionen

- **Mehrspur-Timeline** — Video, Audio, Text, Sticker, Effekte und Filter. Teilen, Kürzen, Duplizieren, Einrasten, Keyframes und 80 Schritte Rückgängig/Wiederholen.
- **GPU-Compositing** — `wgpu`-Shader für Effekte, LUT-Filter, Anpassungen und Übergänge, mit CPU-Fallback.
- **Text und Sticker** — Textstile, System- und importierte Schriften, animierte Sticker (GIF / WebP) und Clip-Animationen.
- **Audio** — Echtzeit-Mehrspurmischung mit Überblendungen, Wellenformen und lokaler Sprachsynthese.
- **Lokale KI-Werkzeuge** — Hintergrundentfernung (ONNX) und automatische Untertitel (Whisper), direkt auf Ihrem Computer.
- **Export** — MP4 (H.264 / H.265) und WebM (VP9) über FFmpeg, mit Hardware-Encodern (NVENC, QSV, AMF), sofern vorhanden.
- **CapCut-Import** — liest CapCut-Entwürfe (nur lesend) und wandelt sie in cupcraft-Projekte um.
- **Sichere Projekte** — automatische Speicherung in einer lokalen SQLite-Datenbank; beim Schließen fragt die App, ob gespeichert werden soll.
- **Selbstaktualisierend** — neue, auf GitHub veröffentlichte Versionen werden direkt in der App angeboten.
- **Privat** — kein Konto, keine Cloud, keine Telemetrie. Der einzige Netzwerkverkehr sind die Update-Prüfung und Downloads, die Sie selbst starten.

## 📦 Download

Die neueste Version finden Sie auf der [**Releases-Seite**](https://github.com/Aitor2010aitor/cupcraft/releases/latest).

| Plattform | Datei | Hinweise |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | Installer, keine Administratorrechte nötig. |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | Portable ausführbare Datei. |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> Die macOS- und Linux-Builds sind **experimentell**. Einige Werkzeuge (Whisper-Untertitel, Piper-Stimmen und CapCut-Import) hängen noch von Windows-Komponenten ab und sind dort nicht verfügbar.

> [!IMPORTANT]
> Die Binärdateien sind noch nicht signiert. Unter Windows kann SmartScreen eine Warnung anzeigen (**Weitere Informationen → Trotzdem ausführen**). Unter macOS entfernen Sie einmalig das Quarantäne-Flag:
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

Für den Export muss [FFmpeg](https://ffmpeg.org/download.html) im `PATH` (oder im cupcraft-Datenordner) verfügbar sein.

## 🔄 Updates

Wenn Sie ein neues Release veröffentlichen, erkennt cupcraft es beim Start und zeigt eine Karte mit **Jetzt aktualisieren / Später**. Der Download wird per SHA-256 geprüft, die ausführbare Datei ersetzt, Ihr Projekt gespeichert und die App neu gestartet.

> [!TIP]
> Es funktioniert ohne GitHub-API und ohne Token: Die App liest nur öffentliche Dateien des neuesten Releases. Details in [docs/RELEASING.md](../docs/RELEASING.md).

## 🛠️ Aus dem Quellcode bauen

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

Die Binärdatei entsteht unter `rust/target/release/cupcraft-app`. Unter Windows genügt auch ein Doppelklick auf `compilar.bat`.

> [!NOTE]
> Unter Linux werden die Entwicklungspakete `libasound2-dev`, `libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libssl-dev` und `pkg-config` benötigt.

## 🗂️ Wo liegen meine Daten?

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 Architektur

Ein Rust-Workspace, aufgeteilt in kleine Crates (`cupcraft-core`, `-persist`, `-render`, `-media`, `-audio`, `-capcut`, `-tts`, `-bridge` und das Binary `-app`). Siehe [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).

## 🤝 Mitwirkende

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | Entwickler und Maintainer |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | Co-Autor — KI-Pair-Programmer |

## 📄 Lizenz

[MIT-Lizenz](../../LICENSE) © 2026 AITOR2010
