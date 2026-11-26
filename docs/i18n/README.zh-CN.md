<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**快速、本地、原生的桌面视频编辑器，100% Rust 编写。**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-0a84ff?style=for-the-badge)](README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-2d2d2d?style=for-the-badge)](README.pt-BR.md)
[![Deutsch](https://img.shields.io/badge/Deutsch-2d2d2d?style=for-the-badge)](README.de.md)
[![日本語](https://img.shields.io/badge/%E6%97%A5%E6%9C%AC%E8%AA%9E-2d2d2d?style=for-the-badge)](README.ja.md)
[![Русский](https://img.shields.io/badge/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9-2d2d2d?style=for-the-badge)](README.ru.md)

[![Latest release](https://img.shields.io/github/v/release/Aitor2010aitor/cupcraft?style=flat-square&color=00c1cd)](https://github.com/Aitor2010aitor/cupcraft/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](../../LICENSE)
![Rust](https://img.shields.io/badge/Rust-egui%20%2B%20wgpu-orange?style=flat-square)
![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey?style=flat-square)

</div>

cupcraft 是一款桌面视频编辑器，灵感来自 CapCut 的工作流程和深色界面。它只是一个原生可执行文件：没有 Electron，没有 WebView，没有 Node.js，也没有服务器。所有操作都在你的电脑上运行。

> [!NOTE]
> cupcraft 是原创实现，不包含 CapCut 的任何二进制文件、标志、字体或专有资源，也与字节跳动无关联，未获其认可。

## ✨ 功能

- **多轨时间线** — 视频、音频、文字、贴纸、特效和滤镜。分割、裁剪、复制、吸附、关键帧，以及 80 步撤销/重做。
- **GPU 合成** — 使用 `wgpu` 着色器处理特效、LUT 滤镜、调节和转场，并提供 CPU 回退。
- **文字与贴纸** — 文字样式、系统字体和导入字体、动画贴纸（GIF / WebP）以及片段动画。
- **音频** — 带淡入淡出的实时多轨混音、波形显示和本地文字转语音。
- **本地 AI 工具** — 背景移除（ONNX）和自动字幕（Whisper），全部在你的电脑上运行。
- **导出** — 通过 FFmpeg 导出 MP4（H.264 / H.265）和 WebM（VP9），有硬件编码器（NVENC、QSV、AMF）时自动使用。
- **导入 CapCut** — 以只读方式读取 CapCut 草稿并转换为 cupcraft 项目。
- **项目安全** — 自动保存到本地 SQLite 数据库；关闭应用时会询问是否保存。
- **自动更新** — GitHub 上发布的新版本会在应用内提示更新。
- **隐私** — 无需账号、无云端、无遥测。唯一的网络流量是检查更新以及你主动发起的下载。

## 📦 下载

请从 [**Releases 页面**](https://github.com/Aitor2010aitor/cupcraft/releases/latest) 下载最新版本。

| 平台 | 文件 | 说明 |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | 安装程序，无需管理员权限。 |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | 便携可执行文件。 |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> macOS 和 Linux 版本目前处于**实验阶段**。部分工具（Whisper 字幕、Piper 语音和 CapCut 草稿导入）仍依赖 Windows 组件，在这些系统上暂不可用。

> [!IMPORTANT]
> 可执行文件尚未进行代码签名。在 Windows 上，SmartScreen 可能会显示警告（**更多信息 → 仍要运行**）。在 macOS 上，请先移除一次隔离标记：
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

导出需要 [FFmpeg](https://ffmpeg.org/download.html) 位于 `PATH` 中（或位于 cupcraft 数据文件夹中）。

## 🔄 更新

当你发布新版本后，cupcraft 会在启动时检测到，并弹出带有 **立即更新 / 稍后** 的卡片。下载内容会通过 SHA-256 校验，替换可执行文件，保存当前项目并重启应用。

> [!TIP]
> 无需 GitHub API 和令牌：应用只读取最新 release 中的公开文件。详见 [docs/RELEASING.md](../docs/RELEASING.md)。

## 🛠️ 从源码构建

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

生成的可执行文件位于 `rust/target/release/cupcraft-app`。在 Windows 上也可以双击 `compilar.bat`。

> [!NOTE]
> Linux 需要安装开发包 `libasound2-dev`、`libgtk-3-dev`、`libxkbcommon-dev`、`libwayland-dev`、`libssl-dev` 和 `pkg-config`。

## 🗂️ 我的数据在哪里？

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 架构

Rust 工作区，拆分为多个小型 crate（`cupcraft-core`、`-persist`、`-render`、`-media`、`-audio`、`-capcut`、`-tts`、`-bridge` 以及 `-app` 可执行程序）。参见 [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)。

## 🤝 致谢

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | 创建者与维护者 |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | 共同作者 — AI 结对编程 |

## 📄 许可证

[MIT 许可证](../../LICENSE) © 2026 AITOR2010
