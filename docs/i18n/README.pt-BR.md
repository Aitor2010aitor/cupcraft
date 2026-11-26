<div align="center">

<img src="../../rust/crates/cupcraft-app/src/logo.png" width="96" alt="cupcraft logo">

# cupcraft

**Um editor de vídeo de desktop rápido, local e nativo. 100% Rust.**

[![English](https://img.shields.io/badge/English-2d2d2d?style=for-the-badge)](../../README.md)
[![Español](https://img.shields.io/badge/Espa%C3%B1ol-2d2d2d?style=for-the-badge)](README.es.md)
[![Français](https://img.shields.io/badge/Fran%C3%A7ais-2d2d2d?style=for-the-badge)](README.fr.md)
[![中文](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-2d2d2d?style=for-the-badge)](README.zh-CN.md)
[![Português](https://img.shields.io/badge/Portugu%C3%AAs-0a84ff?style=for-the-badge)](README.pt-BR.md)
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

O cupcraft é um editor de vídeo de desktop inspirado no fluxo de trabalho e na interface escura do CapCut. É um único executável nativo: sem Electron, sem WebView, sem Node.js e sem servidor. Tudo roda no seu computador.

> [!NOTE]
> O cupcraft é uma implementação original. Não inclui binários, logotipos, fontes nem recursos proprietários do CapCut e não é afiliado nem endossado pela ByteDance.

## ✨ Recursos

- **Linha do tempo multitrilha** — vídeo, áudio, texto, stickers, efeitos e filtros. Dividir, cortar, duplicar, ímã, quadros-chave e 80 passos de desfazer/refazer.
- **Composição na GPU** — shaders `wgpu` para efeitos, filtros LUT, ajustes e transições, com alternativa na CPU.
- **Texto e stickers** — estilos de texto, fontes do sistema e importadas, stickers animados (GIF / WebP) e animações de clipe.
- **Áudio** — mixagem multitrilha em tempo real com fades, formas de onda e síntese de voz local.
- **IA local** — remoção de fundo (ONNX) e legendas automáticas (Whisper), tudo no seu computador.
- **Exportação** — MP4 (H.264 / H.265) e WebM (VP9) via FFmpeg, com codificadores por hardware (NVENC, QSV, AMF) quando disponíveis.
- **Importação do CapCut** — lê rascunhos do CapCut (somente leitura) e os converte em projetos do cupcraft.
- **Projetos seguros** — salvamento automático em um banco SQLite local; ao fechar o app, ele pergunta se você quer salvar.
- **Atualização automática** — novas versões publicadas no GitHub são oferecidas dentro do app.
- **Privado** — sem conta, sem nuvem, sem telemetria. O único tráfego de rede é a verificação de atualizações e os downloads que você iniciar.

## 📦 Download

Baixe a versão mais recente na [**página de Releases**](https://github.com/Aitor2010aitor/cupcraft/releases/latest).

| Plataforma | Arquivo | Observações |
|---|---|---|
| Windows (x64) | `cupcraft-setup-windows-x86_64.exe` | Instalador, sem direitos de administrador. |
| Windows (x64) | `cupcraft-windows-x86_64.exe` | Executável portátil. |
| macOS (Apple Silicon) | `cupcraft-macos-aarch64.tar.gz` | |
| macOS (Intel) | `cupcraft-macos-x86_64.tar.gz` | |
| Linux (x64) | `cupcraft-linux-x86_64.tar.gz` | |

> [!WARNING]
> As versões para macOS e Linux são **experimentais**. Algumas ferramentas (legendas Whisper, vozes Piper e importação do CapCut) ainda dependem de componentes do Windows e não estão disponíveis lá.

> [!IMPORTANT]
> Os binários ainda não são assinados. No Windows, o SmartScreen pode exibir um aviso (**Mais informações → Executar assim mesmo**). No macOS, remova a marca de quarentena uma vez:
> ```sh
> tar -xzf cupcraft-macos-*.tar.gz && xattr -dr com.apple.quarantine cupcraft
> ```

### FFmpeg

Para exportar, o [FFmpeg](https://ffmpeg.org/download.html) precisa estar no seu `PATH` (ou na pasta de dados do cupcraft).

## 🔄 Atualizações

Quando você publica uma nova release, o cupcraft a detecta ao iniciar e mostra um cartão com **Atualizar agora / Mais tarde**. O download é verificado com SHA-256, o executável é substituído, seu projeto é salvo e o app reinicia.

> [!TIP]
> Funciona sem a API do GitHub e sem tokens: o app lê apenas arquivos públicos da última release. Detalhes em [docs/RELEASING.md](../docs/RELEASING.md).

## 🛠️ Compilar a partir do código

```sh
git clone https://github.com/Aitor2010aitor/cupcraft.git
cd cupcraft/rust
cargo build --release -p cupcraft-app
```

O binário é criado em `rust/target/release/cupcraft-app`. No Windows, você também pode dar um clique duplo em `compilar.bat`.

> [!NOTE]
> No Linux, instale os pacotes de desenvolvimento `libasound2-dev`, `libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libssl-dev` e `pkg-config`.

## 🗂️ Onde ficam meus dados?

| OS | |
|---|---|
| Windows | `%LOCALAPPDATA%\openCUP` |
| macOS | `~/Library/Application Support/openCUP` |
| Linux | `~/.local/share/openCUP` |

## 🧱 Arquitetura

Um workspace Rust dividido em crates pequenos (`cupcraft-core`, `-persist`, `-render`, `-media`, `-audio`, `-capcut`, `-tts`, `-bridge` e o binário `-app`). Veja [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).

## 🤝 Créditos

| | |
|---|---|
| [**AITOR2010**](https://github.com/Aitor2010aitor) | Criador e mantenedor |
| [**Claude**](https://www.anthropic.com/claude) (Anthropic) | Coautor — programador em dupla com IA |

## 📄 Licença

[Licença MIT](../../LICENSE) © 2026 AITOR2010
