# Tokens de Interfaz y Diseño Visual (Fase 0)

Este documento extrae todos los valores de diseño, colores, medidas, tipografías y radios de `src/styles.css` para recrear con exactitud pixel-perfect la interfaz en Rust utilizando `egui` y shaders en GPU.

---

## 1. Paleta de Colores

| Token CSS | Valor Hexadecimal | Uso en la Interfaz |
|---|---|---|
| `--bg` | `#141414` | Fondo general de la ventana y de la barra de título. |
| `--bg-2` | `#101010` | Fondo más oscuro de contraste. |
| `--panel` | `#262626` | Fondo principal de paneles laterales y línea de tiempo. |
| `--panel-2` | `#2b2b2b` | Fondo secundario para modales y popovers. |
| `--panel-3` | `#303030` | Tira superior de pestañas de herramientas y tarjetas de medio. |
| `--field` | `#3b3b3b` | Fondo de campos de entrada, botones de importación y zona drop. |
| `--elev` | `#333333` | Fondo de elementos elevados y botones estándar. |
| `--line` | `#1d1d1d` | Bordes divisorios principales entre paneles. |
| `--line-soft` | `#333333` | Bordes suaves secundarios. |
| `--text` | `#f2f2f2` | Color principal de textos y etiquetas. |
| `--muted` | `#8a8a8a` | Texto secundario y etiquetas atenuadas. |
| `--muted-2` | `#6c6c6c` | Texto deshabilitado o iconos inactivos. |
| `--accent` | `#00e5f0` | Cyan brillante: acentos activos, tiradores y selecciones. |
| `--accent-2` | `#00c1cd` | Cyan principal: botones primarios y bordes activos. |
| `--accent-soft` | `#123b3e` | Fondo cyan atenuado para botones e ítems seleccionados. |
| `--accent-ink` | `#041412` | Texto muy oscuro para contrastar sobre fondos cyan. |
| `--blue` | `#4d8dff` | Azul para enlaces e información. |
| `--danger` | `#ff5c5c` | Rojo de advertencia/peligro y borrado. |
| `--warn` | `#ffc857` | Amarillo de aviso o atención. |

---

## 2. Colores Específicos de Clips y Pistas

| Tipo de Clip | Gradiente de Fondo | Borde Normal | Borde Seleccionado |
|---|---|---|---|
| **Video** | `linear-gradient(135deg, #194647, #133033)` | `#2c7474` | `#00c1cd` con resplandor cyan |
| **Imagen** | `linear-gradient(135deg, #2b3038, #1b2026)` | `#46535f` | `#00c1cd` con resplandor cyan |
| **Audio** | `linear-gradient(135deg, #2a2a52, #191a2e)` | `#4a4a7d` | `#00c1cd` con resplandor cyan |
| **Texto** | `linear-gradient(135deg, #3b2a46, #241a2d)` | `#6f4d86` | `#00c1cd` con resplandor cyan |
| **Efecto / Filtro** | `linear-gradient(180deg, #7b46d6, #5a2fa8)` | `#a678ff` | `#00c1cd` con resplandor cyan |

---

## 3. Elementos Gráficos Clave de la Línea de Tiempo

- **Cabezal de lectura (`playhead`)**:
  - Línea vertical: Ancho `1px`, color `#e8e8e8`, sombra sutil.
  - Puntero superior (`playhead-head`): Ancho `10px`, alto `12px`, color `#ffdd00` (amarillo dorado), forma de triángulo invertido (`clip-path: polygon(0 0, 100% 0, 50% 100%)`) con sombra dorada resplandeciente (`rgba(255, 221, 0, 0.8)`).
- **Rombos de Keyframe (`kf-diamond`)**:
  - Tamaño `7px x 7px`, rotados 45°, color `#ffd166`, situados a `3px` del borde inferior del clip.
- **Marcadores de regla (`ruler-marker`)**:
  - Línea vertical `#f1b332` con rombo superior de `8px x 8px` rotado 45° con sombra `rgba(241, 179, 50, 0.7)`.
- **Tiradores de recorte (`clip-handle`)**:
  - Ancho de captura: `7px` a los extremos izquierdo y derecho; al pasar el cursor o arrastrar se iluminan en `rgba(0, 229, 240, 0.65)`.

---

## 4. Medidas y Disposición (Layout)

| Parámetro | Valor | Descripción |
|---|---|---|
| `--topbar-h` | `42px` | Altura fija de la barra de título superior. |
| `--track-h` | `56px` | Altura fija de cada pista en la línea de tiempo. |
| `--label-w` | `140px` | Ancho de la columna izquierda de cabeceras de pista. |
| `--timeline-h` | `clamp(320px, 42vh, 440px)` | Altura flexible del panel inferior de la línea de tiempo. |
| `--radius` | `8px` | Radio de curvatura de esquinas exteriores de paneles principales. |
| Radio de botón | `4px` - `5px` | Radio de esquinas para botones estándar y campos de texto. |
| Radio de clip | `3px` | Radio de esquinas de los bloques de clip en la línea de tiempo. |
| Divisor vertical (`--split-w`)| `6px` | Ancho del canal redimensionable entre las 3 columnas superiores. |
| Tira de herramientas | `44px` | Altura de la tira de iconos/pestañas de la columna izquierda. |
| Rango de zoom | `30` a `260` px/s | Escala horizontal de la línea de tiempo (por defecto `78` px/s). |

---

## 5. Tipografía

- **Familia tipográfica**: `"Segoe UI Variable Text", "Segoe UI", Inter, system-ui, sans-serif`
- **Jerarquía de tamaños**:
  - Títulos de sección: `12px` (peso 600)
  - Botones y campos: `12px` (peso 500)
  - Texto estándar / cuerpo: `13px` (peso 400)
  - Pestañas y cabeceras de pista: `10px` - `11px` (peso 500)
  - Nombres y tiempos de clips: `9px` (peso 600, tabular-nums)
  - Metadatos diminutos y badges: `8px` - `8.5px` (peso 700)
