# openCUP

<img width="1686" height="934" alt="{9E045220-ABD4-45D7-AB6E-E135D48AB93C}" src="https://github.com/user-attachments/assets/9fbae1eb-ff95-453c-ada1-d43bfebb9624" />

Editor de video local en el navegador, inspirado en el flujo de trabajo y la interfaz oscura de CapCut. openCUP es una implementación original: no incluye binarios, logotipos, tipografías ni recursos propietarios de CapCut.

![Pantalla de inicio de openCUP](shots/01-home.png)

![Editor de openCUP con panel de texto](shots/20-template.png)

## Funciones

### Inicio y proyectos

- Pantalla de inicio con proyectos recientes, plantillas y accesos rápidos.
- Crear proyectos con relación de aspecto (16:9, 9:16, 1:1, 4:3, 21:9, 4:5), resolución (480p–4K) y FPS (24/30/60).
- Renombrar y eliminar proyectos; los cambios se guardan automáticamente en IndexedDB.
- Importar medios desde el inicio o desde el editor.

### Importación de proyectos CapCut

- Detecta los borradores nativos de CapCut instalado en Windows (`%LOCALAPPDATA%\CapCut\User Data\Projects\com.lveditor.draft`).
- Muestra nombre, fecha, relación de aspecto, duración, miniatura y número de clips antes de importar.
- Crea un proyecto nuevo de openCUP sin modificar el borrador original: copia los medios a IndexedDB, remapea las pistas y conserva los clips de texto con sus tiempos.
- La importación funciona sin conexión y no usa servicios en la nube de CapCut.

![Lista de borradores nativos de CapCut en openCUP](shots/new-capcut-list.png)

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
- Zoom de la línea de tiempo y ajuste automático al soltar clips.

### Puente de control para IA

openCUP incluye un puente HTTP local para que otra IA, un agente o un script pueda leer el estado del editor y ejecutar acciones sobre la sesión abierta en el navegador. El puente escucha únicamente en `127.0.0.1:8790` y exige un token.

- El token se guarda en `.openCUP-ai-token` o se define con `openCUP_AI_TOKEN`. También aparece en **Menú → Puente IA**.
- El puerto se puede cambiar con `openCUP_AI_PORT`; si está ocupado, el servidor prueba los siguientes puertos y muestra el elegido en el arranque.
- La aplicación debe estar abierta en el navegador. Los comandos se encolan y el navegador los recoge en su siguiente sondeo.
- Si hay varias pestañas de openCUP abiertas, el puente controla la pestaña cargada más recientemente y `/status` indica su `target`.

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

### Exportación

- Captura del fotograma actual en PNG.
- Exportación de video WebM (VP9 + Opus) en 480p, 720p, 1080p o 4K, con el audio de videos y pistas de audio.
- La exportación se graba en tiempo real, por lo que dura aproximadamente lo mismo que el proyecto.
- Se añade el metadato de duración al WebM generado por `MediaRecorder` para que los reproductores muestren la duración correcta.

## Cómo ejecutar

Requisitos:

- Node.js 18 o superior (probado con Node.js 24.22.1).
- Un navegador Chromium actualizado (Chrome, Edge o Brave) con soporte para `MediaRecorder`, `canvas.captureStream` y Web Audio.

```bash
npm start
```

Abre <http://127.0.0.1:8787>. El servidor escucha solo en `127.0.0.1`; no hay dependencias para ejecutar la aplicación.

También puedes cambiar el puerto:

```powershell
$env:PORT=9000; npm start
```

## Aplicación de escritorio

La carpeta `dist/openCUP` contiene una aplicación nativa para Windows x64:

- `openCUP.exe` es un ejecutable **C++ nativo** (Win32 + WebView2 COM): no usa .NET, no necesita DLLs gestionadas y ocupa ~330 KB.
- Abre la interfaz con WebView2 (Edge embebido) en una ventana propia con tema oscuro.
- Incluye `node.exe`, por lo que no hace falta tener Node.js instalado.
- El servidor local arranca oculto al abrir la app y se detiene al cerrar la ventana. Va dentro de un Job de Windows: si la app muere a la fuerza, el servidor hijo se cierra con ella (no quedan procesos huérfanos).
- Solo puede haber una instancia abierta a la vez.
- Los proyectos se guardan en `%LOCALAPPDATA%\openCUP\WebView2`; el registro de arranque está en `%LOCALAPPDATA%\openCUP\openCUP.log`.
- La carpeta se puede copiar a otro equipo Windows 10/11 con WebView2 Runtime.

Para generar de nuevo el ejecutable, el icono y el acceso directo del escritorio:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-desktop.ps1
```

Para generar además un instalador (Inno Setup 6, descargado automáticamente si hace falta):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-installer.ps1
```

El resultado es `dist/openCUP-Setup-0.3.0.exe` (~24 MB): asistente en español, icono en el escritorio y en el menú Inicio, desinstalador integrado en "Aplicaciones y características". Requiere permisos de administrador para instalar en `Archivos de programa`.

