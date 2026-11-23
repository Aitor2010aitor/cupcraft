# Zona "Eliminar Fondo" — Documentación para Claude

> Documento enfocado exclusivamente en la pestaña **Eliminar fondo** del panel Detalles de CUPCRAFT.
> Objetivo: que Claude entienda cómo funciona actualmente, qué herramientas hay, y qué se puede usar para completarla.

---

## 1. Ubicación en el código

### UI (interfaz)
- **Archivo**: `src/panels.js`
- **Función**: `cutoutTab(clip)` (línea ~3346)
- **Se accede desde**: Panel Detalles → pestaña Video → sub-pestaña "Eliminar fondo"
- **Renderizado**: `detailTabBody(clip, tab, sub)` → `if (sub === 'cutout') return prefix + cutoutTab(clip);`

### Renderizado (canvas)
- **Archivo**: `src/renderer.js`
- **Funciones clave**:
  - `applyChromaKeyPixels(ctx, clip, width, height)` — elimina color por píxel
  - `drawAiRemove(ctx, clip, width, height)` — blur de zona de eliminación de IA
  - `applyMask(ctx, clip, width, height)` — recorte con formas
  - `drawMediaLayer(ctx, layer, width, height, pool, options)` — orquesta todo

### Datos del clip
```js
clip.autoCutout          // boolean — eliminación automática
clip.cutoutStroke        // { active, type, color, size, spacing }
clip.customCutout        // { active, size }
clip.chromaKey           // { active, color, strength, shadow, feather, cleanup }
clip.mask                // { shape, scale, rotate, x, y, feather, invert }
clip.aiRemove            // boolean — eliminación de IA por zona
clip.aiRemoveArea        // { x, y, w, h } (porcentajes)
clip.aiRemoveStrength    // número (px de blur)
```

---

## 2. Estructura de la UI actual

### Secciones (en orden)

#### 2.1 Eliminación automática
- **Control**: Switch (`autoCutout`)
- **Estado**: Solo guarda un marcador booleano. No hay segmentación real.
- **Lo que debería hacer**: Separar automáticamente el objeto principal del fondo (persona, objeto) usando un modelo de segmentación.

#### 2.2 Trazo (borde de recorte)
- **Controles**:
  - Switch (`cutoutStroke.active`)
  - Grid de 8 estilos: Ninguno, Canto papel, Luminiscencia, Dibujo mano, Contorno cortado, Contorno sólido, Contorno viento, Contorneado
  - Selector de color (`cutoutStroke.color`)
  - Slider Tamaño (`cutoutStroke.size`, 0-100)
  - Slider Espaciado (`cutoutStroke.spacing`, 0-100)
- **Estado**: Solo UI y valores guardados. El trazo NO se dibuja en el canvas.
- **Lo que debería hacer**: Dibujar un borde alrededor del contorno de la máscara/recorte.

#### 2.3 Eliminación personalizada
- **Controles**:
  - Switch (`customCutout.active`)
  - 4 botones: Pincel, Pincel rápido, Lazo, Borrador
  - Slider Tamaño (`customCutout.size`, 1-100)
  - Botón "Aplicar"
- **Estado**: Solo UI. No hay editor de máscara pintada.
- **Lo que debería hacer**: Permitir pintar una máscara con el ratón (pincel, lazo, borrador) y aplicarla como recorte.

#### 2.4 Chroma key
- **Controles**:
  - Switch (`chromaKey.active`)
  - Selector de color (`chromaKey.color`) — color a eliminar
  - Slider Intensidad (`chromaKey.strength`, 0-100)
  - Slider Sombra (`chromaKey.shadow`, 0-100)
  - Slider Borde de plumas (`chromaKey.feather`, 0-100)
  - Slider Limpiar borde (`chromaKey.cleanup`, 0-100)
- **Estado**: ✅ Funciona. Elimina el color elegido píxel a píxel. Los sliders de sombra, plumas y limpieza ya afectan al resultado.
- **Lo que podría mejorar**: Despill (eliminar color derramado en bordes), refinado de bordes con filtro guiado.

#### 2.5 Recortar con máscara
- **Control**: Botón que salta a la sub-pestaña "Máscara"
- **Estado**: ✅ Funciona. Recorta con 9 formas + invertir.

---

## 3. Cómo funciona el renderizado actual

### Flujo en drawMediaLayer (renderer.js)

