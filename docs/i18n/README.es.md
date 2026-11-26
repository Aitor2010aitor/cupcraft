<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**Un editor de vídeo de escritorio rápido, local y nativo. 100 % Rust.**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-0a84ff?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-2d2d2d?style=for-the-badge)](README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-2d2d2d?style=for-the-badge)](README.pt-BR.md)
[![Deutsch](https://img.shields.io/badge/Deutsch-2d2d2d?style=for-the-badge)](README.de.md)
[![日本語](https://img.shields.io/badge/%E6%97%A5%E6%9C%AC%E8%AA%9E-2d2d2d?style=for-the-badge)](README.ja.md)
[![Русский](https://img.shields.io/badge/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9-2d2d2d?style=for-the-badge)](README.ru.md)

[![Latest release](https://img.shields.io/github/v/release/Aitor2010aitor/cupcraft?style=flat-square&color=00c1cd)](https://github.com/Aitor2010aitor/cupcraft/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](../../LICENSE)
![Rust](https://img.shields.io/badge/Rust-egui%20%2B%20wgpu-orange?style=flat-square)
![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey?style=flat-square)

</div>

<img width="1024" height="768" alt="cupcraft" src="https://github.com/user-attachments/assets/703ab96c-4646-4207-ae87-d8cc019c4994" />

<img width="1686" height="934" alt="cupcraft editor" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

cupcraft es un editor de vídeo de escritorio inspirado en el flujo de trabajo y la interfaz oscura de CapCut. Es un único ejecutable nativo: sin Electron, sin WebView, sin Node.js y sin servidor. Todo se ejecuta en tu equipo.

> [!NOTE]
> cupcraft es una implementación original. No incluye binarios, logotipos, tipografías ni recursos propietarios de CapCut, y no está afiliado ni respaldado por ByteDance.

## ✨ Funciones

- **Línea de tiempo multipista** — vídeo, audio, texto, stickers, efectos y filtros. Dividir, recortar, duplicar, imán, fotogramas clave y 80 pasos de deshacer/rehacer.
- **Composición en GPU** — shaders `wgpu` para efectos, filtros LUT, ajustes y transiciones, con reserva en CPU.
- **Texto y stickers** — estilos de texto, fuentes del sistema e importadas, stickers animados (GIF / WebP) y animaciones de clip.
- **Audio** — mezcla multipista en tiempo real con fundidos, formas de onda y voz sintética local.
- **IA local** — eliminación de fondo (ONNX) y subtítulos automáticos (Whisper), todo en tu ordenador.
- **Exportación** — MP4 (H.264 / H.265) y WebM (VP9) con FFmpeg, usando codificadores por hardware (NVENC, QSV, AMF) si hay.
- **Importar CapCut** — lee borradores de CapCut (solo lectura) y los convierte en proyectos de cupcraft.
- **Proyectos seguros** — guardado automático en una base SQLite local; al cerrar la app te pregunta si quieres guardar.
- **Se actualiza solo** — las nuevas versiones publicadas en GitHub se ofrecen dentro de la app.
- **Privado** — sin cuenta, sin nube, sin telemetría. El único tráfico de red es la comprobación de actualizaciones y las descargas que tú inicies.

## 📦 Descarga

Descarga la última versión desde la [**página de Releases**](https://github.com/Aitor2010aitor/cupcraft/releases/latest).

| Plataforma | Archivo | Notas |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | Instalador, sin permisos de administrador. |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | Ejecutable portátil. |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> Las versiones de macOS y Linux son **experimentales**. Algunas herramientas (subtítulos Whisper, voces Piper e importación de CapCut) todavía dependen de componentes de Windows y no están disponibles allí.

> [!IMPORTANT]
> Los binarios aún no están firmados. En Windows, SmartScreen puede mostrar un aviso (**Más información → Ejecutar de todos modos**). En macOS, quita la marca de cuarentena una vez:
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

Para exportar necesitas [FFmpeg](https://ffmpeg.org/download.html) en tu `PATH` (o en la carpeta de datos de cupcraft).

## 🔄 Actualizaciones

Cuando publicas una nueva release, cupcraft la detecta al arrancar y muestra una tarjeta con **Actualizar ahora / Más tarde**. La descarga se verifica con SHA-256, se reemplaza el ejecutable, se guarda tu proyecto y la app se reinicia.

> [!TIP]
> Funciona sin la API de GitHub y sin tokens: la app solo lee archivos públicos de la última release. Detalles en [docs/RELEASING.md](../docs/RELEASING.md).

## 🛠️ Compilar desde el código

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

El binario se crea en `rust/target/release/cupcraft-app`. En Windows también puedes hacer doble clic en `compilar.bat`.

> [!NOTE]
> En Linux necesitas los paquetes de desarrollo `libasound2-dev`, `libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libssl-dev` y `pkg-config`.

## 🗂️ ¿Dónde están mis datos?

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 Arquitectura

Un workspace de Rust dividido en crates pequeños (`cupcraft-core`, `-persist`, `-render`, `-media`, `-audio`, `-capcut`, `-tts`, `-bridge` y el binario `-app`). Consulta [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).

## 🤝 Créditos

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | Creador y mantenedor |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | Coautor — programador en pareja con IA |

## 📄 Licencia

[Licencia MIT](../../LICENSE) © 2026 AITOR2010
