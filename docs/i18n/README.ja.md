<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**高速・ローカル・ネイティブなデスクトップ動画エディタ。100% Rust 製。**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-2d2d2d?style=for-the-badge)](README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-2d2d2d?style=for-the-badge)](README.pt-BR.md)
[![Deutsch](https://img.shields.io/badge/Deutsch-2d2d2d?style=for-the-badge)](README.de.md)
[![日本語](https://img.shields.io/badge/%E6%97%A5%E6%9C%AC%E8%AA%9E-0a84ff?style=for-the-badge)](README.ja.md)
[![Русский](https://img.shields.io/badge/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9-2d2d2d?style=for-the-badge)](README.ru.md)

[![Latest release](https://img.shields.io/github/v/release/Aitor2010aitor/cupcraft?style=flat-square&color=00c1cd)](https://github.com/Aitor2010aitor/cupcraft/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](../../LICENSE)
![Rust](https://img.shields.io/badge/Rust-egui%20%2B%20wgpu-orange?style=flat-square)
![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey?style=flat-square)

</div>

<img width="1024" height="768" alt="cupcraft" src="https://github.com/user-attachments/assets/703ab96c-4646-4207-ae87-d8cc019c4994" />

<img width="1686" height="934" alt="cupcraft editor" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

cupcraft は、CapCut のワークフローとダークなインターフェースにインスパイアされたデスクトップ動画エディタです。単一のネイティブ実行ファイルで、Electron も WebView も Node.js もサーバーも不要。すべてお使いのパソコン上で動作します。

> [!NOTE]
> cupcraft は独自の実装です。CapCut のバイナリ、ロゴ、フォント、その他の独占的なリソースは含まれておらず、ByteDance との提携や承認もありません。

## ✨ 機能

- **マルチトラックタイムライン** — 動画、音声、テキスト、ステッカー、エフェクト、フィルター。分割、トリム、複製、スナップ、キーフレーム、80 ステップの元に戻す/やり直し。
- **GPU 合成** — エフェクト、LUT フィルター、調整、トランジションに `wgpu` シェーダーを使用（CPU フォールバック付き）。
- **テキストとステッカー** — テキストスタイル、システム/取り込みフォント、アニメーションステッカー（GIF / WebP）、クリップアニメーション。
- **オーディオ** — フェード付きのリアルタイムマルチトラックミキシング、波形表示、ローカル音声合成。
- **ローカル AI ツール** — 背景除去（ONNX）と自動字幕（Whisper）をお使いのパソコン上で実行。
- **書き出し** — FFmpeg による MP4（H.264 / H.265）と WebM（VP9）。利用可能な場合はハードウェアエンコーダー（NVENC、QSV、AMF）を使用。
- **CapCut 取り込み** — CapCut のドラフトを読み取り専用で読み込み、cupcraft プロジェクトに変換。
- **安全なプロジェクト** — ローカル SQLite データベースへの自動保存。アプリを閉じるときに保存するか確認します。
- **自動アップデート** — GitHub で公開された新バージョンをアプリ内で案内。
- **プライバシー重視** — アカウント不要、クラウドなし、テレメトリなし。ネットワーク通信は更新確認とご自身が開始したダウンロードのみです。

## 📦 ダウンロード

最新ビルドは [**Releases ページ**](https://github.com/Aitor2010aitor/cupcraft/releases/latest) から入手できます。

| プラットフォーム | ファイル | 備考 |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | インストーラー（管理者権限は不要）。 |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | ポータブル実行ファイル。 |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> macOS 版と Linux 版は**試験的**です。一部のツール（Whisper 字幕、Piper 音声、CapCut ドラフトの取り込み）はまだ Windows のコンポーネントに依存しており、これらの環境では利用できません。

> [!IMPORTANT]
> バイナリはまだコード署名されていません。Windows では SmartScreen の警告が表示されることがあります（**詳細情報 → 実行**）。macOS では、一度だけ隔離属性を削除してください：
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

書き出しには、`PATH`（または cupcraft のデータフォルダー）に [FFmpeg](https://ffmpeg.org/download.html) が必要です。

## 🔄 アップデート

新しいリリースを公開すると、cupcraft は起動時にそれを検出し、**今すぐ更新 / あとで** のカードを表示します。ダウンロードは SHA-256 で検証され、実行ファイルが置き換えられ、プロジェクトが保存されてアプリが再起動します。

> [!TIP]
> GitHub API もトークンも不要です。アプリは最新リリースの公開ファイルを読み取るだけです。詳細は [docs/RELEASING.md](../docs/RELEASING.md) をご覧ください。

## 🛠️ ソースからビルド

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

バイナリは `rust/target/release/cupcraft-app` に作成されます。Windows では `compilar.bat` をダブルクリックしてもビルドできます。

> [!NOTE]
> Linux では、開発用パッケージ `libasound2-dev`、`libgtk-3-dev`、`libxkbcommon-dev`、`libwayland-dev`、`libssl-dev`、`pkg-config` が必要です。

## 🗂️ データの保存場所

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 アーキテクチャ

小さな crate に分割された Rust ワークスペース（`cupcraft-core`、`-persist`、`-render`、`-media`、`-audio`、`-capcut`、`-tts`、`-bridge`、バイナリの `-app`）。詳しくは [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) を参照してください。

## 🤝 クレジット

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | 作者・メンテナー |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | 共同作者 — AI ペアプログラマー |

## 📄 ライセンス

[MIT ライセンス](../../LICENSE) © 2026 AITOR2010