```
1. Obtener clip y media
2. Calcular transformación (posición, escala, rotación)
3. Aplicar máscara (ctx.clip()) si está activa
4. Dibujar medio (video/imagen) con drawMediaSource o drawMediaSourceChroma
   - Si chromaKey.active → dibuja a offscreen, aplica applyChromaKeyPixels, luego drawImage
5. Dibujar ghosts/clones si hay efecto
6. Aplicar drawAiRemove (blur de zona) si aiRemove está activo
7. Aplicar viñeta
8. Restaurar máscara y transformación
```

### applyChromaKeyPixels (función actual)

```js
function applyChromaKeyPixels(ctx, clip, width, height) {
  const ck = clip?.chromaKey || {};
  const keyColor = ck.color || '#00ff00';
  const strength = (ck.strength ?? 20) / 100;      // 0-1
  const feather = (ck.feather ?? 0) / 100;          // 0-1
  const shadow = (ck.shadow ?? 0) / 100;            // 0-1
  const cleanup = (ck.cleanup ?? 0) / 100;           // 0-1

  // Parsear color clave
  let kr = 0, kg = 255, kb = 0;
  if (keyColor.startsWith('#') && keyColor.length >= 7) {
    kr = parseInt(keyColor.slice(1, 3), 16);
    kg = parseInt(keyColor.slice(3, 5), 16);
    kb = parseInt(keyColor.slice(5, 7), 16);
  }

  const imgData = ctx.getImageData(0, 0, width, height);
  const data = imgData.data;
  const threshold = 0.15 + strength * 0.6 + cleanup * 0.15;
  const featherBand = Math.max(0.02, feather + 0.05);

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i], g = data[i + 1], b = data[i + 2];
    const dist = Math.sqrt((r - kr) ** 2 + (g - kg) ** 2 + (b - kb) ** 2) / 441.67;

    if (dist < threshold) {
      // Eliminar píxel (hacer transparente)
      const alpha = Math.min(1, (threshold - dist) / featherBand);
      data[i + 3] = Math.round(data[i + 3] * (1 - alpha));
    } else if (shadow > 0) {
      // Oscurecer el resto (simular sombra)
      data[i] = Math.round(data[i] * (1 - shadow * 0.25));
      data[i + 1] = Math.round(data[i + 1] * (1 - shadow * 0.25));
      data[i + 2] = Math.round(data[i + 2] * (1 - shadow * 0.25));
    }
  }
  ctx.putImageData(imgData, 0, 0);
}
```

### drawAiRemove (función actual)

```js
function drawAiRemove(ctx, clip, width, height) {
  if (!clip.aiRemove) return;
  const area = clip.aiRemoveArea || {};
  const w = width * Math.max(0.02, Math.min(1, (area.w ?? 30) / 100));
  const h = height * Math.max(0.02, Math.min(1, (area.h ?? 30) / 100));
  const dx = (width * (area.x ?? 0)) / 100 - w / 2;
  const dy = (height * (area.y ?? 0)) / 100 - h / 2;
  const sx = dx + width / 2;
  const sy = dy + height / 2;
  const blur = Math.max(4, Math.min(60, Number(clip.aiRemoveStrength ?? 28)));

  ctx.save();
  ctx.beginPath();
  ctx.rect(dx, dy, w, h);
  ctx.clip();
  ctx.filter = `blur(${blur}px)`;
  try {
    ctx.drawImage(ctx.canvas, sx, sy, w, h, dx, dy, w, h);
  } catch { /* canvas no accesible */ }
  ctx.restore();
}
```

### applyMask (función actual)

```js
function applyMask(ctx, clip, width, height) {
  if (!maskActive(clip)) return false;
  const invert = !!clip.mask.invert;
  ctx.save();
  if (invert) {
    ctx.beginPath();
    ctx.rect(-width * 2, -height * 2, width * 4, height * 4);
    traceMaskPath(ctx, clip, width, height);
    ctx.clip('evenodd');
  } else {
    traceMaskPath(ctx, clip, width, height);
    ctx.clip();
  }
  return true;
}
```

### traceMaskPath (9 formas)

