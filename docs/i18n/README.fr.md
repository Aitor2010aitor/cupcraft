<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**Un éditeur vidéo de bureau rapide, local et natif. 100 % Rust.**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-0a84ff?style=for-the-badge)](README.fr.md)
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

cupcraft est un éditeur vidéo de bureau inspiré du flux de travail et de l'interface sombre de CapCut. C'est un unique exécutable natif : pas d'Electron, pas de WebView, pas de Node.js, pas de serveur. Tout s'exécute sur votre machine.

> [!NOTE]
> cupcraft est une implémentation originale. Il n'inclut ni binaires, ni logos, ni polices, ni ressources propriétaires de CapCut, et n'est ni affilié à ByteDance ni approuvé par celle-ci.

## ✨ Fonctionnalités

- **Timeline multipiste** — vidéo, audio, texte, stickers, effets et filtres. Découpe, ajustement, duplication, magnétisme, images clés et 80 niveaux d'annulation/rétablissement.
- **Composition GPU** — shaders `wgpu` pour les effets, filtres LUT, réglages et transitions, avec repli sur le CPU.
- **Texte et stickers** — styles de texte, polices système et importées, stickers animés (GIF / WebP) et animations de clips.
- **Audio** — mixage multipiste en temps réel avec fondus, formes d'onde et synthèse vocale locale.
- **IA locale** — suppression d'arrière-plan (ONNX) et sous-titres automatiques (Whisper), directement sur votre ordinateur.
- **Export** — MP4 (H.264 / H.265) et WebM (VP9) via FFmpeg, avec encodeurs matériels (NVENC, QSV, AMF) si disponibles.
- **Import CapCut** — lit les brouillons CapCut (lecture seule) et les convertit en projets cupcraft.
- **Projets en sécurité** — sauvegarde automatique dans une base SQLite locale ; à la fermeture, l'application demande si vous voulez enregistrer.
- **Mise à jour automatique** — les nouvelles versions publiées sur GitHub sont proposées dans l'application.
- **Privé** — pas de compte, pas de cloud, pas de télémétrie. Le seul trafic réseau est la vérification des mises à jour et les téléchargements que vous lancez vous-même.

## 📦 Téléchargement

Téléchargez la dernière version sur la [**page des Releases**](https://github.com/Aitor2010aitor/cupcraft/releases/latest).

| Plateforme | Fichier | Remarques |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | Installateur, sans droits administrateur. |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | Exécutable portable. |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> Les versions macOS et Linux sont **expérimentales**. Certains outils (sous-titres Whisper, voix Piper et import CapCut) dépendent encore de composants Windows et n'y sont pas disponibles.

> [!IMPORTANT]
> Les binaires ne sont pas encore signés. Sous Windows, SmartScreen peut afficher un avertissement (**Informations complémentaires → Exécuter quand même**). Sous macOS, retirez une fois l'indicateur de quarantaine :
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

Pour exporter, [FFmpeg](https://ffmpeg.org/download.html) doit être dans votre `PATH` (ou dans le dossier de données de cupcraft).

## 🔄 Mises à jour

Quand vous publiez une nouvelle release, cupcraft la détecte au démarrage et affiche une carte **Mettre à jour / Plus tard**. Le téléchargement est vérifié par SHA-256, l'exécutable est remplacé, votre projet est enregistré et l'application redémarre.

> [!TIP]
> Cela fonctionne sans l'API GitHub et sans jeton : l'application lit seulement des fichiers publics de la dernière release. Détails dans [docs/RELEASING.md](../docs/RELEASING.md).

## 🛠️ Compiler depuis les sources

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

Le binaire est créé dans `rust/target/release/cupcraft-app`. Sous Windows, vous pouvez aussi double-cliquer sur `compilar.bat`.

> [!NOTE]
> Sous Linux, installez les paquets de développement `libasound2-dev`, `libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libssl-dev` et `pkg-config`.

## 🗂️ Où sont mes données ?

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 Architecture

Un workspace Rust découpé en petits crates (`cupcraft-core`, `-persist`, `-render`, `-media`, `-audio`, `-capcut`, `-tts`, `-bridge` et le binaire `-app`). Voir [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).

## 🤝 Crédits

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | Créateur et mainteneur |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | Co-auteur — programmeur IA en binôme |

## 📄 Licence

[Licence MIT](../../LICENSE) © 2026 AITOR2010
