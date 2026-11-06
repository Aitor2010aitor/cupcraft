# cupcraft

![READ IN ENGLISH](https://github.com/Aitor2010aitor/cupcraft/blob/version.04/READMEN.md)




<img width="1686" height="934" alt="{9E045220-ABD4-45D7-AB6E-E135D48AB93C}" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

A local video editor that runs in the browser, inspired by CapCut's workflow and dark interface. cupcraft is an original implementation: it does not include CapCut binaries, logos, typefaces, or proprietary assets.

![cupcraft home screen](shots/01-home.png)

![cupcraft editor with the text panel](shots/20-template.png)

## Features

### Home and projects

- Home screen with recent projects, templates, and quick shortcuts.
- Create projects with an aspect ratio (16:9, 9:16, 1:1, 4:3, 21:9, 4:5), resolution (480p–4K), and FPS (24/30/60).
- Rename and delete projects; changes are saved automatically to IndexedDB.
- Import media from the home screen or from the editor.

### CapCut project import

- Detects native drafts from CapCut installed on Windows (`%LOCALAPPDATA%\CapCut\User Data\Projects\com.lveditor.draft`).
- Displays the name, date, aspect ratio, duration, thumbnail, and number of clips before importing.
- Creates a new cupcraft project without modifying the original draft: it copies media to IndexedDB, remaps tracks, and preserves text clips and their timings.
- Import works offline and does not use CapCut cloud services.

![List of native CapCut drafts in cupcraft](shots/new-capcut-list.png)

### Editing

- Canvas preview with real-time playback.
- Direct manipulation in the preview: drag text and stickers, resize them using the four corners, rotate them using the top handle, and use the center guides. Each gesture is saved as a single history step.
- Timeline with text, video, and audio tracks.
- Import videos, images, and audio; thumbnails and waveforms are generated automatically.
- Move, trim from either end, split at the playhead, duplicate, and delete clips.
- Undo and redo (up to 80 steps).
- Text with customizable styles, size, weight, color, alignment, shadow, and background.
- 731 filters organized into the 19 categories from CapCut 9.6 (Featured, Halloween, NEW, Bangers, Life, Scenery, Portrait, Black & White, Movies, Retro, Tools, Alternative, Night, Cool Filter, Warm Filter, Food, Photo Booth, CCD, and Pets), with live previews and category filters.
- 168 visual effects in 19 categories (Trending, Duck, Halloween, Specials, Black Friday, Classic, Magic Cutout, NEW, Bangers, Intro & Outro, Random Frames, Party, Motion, Motion 2, Edits, 3D, Light, Retro, and Body Effects). An original procedural engine renders particles, frames, lighting, VHS, glitch, and motion effects in real time.
- 16 entrance transitions that blend with the previous clip on the track.
- 219 emoji stickers across 19 categories inspired by CapCut's panel (Trending, AI-generated, Halloween, Black Friday, Icons, Emoji, Style, Luggage, Love, LETTERS, Shapes, etc.), editable as layers. The artwork is original (emoji); no CapCut assets are redistributed.
- 8 local avatars with colored backgrounds, plus the option to upload a photo.
- Subtitle styles (clean, box, yellow, karaoke, minimal, neon) and support for importing timed `.srt` files.
- 10 templates with a title and subtitle, available from the home screen and the editor panel.
- Per-clip volume, playback speed, and track muting.
- Voice recording using the microphone and the `MediaRecorder` API.
- Timeline zoom and automatic fitting when clips are dropped.

### AI control bridge

cupcraft includes a local HTTP bridge that allows another AI, agent, or script to read the editor's state and perform actions on the session open in the browser. The bridge listens only on `127.0.0.1:8790` and requires a token.

- The token is stored in `.openCUP-ai-token` or set using `openCUP_AI_TOKEN`. It is also displayed under **Menu → AI Bridge**.
- The port can be changed using `openCUP_AI_PORT`. If it is already in use, the server tries subsequent ports and displays the selected port at startup.
- The application must be open in the browser. Commands are queued, and the browser retrieves them during its next poll.
- If multiple cupcraft tabs are open, the bridge controls the most recently loaded tab, and `/status` reports its `target`.

| Endpoint | Purpose |
| --- | --- |
| `GET /api/ai/status` | Bridge status, connected clients, and pending commands |
| `GET /api/ai/state` | Latest state received from the editor: project, tracks, clips, media, selection, and panel |
| `GET /api/ai/actions` | Catalog of actions and parameters |
| `POST /api/ai/command` | Queues an action; with `wait: true`, waits for the result |
| `GET /api/ai/result?id=…` | Retrieves the result of an asynchronous command |
| `GET /api/ai/commands` | Recent command history |
| `POST /api/ai/command/cancel` | Cancels a pending command |

The token is sent using `Authorization: Bearer <token>`, `X-openCUP-Token`, or `?token=`:

```bash
curl -H "Authorization: Bearer $openCUP_AI_TOKEN" http://127.0.0.1:8790/api/ai/state
curl -X POST -H "Authorization: Bearer $openCUP_AI_TOKEN" -H "Content-Type: application/json" \
  -d '{"action":"addText","params":{"content":"Hello from an AI","start":0,"duration":3},"wait":true}' \
  http://127.0.0.1:8790/api/ai/command
```

Available actions: `getState`, `listProjects`, `newProject`, `openProject`, `goHome`, `save`, `listCapCutProjects`, `openCapCutProject`, `setPanel`, `seek`, `play`, `pause`, `togglePlay`, `selectClip`, `addText`, `addSticker`, `applyFilter`, `applyEffect`, `applyTransition`, `updateClip`, `split`, `deleteClip`, `duplicateClip`, `undo`, `redo`, `setZoom`, `listFilters`, `listStickers`, `listEffects`, `listTransitions`, and `screenshot`. The `screenshot` action returns a JPEG data URL of the current frame.

![AI control bridge dialog](shots/new-bridge.png)

### Export

- Capture the current frame as PNG.
- Export video in WebM format (VP9 + Opus) at 480p, 720p, 1080p, or 4K, including audio from videos and audio tracks.
- Exporting is performed in real time, so it takes approximately as long as the project itself.
- Duration metadata is added to the WebM generated by `MediaRecorder` so that media players display the correct duration.

## How to run

Requirements:

- Node.js 18 or later (tested with Node.js 24.22.1).
- An up-to-date Chromium browser (Chrome, Edge, or Brave) with support for `MediaRecorder`, `canvas.captureStream`, and Web Audio.

```bash
npm start
```

Open <http://127.0.0.1:8787>. The server listens only on `127.0.0.1`; no dependencies are required to run the application.

You can also change the port:

```powershell
$env:PORT=9000; npm start
```

## Desktop application

The `dist/cupcraft` folder contains a native application for Windows x64:

- `cupcraft.exe` is a **native C++ executable** (Win32 + WebView2 COM): it does not use .NET, requires no managed DLLs, and is approximately 330 KB in size.
- It opens the interface in its own dark-themed window using WebView2 (embedded Edge).
- It includes `node.exe`, so Node.js does not need to be installed.
- The local server starts hidden when the app opens and stops when the window closes. It runs inside a Windows Job Object: if the app is forcibly terminated, the child server process is closed as well (no orphaned processes remain).
- Only one instance can be open at a time.
- Projects are stored in `%LOCALAPPDATA%\openCUP\WebView2`; the startup log is located at `%LOCALAPPDATA%\openCUP\openCUP.log`.
- The folder can be copied to another Windows 10/11 computer with the WebView2 Runtime installed.

To rebuild the executable, icon, and desktop shortcut:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-desktop.ps1
```

To also build an installer (Inno Setup 6, downloaded automatically if necessary):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-installer.ps1
```

The output is `dist/cupcraft-Setup-0.3.0.exe` (~24 MB): an installer wizard in Spanish, an icon on the desktop and in the Start menu, and an uninstaller integrated into "Apps & features". Administrator privileges are required to install to `Program Files`.

The build script locates the MSVC C++ tools (Visual Studio / Build Tools), downloads the WebView2 SDK using `tools/fetch-webview2.ps1` if it is missing, compiles `launcher/main.cpp` and embeds the icon (`rc.exe`), then copies the application, server, and Node.js into `dist/cupcraft`. The Visual Studio **Desktop development with C++** workload is required. The installer requires Inno Setup 6 (`tools/build-installer.ps1` downloads and installs it automatically if it is not already present).

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Space` | Play / pause |
| `B` | Split the selected clip at the playhead |
| `Delete` / `Backspace` | Delete the selected clip |
| `Ctrl/Cmd + Z` | Undo |
| `Ctrl/Cmd + Shift + Z` or `Ctrl/Cmd + Y` | Redo |
| `Ctrl/Cmd + D` | Duplicate the selected clip |
| `Ctrl/Cmd + I` | Import media |
| `Ctrl/Cmd + S` | Save now |
| `←` / `→` | Previous / next frame |
| `Home` / `End` | Go to the beginning / end |
| `Esc` | Close the dialog or deselect |

Shortcuts are disabled while typing in a text field or while a dialog is open.

## Local storage

- Projects and files are stored in IndexedDB (`openCUP-db`): metadata in `projects` and blobs in `media`.
- Nothing is uploaded to a server. All processing takes place in the browser.
- The Node server only serves static files and the local CapCut and AI bridge routes; it does not copy media off the computer.
- Deleting the site's data also deletes its projects and media.
- Available storage space depends on the browser's storage quota.
- The CapCut importer reads original drafts in read-only mode; CapCut files are not modified.

### Details panel

The right-hand panel lets you edit the project or selected clip without leaving the editor: name, aspect ratio, resolution, FPS, text, filters, transform, volume, fades, and speed. On narrow screens, it is hidden to prioritize the preview.

## Known limitations

- CapCut cloud AI features (automatic transcription, animated avatars, background removal, and video generation) are not included. The interface marks them as unavailable offline.
- The CapCut importer is designed for native drafts from the Windows desktop version. It reconstructs media, tracks, text, and timings; CapCut's proprietary effects, transitions, and animations are not recreated.
- The AI bridge only works while cupcraft is open in the browser and the local server is running. It is not designed to be exposed to the Internet.
- Export uses `MediaRecorder`; the output container is WebM. MP4 would require transcoding outside the browser.
- Export runs in real time and does not use GPU acceleration beyond what the browser provides.
- Supported audio and video formats depend on the codecs supported by the browser.
- There is no collaboration or account system: the project exists in the browser where it was created.

## Project structure

```text
cupcraft/
├─ index.html
├─ server.mjs              Static server, CapCut API, and internal AI API
├─ server/
│  ├─ capcut.mjs           Scans and imports native CapCut drafts
│  └─ ai-bridge.mjs        Port 8790, authentication, commands, and documentation
├─ src/
│  ├─ main.js              Startup, global shortcuts, and drag & drop
│  ├─ home.js              Home screen, projects, and templates
│  ├─ editor.js            Top bar, tabs, and editor lifecycle
│  ├─ store.js             State, actions, history, and persistence
│  ├─ db.js                IndexedDB
│  ├─ renderer.js          Canvas rendering, effects, transitions, and audio
│  ├─ preview.js           Preview and playback controls
│  ├─ timeline.js          Rules, tracks, and clip manipulation
│  ├─ panels.js            Media, text, stickers, effects, filters, and details
│  ├─ filter-catalog.js    Catalog of 731 filters in 19 CapCut-style categories
│  ├─ library.js           Effect, sticker, transition, and avatar catalogs
│  ├─ capcut.js            CapCut import client and project dialog
│  ├─ ai-bridge.js         Browser bridge, actions, and control dialog
│  ├─ export.js            WebM export and PNG capture
│  ├─ webm.js              EBML duration patch for WebM
│  ├─ templates.js         Template data
│  ├─ dialogs.js           Shortcuts and About dialog
│  ├─ modal.js             Modal dialogs
│  ├─ icons.js             SVG icons
│  ├─ utils.js             Utilities (time, bytes, toasts)
│  └─ styles.css           Dark theme and responsive layout
├─ launcher/main.cpp        Native Win32 + WebView2 launcher written in pure C++
├─ tools/                  Browser tests and packaging scripts
│  ├─ build-desktop.ps1    Compiles the C++ executable and assembles dist/cupcraft
│  ├─ build-installer.ps1  Generates dist/cupcraft-Setup-0.3.0.exe with Inno Setup 6
│  ├─ cacup.iss            Inno Setup script (Spanish, uninstaller)
│  ├─ fetch-webview2.ps1   Downloads the native WebView2 SDK from nuget.org
│  ├─ run-all-tests.ps1    Starts the server and runs all 11 test suites
│  ├─ make-icon.ps1        Generates the multi-resolution icon
│  └─ fixtures/            Test media generated with ffmpeg
└─ shots/                  Screenshots generated by the tests
```

## Tests

The tests use `puppeteer-core` with an installed copy of Chrome. They are not part of the application and are not required to use it.

```bash
npm install                 # installs puppeteer-core (devDependency)
npm start                   # in another terminal, keeps the server running on port 8787
npm test                    # complete editing and persistence workflow
npm run test:layout         # responsive layout audit from 1920 to 760 px
npm run test:features       # rename, delete, templates, and SRT
npm run test:integrations   # galleries, CapCut import, and AI bridge
npm run test:voice          # recording with Chrome's fake microphone
npm run test:export         # WebM without audio + ffprobe
npm run test:audio          # WebM with Opus audio + volume
npm run test:visual         # home screen and editor screenshots
npm run test:preview        # dragging, scaling, rotation, stickers, and undo
npm run test:effects        # screenshots of eight procedural effects
```

Notes:

- The scripts expect Chrome at `C:/Program Files/Google/Chrome/Application/chrome.exe` and the server at `http://127.0.0.1:8787`.
- `test:integrations` checks the bridge on port `8790` and opens CapCut draft `1007`; if that path does not exist, the CapCut checks will fail. Adjust the ID or name in `tools/new-features-test.mjs` for your machine.
- Test media is included in `tools/fixtures/`; these are synthetic videos, images, and audio generated with ffmpeg.
- `test:export` and `test:audio` require `ffmpeg`/`ffprobe` in the `PATH`.
- Screenshots are written to `shots/`; exported test files are written to `shots/*-test/`.
- `tools/visual-test.mjs` generates screenshots of the home screen and editor.

## Notice

cupcraft is an independent educational project. CapCut is a trademark of ByteDance. This repository is not affiliated with CapCut and does not distribute its assets.