```js
function traceMaskPath(ctx, clip, width, height) {
  const mask = clip.mask || {};
  const scale = Math.max(0.05, Number(mask.scale ?? 1));
  const w = width * scale;
  const h = height * scale;
  ctx.translate(((mask.x ?? 0) / 100) * width, ((mask.y ?? 0) / 100) * height);
  if (mask.rotate) ctx.rotate((mask.rotate * Math.PI) / 180);
  ctx.beginPath();
  switch (mask.shape) {
    case 'circle': ctx.ellipse(0, 0, w / 2, h / 2, 0, 0, Math.PI * 2); break;
    case 'rect': ctx.rect(-w / 2, -h / 2, w, h); break;
    case 'rounded': ctx.roundRect(-w / 2, -h / 2, w, h, Math.min(w, h) * 0.12); break;
    case 'triangle': /* triángulo */ break;
    case 'heart': /* corazón con bezier */ break;
    case 'star': /* estrella de 5 puntas */ break;
    case 'linear': /* forma diagonal */ break;
    case 'mirror': ctx.rect(-w / 2, -h / 2, w / 2, h); break;
    default: ctx.rect(-w / 2, -h / 2, w, h);
  }
}
```

---

## 4. Herramientas Open-Source para completar "Eliminar fondo"

### 4.1 Recorte automático (segmentación)

| Herramienta | Licencia | Tipo | Velocidad | Calidad bordes | Integración |
|---|---|---|---|---|
| **MediaPipe Selfie Segmentation** | Apache-2.0 | Modelo TFLite | ~30 FPS (CPU) | Buena para persona | WASM/JS, fácil |
| **U²-Net** | Revisar (código MIT, pesos unclear) | ONNX | ~5-10 FPS (CPU) | Excelente | ONNX Runtime WASM |
| **MODNet** | Revisar | ONNX | ~10-15 FPS (CPU) | Excelente (pelo) | ONNX Runtime WASM |
| **BodyPix** (TensorFlow.js) | Apache-2.0 | TF.js | ~10-20 FPS (CPU) | Buena para cuerpo | TF.js, fácil |
| **BackgroundMattingV2** | MIT | PyTorch/ONNX | ~15-30 FPS (GPU) | Excelente | Requiere foto de fondo |
| **RobustVideoMatting** | GPL-3.0 ⚠️ | PyTorch/ONNX | ~20-40 FPS (GPU) | Excelente | Copyleft, evitar si no es GPL |

**Recomendación**: MediaPipe Selfie Segmentation para tiempo real (rápido, Apache-2.0). Si se necesita mejor calidad de bordes, U²-Net o MODNet.

### 4.2 Trazo (borde de recorte)

**No necesita IA.** Se implementa con Canvas 2D:

```js
// Pseudocódigo para trazo
function drawStroke(ctx, clip, width, height) {
  if (!clip.cutoutStroke?.active) return;
  const mask = clip.mask || {};
  const color = clip.cutoutStroke.color || '#ffffff';
  const size = clip.cutoutStroke.size || 10;
  const spacing = clip.cutoutStroke.spacing || 10;

  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = size;
  ctx.setLineDash(spacing > 0 ? [spacing, spacing] : []);
  traceMaskPath(ctx, clip, width, height);
  ctx.stroke();
  ctx.restore();
}
```

**Estilos de trazo** (los 8 de CapCut):
1. Ninguno — sin borde
2. Canto papel — borde irregular/ondulado
3. Luminiscencia — borde con glow (shadowBlur)
4. Dibujo mano — borde estilo sketch (líneas irregulares)
5. Contorno cortado — borde con efecto de corte
6. Contorno sólido — borde sólido simple
7. Contorno viento — borde con efecto de viento (líneas direccionales)
8. Contorneado — borde con patrón de puntos

Los estilos 2-8 requieren shaders o dibujo procedural. Los más simples (sólido, puntos) se pueden hacer con Canvas 2D. Los complejos (papel, viento) necesitan WebGL/WGSL.

### 4.3 Eliminación personalizada (pincel/lazo/borrador)

**No necesita IA para el pincel básico.** Se implementa con:

1. **Canvas de máscara**: un canvas offscreen donde se pinta la máscara.
2. **Herramientas**:
   - Pincel: dibuja círculos en la máscara
   - Borrador: borra círculos de la máscara
   - Lazo: selecciona un área poligonal
   - Pincel rápido: pincel con suavizado (interpolación)
3. **Aplicar**: usa la máscara pintada como `ctx.clip()` en el renderizado.

```js
// Pseudocódigo para pincel
function paintMask(maskCanvas, x, y, radius, mode) {
  const ctx = maskCanvas.getContext('2d');
  ctx.globalCompositeOperation = mode === 'erase' ? 'destination-out' : 'source-over';
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.fill();
}

// En el renderizado:
function applyPaintedMask(ctx, clip, width, height) {
  if (!clip.customCutout?.active) return false;
  const mask = clip.customCutout.mask; // ImageData o canvas
  if (!mask) return false;
  ctx.save();
  ctx.clip(mask); // usar la máscara pintada
  return true;
}
```

