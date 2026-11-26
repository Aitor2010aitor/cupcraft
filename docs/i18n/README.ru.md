<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**Быстрый, локальный, нативный видеоредактор для компьютера. 100 % на Rust.**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-2d2d2d?style=for-the-badge)](README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-2d2d2d?style=for-the-badge)](README.pt-BR.md)
[![Deutsch](https://img.shields.io/badge/Deutsch-2d2d2d?style=for-the-badge)](README.de.md)
[![日本語](https://img.shields.io/badge/%E6%97%A5%E6%9C%AC%E8%AA%9E-2d2d2d?style=for-the-badge)](README.ja.md)
[![Русский](https://img.shields.io/badge/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9-0a84ff?style=for-the-badge)](README.ru.md)

[![Latest release](https://img.shields.io/github/v/release/Aitor2010aitor/cupcraft?style=flat-square&color=00c1cd)](https://github.com/Aitor2010aitor/cupcraft/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](../../LICENSE)
![Rust](https://img.shields.io/badge/Rust-egui%20%2B%20wgpu-orange?style=flat-square)
![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey?style=flat-square)

</div>

<img width="1024" height="768" alt="cupcraft" src="https://github.com/user-attachments/assets/703ab96c-4646-4207-ae87-d8cc019c4994" />

<img width="1686" height="934" alt="cupcraft editor" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

cupcraft — настольный видеоредактор, вдохновлённый рабочим процессом и тёмным интерфейсом CapCut. Это один нативный исполняемый файл: без Electron, без WebView, без Node.js и без сервера. Всё работает на вашем компьютере.

> [!NOTE]
> cupcraft — оригинальная реализация. В нём нет бинарных файлов, логотипов, шрифтов и других проприетарных ресурсов CapCut, и он не связан с ByteDance и не одобрен ею.

## ✨ Возможности

- **Многодорожечный таймлайн** — видео, аудио, текст, стикеры, эффекты и фильтры. Разделение, обрезка, дублирование, привязка, ключевые кадры и 80 шагов отмены/повтора.
- **Композитинг на GPU** — шейдеры `wgpu` для эффектов, LUT-фильтров, коррекции и переходов с запасным вариантом на CPU.
- **Текст и стикеры** — стили текста, системные и импортированные шрифты, анимированные стикеры (GIF / WebP) и анимации клипов.
- **Аудио** — многодорожечное микширование в реальном времени с плавными переходами, формами волны и локальным синтезом речи.
- **Локальные ИИ-инструменты** — удаление фона (ONNX) и автоматические субтитры (Whisper) прямо на вашем компьютере.
- **Экспорт** — MP4 (H.264 / H.265) и WebM (VP9) через FFmpeg, с аппаратными кодировщиками (NVENC, QSV, AMF) при их наличии.
- **Импорт из CapCut** — читает черновики CapCut (только чтение) и преобразует их в проекты cupcraft.
- **Надёжные проекты** — автосохранение в локальную базу SQLite; при закрытии приложение спрашивает, сохранить ли проект.
- **Автообновление** — новые версии, опубликованные на GitHub, предлагаются прямо в приложении.
- **Приватность** — без аккаунта, без облака, без телеметрии. Единственный сетевой трафик — проверка обновлений и загрузки, которые вы запускаете сами.

## 📦 Скачать

Последнюю сборку можно скачать на [**странице Releases**](https://github.com/Aitor2010aitor/cupcraft/releases/latest).

| Платформа | Файл | Примечания |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | Установщик, права администратора не нужны. |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | Портативный исполняемый файл. |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> Сборки для macOS и Linux — **экспериментальные**. Некоторые инструменты (субтитры Whisper, голоса Piper и импорт черновиков CapCut) пока зависят от компонентов Windows и там недоступны.

> [!IMPORTANT]
> Бинарные файлы пока не подписаны. В Windows SmartScreen может показать предупреждение (**Подробнее → Выполнить в любом случае**). В macOS один раз снимите флаг карантина:
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

Для экспорта нужен [FFmpeg](https://ffmpeg.org/download.html) в `PATH` (или в папке данных cupcraft).

## 🔄 Обновления

Когда вы публикуете новый релиз, cupcraft обнаруживает его при запуске и показывает карточку **Обновить сейчас / Позже**. Загрузка проверяется по SHA-256, исполняемый файл заменяется, проект сохраняется, и приложение перезапускается.

> [!TIP]
> Работает без GitHub API и без токенов: приложение читает только публичные файлы последнего релиза. Подробнее в [docs/RELEASING.md](../docs/RELEASING.md).

## 🛠️ Сборка из исходников

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

Исполняемый файл создаётся в `rust/target/release/cupcraft-app`. В Windows можно также дважды щёлкнуть `compilar.bat`.

> [!NOTE]
> В Linux нужны пакеты разработки `libasound2-dev`, `libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libssl-dev` и `pkg-config`.

## 🗂️ Где хранятся мои данные?

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 Архитектура

Рабочее пространство Rust, разбитое на небольшие крейты (`cupcraft-core`, `-persist`, `-render`, `-media`, `-audio`, `-capcut`, `-tts`, `-bridge` и бинарный `-app`). См. [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).

## 🤝 Авторы

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | Создатель и мейнтейнер |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | Соавтор — ИИ-напарник по программированию |

## 📄 Лицензия

[Лицензия MIT](../../LICENSE) © 2026 AITOR2010
