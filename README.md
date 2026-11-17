# cupcraft
![README IN ENGLISH](https://github.com/Aitor2010aitor/cupcraft/blob/0.4.5/READMEN.md)



<img width="1024" height="768" alt="hola" src="https://github.com/user-attachments/assets/703ab96c-4646-4207-ae87-d8cc019c4994" />




<img width="1686" height="934" alt="{9E045220-ABD4-45D7-AB6E-E135D48AB93C}" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

Editor de video local en el navegador, inspirado en el flujo de trabajo y la interfaz oscura de CapCut. cupcraft es una implementación original: no incluye binarios, logotipos, tipografías ni recursos propietarios de CapCut.

La aplicación es **Rust de principio a fin**: un único ejecutable abre la interfaz web en una ventana WebView2, sin Electron, sin Node.js y sin .NET.

![Pantalla de inicio de cupcraft](shots/01-home.png)

![Editor de cupcraft con panel de texto](shots/20-template.png)

## Novedades de la 0.4.5

- **Escritorio nativo en Rust**: `cupcraft-desktop` (tao + wry) arranca el servidor local y abre la UI en WebView2. Un solo `.exe` de unos 4 MB.
- **Stickers animados arreglados**: los stickers WebP y GIF de CapCut ya se decodifican y se dibujan fotograma a fotograma siguiendo el tiempo del clip.
- **Importación de CapCut más robusta**: nuevo `material_path()` con varios fallbacks, así se encuentran también los stickers que vienen sin extensión en el borrador.
- **Línea de tiempo estilo CapCut**: zoom con la rueda del ratón manteniendo el punto temporal bajo el cursor, y cabezal amarillo más visible.
- **Cerrar un proyecto ya no cierra la app**: al salir del editor se vuelve a la pantalla de inicio con el servidor y la ventana vivos, como en CapCut.
- **Título con versión**: la barra de la ventana muestra `cupcraft v0.1.0`, sin controles de ventana duplicados dentro de la UI.
- **Voces locales con Piper (TTS-Abierto)**: síntesis de voz neuronal en el equipo, sin enviar texto a la nube.

## Funciones

### Inicio y proyectos

- Pantalla de inicio con proyectos recientes, plantillas y accesos rápidos.
- Crear proyectos con relación de aspecto (16:9, 9:16, 1:1, 4:3, 21:9, 4:5), resolución (480p–4K) y FPS (24/30/60).
- Renombrar y eliminar proyectos; los cambios se guardan automáticamente en IndexedDB.
- Importar medios desde el inicio o desde el editor.

### Importación de proyectos CapCut

- Detecta los borradores nativos de CapCut instalado en Windows (`%LOCALAPPDATA%\CapCut\User Data\Projects\com.lveditor.draft`).
- Muestra nombre, fecha, relación de aspecto, duración, miniatura y número de clips antes de importar.
- Crea un proyecto nuevo de cupcraft sin modificar el borrador original: copia los medios a IndexedDB, remapea las pistas y conserva los clips de texto con sus tiempos.
- Los **stickers animados** (WebP animado y GIF) se importan completos y se reproducen sincronizados con el tiempo local de cada clip.
- Los stickers que CapCut guarda sin extensión se localizan probando varias rutas y extensiones conocidas.
- La importación funciona sin conexión y no usa servicios en la nube de CapCut.

![Lista de borradores nativos de CapCut en cupcraft](shots/new-capcut-list.png)

### Edición

- Vista previa en canvas con reproducción en tiempo real.
- Manipulación directa en la vista previa: arrastra texto y stickers, escala con las cuatro esquinas, rota con el tirador superior y usa las guías de centrado. Cada gesto se guarda como un solo paso de historial.
- Línea de tiempo con pistas de texto, video e audio.
- Importar videos, imágenes y audio; miniaturas y formas de onda automáticas.
- Mover, recortar por ambos extremos, dividir en el cursor, duplicar y eliminar clips.
- Deshacer y rehacer (hasta 80 pasos).
- Texto con estilos, tamaño, grosor, color, alineación, sombra y fondo.
- 731 filtros organizados en las 19 categorías de CapCut 9.6 (Destacado, Halloween, NUEVO, Bombazos, Vida, Paisaje, Retrato, Blanco y negro, Películas, Retro, Herramientas, Alternativo, Noche, Filtro frío, Filtro cálido, Comida, Cabina fotográfica, CCD y Mascotas), con vista previa en vivo y filtros por categoría.
- 168 efectos visuales en 19 categorías (Tendencias, Pato, Halloween, Especiales, Black Friday, Clásica, Recorte mágico, NUEVO, Bombazos, Intro y cierre, Marcos aleatorios, Fiesta, Movimiento, Movimiento 2, Ediciones, 3D, Luz, Retro y Efectos corporales). Un motor procedural propio dibuja partículas, marcos, luz, VHS, glitch y movimiento en tiempo real.
- 16 transiciones de entrada que se mezclan con el clip anterior de la pista.
- 219 stickers emoji repartidos en 19 categorías inspiradas en el panel de CapCut (Tendencias, generados por IA, Halloween, Black Friday, iconos, emoji, estilo, equipaje, amor, LETTERS, formas…), editables como capas. El arte es propio (emoji) y no se redistribuyen recursos de CapCut.
- 8 avatares locales con fondo de color, además de la opción de subir una foto.
- Estilos de subtítulo (limpio, caja, amarillo, karaoke, minimal, neón) e importación de archivos `.srt` con tiempos.
- 10 plantillas con título y subtítulo desde el inicio y desde el panel del editor.
- Volumen por clip, velocidad y silencio de pista.
- Grabación de voz con el micrófono y la API `MediaRecorder`.
- Zoom de la línea de tiempo con la rueda del ratón, centrado en el cursor, y ajuste automático al mover clips.

### Puente de control para IA

cupcraft incluye un puente HTTP local para que otra IA, un agente o un script pueda leer el estado del editor y ejecutar acciones sobre la sesión abierta en el navegador. El puente escucha únicamente en `127.0.0.1:8790` y exige un token.

- El token se guarda en `.openCUP-ai-token` o se define con `openCUP_AI_TOKEN`. También aparece en **Menú → Puente IA**.
- El puerto se puede cambiar con `openCUP_AI_PORT`; si está ocupado, el servidor prueba los siguientes puertos y muestra el elegido en el arranque.
- La aplicación debe estar abierta en el navegador. Los comandos se encolan y el navegador los recoge en su siguiente sondeo.
- Si hay varias pestañas de cupcraft abiertas, el puente controla la pestaña cargada más recientemente y `/status` indica su `target`.

| Ruta | Uso |
| --- | --- |
| `GET /api/ai/status` | Estado del puente, clientes conectados y comandos pendientes |
| `GET /api/ai/state` | Último estado recibido del editor: proyecto, pistas, clips, medios, selección y panel |
| `GET /api/ai/actions` | Catálogo de acciones y parámetros |
| `POST /api/ai/command` | Encola una acción; con `wait: true` espera el resultado |
| `GET /api/ai/result?id=…` | Consulta el resultado de un comando asíncrono |
| `GET /api/ai/commands` | Historial reciente de comandos |
| `POST /api/ai/command/cancel` | Cancela un comando pendiente |

El token se envía con `Authorization: Bearer <token>`, con `X-openCUP-Token` o como `?token=`:

```bash
curl -H "Authorization: Bearer $openCUP_AI_TOKEN" http://127.0.0.1:8790/api/ai/state
curl -X POST -H "Authorization: Bearer $openCUP_AI_TOKEN" -H "Content-Type: application/json" \
  -d '{"action":"addText","params":{"content":"Hola desde una IA","start":0,"duration":3},"wait":true}' \
  http://127.0.0.1:8790/api/ai/command
```

Acciones disponibles: `getState`, `listProjects`, `newProject`, `openProject`, `goHome`, `save`, `listCapCutProjects`, `openCapCutProject`, `setPanel`, `seek`, `play`, `pause`, `togglePlay`, `selectClip`, `addText`, `addSticker`, `applyFilter`, `applyEffect`, `applyTransition`, `updateClip`, `split`, `deleteClip`, `duplicateClip`, `undo`, `redo`, `setZoom`, `listFilters`, `listStickers`, `listEffects`, `listTransitions` y `screenshot`. La acción `screenshot` devuelve un data URL JPEG del fotograma actual.

![Diálogo del puente de control para IA](shots/new-bridge.png)

### Voces locales (TTS-Abierto)

- Síntesis de voz con **Piper** ejecutada en el propio equipo: el texto no sale de tu máquina.
- Voces en español listas para locutar textos o subtítulos y añadirlos como audio del proyecto.
- Se sirve desde `GET /api/tts/voices` y `POST /api/tts/generate`; si Piper no está instalado, el panel lo indica y no bloquea la edición.
- Los modelos se descargan aparte en `tools/piper/` y no se distribuyen con el repositorio (están excluidos en `.gitignore`).

### Exportación

- Captura del fotograma actual en PNG.
- Exportación de video WebM (VP9 + Opus) en 480p, 720p, 1080p o 4K, con el audio de videos y pistas de audio.
- La exportación se graba en tiempo real, por lo que dura aproximadamente lo mismo que el proyecto.
- Se añade el metadato de duración al WebM generado por `MediaRecorder` para que los reproductores muestren la duración correcta.

## Cómo ejecutar

cupcraft es una aplicación **Rust**. Un único ejecutable arranca el servidor local, sirve la interfaz web y la abre en una ventana **WebView2**. No usa Electron, ni Node.js, ni C++, ni .NET.

Requisitos:

- Windows 10/11 x64.
- [Rust](https://rustup.rs) con el toolchain `stable-x86_64-pc-windows-msvc`.
- Visual Studio 2022 Build Tools con «Desarrollo de escritorio en C++» (MSVC y Windows SDK).
- WebView2 Runtime (incluido en Windows 11 y en Microsoft Edge actualizado).

```bat
cupcraft.cmd
```

El lanzador compila la primera vez y abre la ventana. La barra de título muestra la versión en uso (`cupcraft v0.1.0`). Al cerrar un proyecto con la **X** del editor se vuelve a la pantalla de inicio; la ventana y el servidor siguen abiertos, como en CapCut.

También puedes hacerlo a mano:

```powershell
cd rust
cargo build -p cupcraft-desktop
.\target\debug\cupcraft-desktop.exe
```

Para usar solo el servidor (sin ventana), por ejemplo con un navegador:

```powershell
.\rust\target\debug\cupcraft-desktop.exe --server-only
```

Abre <http://127.0.0.1:8787>. El servidor escucha solo en `127.0.0.1` y no necesita dependencias externas.

También puedes cambiar los puertos: `PORT` para el servidor principal (por defecto `8787`) y `openCUP_AI_PORT` para el puente IA (por defecto `8790`).

```powershell
$env:PORT=9000; .\rust\target\debug\cupcraft-desktop.exe --server-only
```

## Aplicación de escritorio

- `rust/crates/cupcraft-desktop` crea la ventana nativa con `tao` y embebe WebView2 con `wry`.
- `rust/crates/cupcraft-server` sirve `index.html`, `src/` y `assets/`, la API `/api/capcut/*`, el proxy `/api/img`, el puente IA `/api/ai/*` y las voces locales `/api/tts/*`.
- El servidor arranca con la ventana y se detiene al cerrarla. Cerrar un proyecto solo vuelve al inicio: no termina el proceso ni abre una segunda instancia.
- Los proyectos se guardan en el almacenamiento de WebView2 (`%LOCALAPPDATA%\openCUP\WebView2`); el registro de arranque está en `%LOCALAPPDATA%\openCUP\openCUP.log`.
- Las carpetas `launcher/`, `dotnet/` y los archivos `server.mjs`/`server/*.mjs` se conservan solo como referencia histórica: la aplicación no los usa.

Para compilar en release y montar una carpeta lista para copiar:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-rust.ps1
```

El resultado es `dist/cupcraft-rust`, con `cupcraft-desktop.exe`, `index.html`, `src/` y `assets/`. Para compilar solo el binario: `cd rust; cargo build --release -p cupcraft-desktop`.

## Atajos de teclado

| Atajo | Acción |
| --- | --- |
| `Espacio` | Reproducir / pausar |
| `B` | Dividir el clip seleccionado en el cursor |
| `Supr` / `Retroceso` | Eliminar el clip seleccionado |
| `Ctrl/Cmd + Z` | Deshacer |
| `Ctrl/Cmd + Shift + Z` o `Ctrl/Cmd + Y` | Rehacer |
| `Ctrl/Cmd + D` | Duplicar el clip seleccionado |
| `Ctrl/Cmd + I` | Importar medios |
| `Ctrl/Cmd + S` | Guardar ahora |
| `←` / `→` | Fotograma anterior / siguiente |
| `Inicio` / `Fin` | Ir al principio / final |
| `Esc` | Cerrar el diálogo o deseleccionar |

Los atajos no se activan mientras escribes en un campo de texto ni con un diálogo abierto.

## Almacenamiento local

- Los proyectos y los archivos se guardan en IndexedDB (`openCUP-db`): metadatos en `projects` y blobs en `media`.
- No se sube nada a un servidor. Todo el procesamiento ocurre en el navegador.
- El servidor local en Rust solo sirve archivos estáticos y las rutas locales de CapCut y del puente IA; no copia los medios fuera del equipo.
- Al borrar los datos del sitio se eliminan también los proyectos y sus medios.
- El espacio disponible depende del límite de almacenamiento del navegador.
- La importación de CapCut lee los borradores originales en modo solo lectura; los archivos de CapCut no se modifican.

### Panel de detalles

El panel derecho permite editar el proyecto o el clip seleccionado sin salir del editor: nombre, relación de aspecto, resolución, FPS, texto, filtros, transformación, volumen, fundidos y velocidad. En pantallas estrechas se oculta para dar prioridad a la vista previa.

## Limitaciones conocidas

- Las funciones de IA en la nube de CapCut (transcripción automática, avatares animados, eliminación de fondo, generación de video) no están incluidas. La interfaz las presenta como funciones no disponibles sin conexión.
- El importador de CapCut está pensado para borradores nativos de la versión de escritorio en Windows. Reconstruye medios, pistas, textos y tiempos; los efectos, transiciones y animaciones propietarios de CapCut no se recrean.
- Los stickers en formato `.pag` de CapCut y sus animaciones propietarias todavía no se reproducen; los WebP y GIF animados sí. El seguimiento de stickers está en desarrollo.
- El puente IA solo funciona mientras cupcraft está abierto en el navegador y mientras el servidor local está en marcha. No está pensado para exponerse a Internet.
- La exportación usa `MediaRecorder`; el contenedor es WebM. Para MP4 habría que transcodificar fuera del navegador.
- La exportación es en tiempo real y no usa aceleración por GPU más allá de la que aplique el navegador.
- El formato de audio y video reproducible depende del códec soportado por el navegador.
- No hay colaboración ni cuentas: el proyecto vive en el navegador donde se creó.

## Estructura

```text
cupcraft/
├─ index.html
├─ src/
│  ├─ main.js              Arranque, atajos globales y drag & drop
│  ├─ home.js              Pantalla de inicio, proyectos y plantillas
│  ├─ editor.js            Topbar, pestañas y ciclo de vida del editor
│  ├─ store.js             Estado, acciones, historial y persistencia
│  ├─ db.js                IndexedDB
│  ├─ renderer.js          Render en canvas, efectos, transiciones y audio
│  ├─ preview.js           Vista previa y transporte
│  ├─ timeline.js          Reglas, pistas y manipulación de clips
│  ├─ panels.js            Medios, texto, stickers, efectos, filtros y detalles
│  ├─ filter-line.js       Línea propia de 82 filtros en 12 categorías (CSS + GL)
│  ├─ library.js           Catálogos de efectos, stickers, transiciones y avatares
│  ├─ capcut.js            Cliente de importación y diálogo de proyectos CapCut
│  ├─ sticker-animation.js Decodificador de stickers animados (WebP/GIF) por tiempo
│  ├─ tts-abierto.js       Cliente de voces locales Piper para el editor
│  ├─ ai-bridge.js         Puente del navegador, acciones y diálogo de control
│  ├─ export.js            Exportación WebM y captura PNG
│  ├─ webm.js              Parche de duración EBML para WebM
│  ├─ templates.js         Datos de las plantillas
│  ├─ dialogs.js           Atajos y acerca de
│  ├─ modal.js             Diálogos modales
│  ├─ icons.js             Iconos SVG
│  ├─ utils.js             Utilidades (tiempo, bytes, toasts)
│  └─ styles.css           Tema oscuro y diseño responsive
├─ cupcraft.cmd            Lanzador: compila en Rust y abre la ventana
├─ rust/                   Aplicación en Rust (cargo workspace)
│  ├─ crates/cupcraft-desktop  Ventana nativa (tao) + WebView2 (wry)
│  ├─ crates/cupcraft-server   Servidor estático, API CapCut, puente IA y TTS
│  ├─ crates/cupcraft-core     Modelo, timeline, historial y filtros
│  ├─ crates/cupcraft-persist  Almacén SQLite y archivos .opencup.json
│  └─ crates/cupcraft-app      Rewrite experimental de UI con egui
├─ server.mjs              (histórico) servidor Node equivalente, sin uso
├─ server/                 (histórico) módulos Node de CapCut y puente IA
├─ launcher/main.cpp       (histórico) lanzador C++ + WebView2
├─ tools/                  Pruebas de navegador y scripts de empaquetado
│  ├─ build-rust.ps1       Compila en release y monta dist/cupcraft-rust
│  ├─ piper/               (local) binarios y modelos de voz Piper, fuera de git
│  ├─ build-desktop.ps1    (histórico) compila el .exe en C++
│  ├─ build-installer.ps1  (histórico) instalador con Inno Setup 6
│  ├─ run-all-tests.ps1    Levanta el servidor Rust y ejecuta las suites
│  ├─ make-icon.ps1        Genera el icono multi-resolución
│  └─ fixtures/            Medios de prueba generados con ffmpeg
└─ shots/                  Capturas generadas por las pruebas
```

## Pruebas

Las pruebas usan `puppeteer-core` con un Chrome instalado. No forman parte de la aplicación y no son necesarias para usarla.

```bash
npm install                 # instala puppeteer-core (devDependency)
cupcraft.cmd --server-only  # en otra terminal, servidor Rust en 8787
npm test                    # flujo completo de edición y persistencia
npm run test:layout         # auditoría responsive 1920–760 px
npm run test:features       # renombrar, eliminar, plantillas y SRT
npm run test:integrations   # galerías, importación CapCut y puente IA
npm run test:voice          # grabación con micrófono falso de Chrome
npm run test:export         # WebM sin audio + ffprobe
npm run test:audio          # WebM con audio Opus + volumen
npm run test:visual         # capturas de inicio y editor
npm run test:preview        # arrastre, escala, rotación, stickers y deshacer
npm run test:effects        # capturas de ocho efectos procedurales
```

Notas:

- Los scripts esperan Chrome en `C:/Program Files/Google/Chrome/Application/chrome.exe` y el servidor en `http://127.0.0.1:8787`.
- `test:integrations` comprueba el puente en `8790` y abre el borrador CapCut `1007`; si no existe esa ruta, las comprobaciones de CapCut fallarán. Ajusta el id o el nombre en `tools/new-features-test.mjs` para tu equipo.
- Los medios de prueba están incluidos en `tools/fixtures/`; son videos, imágenes y audio sintéticos generados con ffmpeg.
- `test:export` y `test:audio` requieren `ffmpeg`/`ffprobe` en el `PATH`.
- Las capturas se escriben en `shots/`; los archivos exportados de prueba, en `shots/*-test/`.
- `tools/visual-test.mjs` genera capturas de la pantalla de inicio y del editor.

## Aviso

cupcraft es un proyecto educativo e independiente. CapCut es una marca de ByteDance. Este repositorio no está afiliado a CapCut ni distribuye sus recursos.

## Catálogo de CapCut

cupcraft incluye los paneles reales de CapCut (efectos, transiciones, filtros, stickers y plantillas de texto) con sus categorías en el orden original y sus portadas. Detalles del proceso en [docs/CAPCUT.md](docs/CAPCUT.md).

- Efectos: 2.469 · Transiciones: 2.047 · Filtros: 761 · Stickers: 2.167 · Texto: 429
- Conmutador CapCut / Local en cada panel.
- 13 shaders WebGL portados de paquetes de CapCut (src/gl-post.js).

Comunidad: [Discord](https://discord.gg/N2kfVGjeZ7) · [GitHub](https://github.com/Aitor2010aitor/cupcraft)