El script de compilación localiza las herramientas C++ de MSVC (Visual Studio / Build Tools), descarga el SDK de WebView2 con `tools/fetch-webview2.ps1` si falta, compila `launcher/main.cpp` e incrusta el icono (`rc.exe`), y copia la aplicación, el servidor y Node.js en `dist/openCUP`. Requiere el componente «Desarrollo de escritorio en C++» de Visual Studio. El instalador requiere Inno Setup 6 (`tools/build-installer.ps1` lo descarga e instala automáticamente si no está).

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
- El servidor de Node solo sirve archivos estáticos y las rutas locales de CapCut y del puente IA; no copia los medios fuera del equipo.
- Al borrar los datos del sitio se eliminan también los proyectos y sus medios.
- El espacio disponible depende del límite de almacenamiento del navegador.
- La importación de CapCut lee los borradores originales en modo solo lectura; los archivos de CapCut no se modifican.

### Panel de detalles

El panel derecho permite editar el proyecto o el clip seleccionado sin salir del editor: nombre, relación de aspecto, resolución, FPS, texto, filtros, transformación, volumen, fundidos y velocidad. En pantallas estrechas se oculta para dar prioridad a la vista previa.

## Limitaciones conocidas

- Las funciones de IA en la nube de CapCut (transcripción automática, avatares animados, eliminación de fondo, generación de video) no están incluidas. La interfaz las presenta como funciones no disponibles sin conexión.
- El importador de CapCut está pensado para borradores nativos de la versión de escritorio en Windows. Reconstruye medios, pistas, textos y tiempos; los efectos, transiciones y animaciones propietarios de CapCut no se recrean.
- El puente IA solo funciona mientras openCUP está abierto en el navegador y mientras el servidor local está en marcha. No está pensado para exponerse a Internet.
- La exportación usa `MediaRecorder`; el contenedor es WebM. Para MP4 habría que transcodificar fuera del navegador.
- La exportación es en tiempo real y no usa aceleración por GPU más allá de la que aplique el navegador.
- El formato de audio y video reproducible depende del códec soportado por el navegador.
- No hay colaboración ni cuentas: el proyecto vive en el navegador donde se creó.

## Estructura

```text
openCUP/
├─ index.html
├─ server.mjs              Servidor estático, API de CapCut y API interna de IA
├─ server/
│  ├─ capcut.mjs           Escaneo e importación de borradores nativos de CapCut
│  └─ ai-bridge.mjs        Puerto 8790, autenticación, comandos y documentación
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
│  ├─ filter-catalog.js    Catálogo de 731 filtros en 19 categorías estilo CapCut
│  ├─ library.js           Catálogos de efectos, stickers, transiciones y avatares
│  ├─ capcut.js            Cliente de importación y diálogo de proyectos CapCut
│  ├─ ai-bridge.js         Puente del navegador, acciones y diálogo de control
│  ├─ export.js            Exportación WebM y captura PNG
│  ├─ webm.js              Parche de duración EBML para WebM
│  ├─ templates.js         Datos de las plantillas
│  ├─ dialogs.js           Atajos y acerca de
│  ├─ modal.js             Diálogos modales
│  ├─ icons.js             Iconos SVG
│  ├─ utils.js             Utilidades (tiempo, bytes, toasts)
│  └─ styles.css           Tema oscuro y diseño responsive
├─ launcher/main.cpp        Lanzador Win32 + WebView2 escrito en C++ puro
├─ tools/                  Pruebas de navegador y scripts de empaquetado
│  ├─ build-desktop.ps1    Compila el .exe en C++ y monta dist/openCUP
│  ├─ build-installer.ps1  Genera dist/openCUP-Setup-0.3.0.exe con Inno Setup 6
│  ├─ openCUP.iss            Script de Inno Setup (español, desinstalador)
│  ├─ fetch-webview2.ps1   Descarga el SDK nativo de WebView2 desde nuget.org
│  ├─ run-all-tests.ps1    Levanta el servidor y ejecuta las 11 suites
│  ├─ make-icon.ps1        Genera el icono multi-resolución
│  └─ fixtures/            Medios de prueba generados con ffmpeg
└─ shots/                  Capturas generadas por las pruebas
```

## Pruebas

Las pruebas usan `puppeteer-core` con un Chrome instalado. No forman parte de la aplicación y no son necesarias para usarla.

```bash
npm install                 # instala puppeteer-core (devDependency)
npm start                   # en otra terminal, deja el servidor en 8787
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

openCUP es un proyecto educativo e independiente. CapCut es una marca de ByteDance. Este repositorio no está afiliado a CapCut ni distribuye sus recursos.

## Catálogo de CapCut

openCUP incluye los paneles reales de CapCut (efectos, transiciones, filtros, stickers y plantillas de texto) con sus categorías en el orden original y sus portadas. Detalles del proceso en [docs/CAPCUT.md](docs/CAPCUT.md).

- Efectos: 2.469 · Transiciones: 2.047 · Filtros: 761 · Stickers: 2.167 · Texto: 429
- Conmutador CapCut / Local en cada panel.
- 13 shaders WebGL portados de paquetes de CapCut (src/gl-post.js).

Comunidad: [Discord](https://discord.gg/N2kfVGjeZ7) · [GitHub](https://github.com/Aitor2010aitor/openCUP)