**Para clics inteligentes (lazo que detecta bordes)**: Segment Anything (SAM) o MobileSAM. El usuario hace clic y el modelo genera una máscara automáticamente.

### 4.4 Chroma key mejorado (despill)

**Despill** elimina el color del fondo que se "derrama" en los bordes del objeto. Se implementa post-chroma-key:

```js
// Despill: reducir el componente del color clave en los píxeles restantes
function despill(data, kr, kg, kb, strength) {
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] > 0) { // píxel no eliminado
      // Restar el componente del color clave
      const r = data[i], g = data[i + 1], b = data[i + 2];
      const factor = strength * 0.5;
      data[i] = Math.max(0, r - kr * factor);
      data[i + 1] = Math.max(0, g - kg * factor);
      data[i + 2] = Math.max(0, b - kb * factor);
    }
  }
}
```

### 4.5 Refinado de bordes (feather, cleanup)

- **Feather**: ya implementado (gradiente de alpha en el borde).
- **Cleanup**: eliminar píxeles aislados (ruido) en la máscara. Se puede hacer con un filtro de mediana o morfológico.
- **Sombra**: ya implementado (oscurecer el resto).

---

## 5. Flujo de datos actual

```
Usuario selecciona clip
  → panels.js: cutoutTab(clip) genera HTML
  → Usuario cambia switch/slider
  → updateClip(found.clip.id, { chromaKey: { ... } })
  → store.js: guarda en el clip
  → renderer.js: drawMediaLayer lee clip.chromaKey
  → applyChromaKeyPixels: procesa píxeles
  → Canvas actualizado
```

---

## 6. Archivos a modificar para completar "Eliminar fondo"

| Archivo | Qué modificar |
|---|---|
| `src/panels.js` → `cutoutTab()` | Añadir controles de trazo, pincel, etc. |
| `src/renderer.js` → `applyChromaKeyPixels()` | Añadir despill, refinado de bordes |
| `src/renderer.js` → `drawMediaLayer()` | Integrar trazo, máscara pintada |
| `src/renderer.js` → nueva `drawStroke()` | Dibujar borde de recorte |
| `src/renderer.js` → nueva `applyPaintedMask()` | Aplicar máscara pintada |
| `src/styles.css` | Estilos para nuevos controles |
| `src/icons.js` | Iconos para pincel, lazo, borrador |

---

## 7. Herramientas open-source recomendadas por función

| Función | Herramienta | Licencia | Integración |
|---|---|---|---|
| Recorte automático | MediaPipe Selfie Segmentation | Apache-2.0 | WASM, ~30 FPS |
| Recorte automático (mejor calidad) | U²-Net / MODNet | Revisar | ONNX Runtime WASM |
| Pincel/lazo inteligente | Segment Anything (SAM) | Revisar | ONNX, clics → máscara |
| Trazo simple (sólido, puntos) | Canvas 2D nativo | Nativo | `ctx.stroke()` |
| Trazo complejo (papel, viento) | WebGL/WGSL shaders | Nativo | Portar de CapCut |
| Despill | Algoritmo propio | Nativo | Post-chroma-key |
| Refinado de bordes | Filtro guiado (guided filter) | Nativo | Algoritmo propio en JS |
| Object removal (inpainting) | LaMa (Large Mask Inpainting) | Apache-2.0 | ONNX, ~1-2s por frame |

---

## 8. Lo que NO necesita IA (se puede hacer con Canvas 2D nativo)

1. **Máscara** (ya funciona)
2. **Chroma key** (ya funciona)
3. **Trazo simple** (sólido, puntos, rayas) — `ctx.stroke()` con `setLineDash`
4. **Pincel/borrador básico** — pintar en canvas offscreen
5. **Despill** — algoritmo de resta de color
6. **Feather/plumas** — gradiente de alpha (ya funciona)
7. **Sombra** — oscurecer píxeles (ya funciona)

## 9. Lo que SÍ necesita IA o procesamiento pesado

1. **Recorte automático** → MediaPipe/U²-Net/MODNet
2. **Pincel inteligente (lazo que detecta bordes)** → SAM/MobileSAM
3. **Object removal (eliminar objeto y rellenar fondo)** → LaMa/inpainting
4. **Mejora de calidad (super-resolución)** → Real-ESRGAN
5. **Trazo complejo (papel, viento)** → Shaders WebGL/WGSL

---

*Documento enfocado en la zona "Eliminar fondo" para revisión por Claude.*