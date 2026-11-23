# Guía: eliminación de fondo en cupcraft (versión nativa)

Documento solo sobre la pestaña **Video → Eliminar fondo**. Describe el estado actual, lo que falta, cómo implementarlo sin navegador y qué herramientas open source usar.

Estado de las fuentes: el análisis parte del documento de contexto de la pestaña que aportó el usuario. El código real de esa versión no está en GitHub (rama `0.4.5`); hay que revisarlo contra la copia de trabajo antes de dar nada por cerrado.

---

## 1. Estado actual

| Sección | Control | Estado | Qué falta |
|---|---|---|---|
| Eliminación automática | Switch `autoCutout` | Solo guarda un booleano | Segmentación real (persona/objeto) |
| Trazo | Switch, 8 estilos, color, tamaño, espaciado | Solo UI; no se dibuja | Borde alrededor del recorte |
| Eliminación personalizada | Switch, Pincel, Pincel rápido, Lazo, Borrador, tamaño, Aplicar | Solo UI | Editor de máscara y almacenamiento |
| Chroma key | Switch, color, intensidad, sombra, plumas, limpiar borde | **Funciona** (por píxel, en CPU) | Despill y refinado de bordes |
| Recortar con máscara | Botón que lleva a la sub-pestaña Máscara | **Funciona** (9 formas + invertir) | Nada |

Aparte, existe **«Eliminar IA» por zona** (`aiRemove`): desenfoca un rectángulo. No es inteligencia artificial; conviene renombrarlo en la interfaz.

### 1.1 Datos del clip

```
clip.autoCutout          boolean
clip.cutoutStroke        { active, type, color, size, spacing }
clip.customCutout        { active, size }          ← falta: la máscara pintada
clip.chromaKey           { active, color, strength, shadow, feather, cleanup }
clip.mask                { shape, scale, rotate, x, y, feather, invert }
clip.aiRemove            boolean
clip.aiRemoveArea        { x, y, w, h }            (porcentajes)
clip.aiRemoveStrength    número (px de desenfoque)
```

---

## 2. Qué hay que cambiar

### 2.1 Trazo (borde de recorte)

- El trazo **no** puede salir solo de las formas geométricas: sirve para las 9 máscaras, pero no para un recorte automático o de chroma.
- Solución: calcular el **campo de distancia** de la máscara alfa (distancia de cada píxel al borde) y dibujar un anillo de `size` píxeles con el color y el espaciado elegidos. Se hace en GPU con WGSL, o en CPU a baja resolución si la máscara cambia poco.
- Los 8 estilos (ninguno, canto papel, luminiscencia, dibujo a mano, contorno cortado, sólido, viento, puntos):
  - **Sólido y puntos**: anillo con máscara de discontinuidad. Viables en la primera versión.
  - **Luminiscencia**: anillo con desenfoque exterior.
  - **Papel, dibujo a mano y viento**: ruido procedural sobre el anillo. Fase posterior.

### 2.2 Eliminación personalizada (pincel, lazo, borrador)

- Falta **dónde guardar la máscara pintada**. Propuesta: un canal alfa de la resolución del vídeo (o a media resolución), guardado como PNG en la base de datos local y referenciado desde `customCutout.maskRef`.
- **No** usar `ctx.clip(mask)`: el canvas solo acepta rutas, no máscaras de píxeles. En nativo, la máscara se multiplica por el alfa en un shader.
- Herramientas:
  - **Pincel y borrador**: círculos con borde suave escritos sobre la máscara.
  - **Pincel rápido**: interpolación entre puntos del trazo para evitar huecos.
  - **Lazo**: polígono que se rasteriza en la máscara al cerrarlo.
  - **Pincel inteligente** (opcional): clics que generan la máscara con SAM o MobileSAM.

### 2.3 Chroma key

- Hoy se calcula en CPU, píxel a píxel, en cada fotograma. Pasarlo a WGSL.
- **Despill**: el método del documento (restar un valor fijo) oscurece los bordes. Para verde, la técnica habitual es limitar el canal verde: `g = min(g, max(r, b))` solo en píxeles cercanos al color clave, con peso según la distancia. Para azul, lo mismo con el canal azul.
- **Refinado**: eliminar píxeles aislados de la máscara (apertura morfológica) y suavizar bordes (filtro guiado).

### 2.4 Eliminación automática (segmentación)

- Ejecutar el modelo a **baja resolución** (por ejemplo 256 px de lado) y reescalar la máscara en GPU. Así se mantiene el tiempo real en un equipo modesto.
- Ejecutar la inferencia en un hilo de fondo. Mientras tanto, mostrar la última máscara válida.
- Guardar la máscara por fotograma (o cada N fotogramas e interpolar) para no recalcularla al reproducir.

### 2.5 Eliminar IA (zona)

- Renombrar la función en la interfaz a «Desenfoque de zona» para que no se confunda con IA.
- Pasar el desenfoque a un shader. El método actual dibuja el canvas sobre sí mismo con `ctx.filter`, que no existe en nativo.

---

## 3. Cadena de render propuesta

```
fotograma (decodificado en hilo de fondo)
  → [opcional] segmentación ONNX a baja resolución → máscara alfa (textura)
  → chroma key (WGSL): tolerancia, sombra, plumas
  → despill (WGSL)
  → máscara pintada o forma geométrica (WGSL: multiplica el alfa)
  → trazo: campo de distancia → anillo de color (WGSL)
  → desenfoque de zona
  → viñeta y ajustes de color
  → composición
```

Principios:
- Todo el procesado de píxeles va en GPU. La CPU solo decodifica y prepara máscaras.
- Cada paso debe poder desactivarse sin pasar por los demás.
- Las máscaras se calculan a la resolución del vídeo solo si el usuario lo necesita; si no, a media resolución.

---

## 4. Herramientas open source

Licencias verificadas en el repositorio oficial están marcadas como **verificado**. El resto, **revisar** antes de usarlo.

| Herramienta | Uso en esta zona | Licencia | Estado |
|---|---|---|---|
| **BackgroundMattingV2** (github.com/PeterL1n/BackgroundMattingV2) | Matting de vídeo con cámara fija; necesita foto del fondo | MIT | Verificado |
| **RobustVideoMatting** (github.com/PeterL1n/RobustVideoMatting) | Matting de vídeo sin foto de fondo | **GPL-3.0** (copyleft) | Verificado. **No usar** si el proyecto no es GPL |
| **rembg** (github.com/danielgatis/rembg) | Quitar fondo de imagen | MIT (código) | Verificado. Su modelo por defecto (`bria-rmbg`) requiere acuerdo de pago para uso comercial. **No usar ese modelo** |
| **MediaPipe Selfie Segmentation** | Recorte automático de persona, tiempo real | Apache-2.0 | Revisar en el repositorio |
| **U²-Net** | Recorte de objeto principal, muy buena calidad | Revisar código y pesos | Pendiente |
| **MODNet** | Matting de retrato, buen pelo | Revisar | Pendiente |
| **SAM / MobileSAM** | Máscara a partir de clics | Revisar | Pendiente |
| **ONNX Runtime** (crate `ort` en Rust) | Ejecutar los modelos en Windows (DirectML para GPU) | Revisar | Pendiente |
| **tract** (Rust puro) | Alternativa a ONNX Runtime, sin dependencias nativas | Revisar | Pendiente |

**Descartes**
- **RobustVideoMatting**: GPL-3.0, incompatible con un proyecto MIT que se distribuya cerrado.
- **rembg con `bria-rmbg`**: requiere pago para uso comercial.
- **BodyPix / TensorFlow.js**: es JavaScript. En la versión nativa no aporta nada frente a ONNX.

**Recomendación**
1. Recorte automático: MediaPipe Selfie Segmentation (si su licencia y su conversión a ONNX se confirman), con U²-Net como alternativa.
2. Pincel inteligente: MobileSAM, si su licencia se confirma.
3. Todo lo demás (chroma, despill, trazo, pincel, lazo, refinado): código propio en WGSL, sin dependencias.

---

## 5. Plan por fases

1. **Chroma key y despill en GPU.** Validar contra las capturas de la versión web con las mismas imágenes.
2. **Máscara pintada.** Diseñar el almacenamiento, implementar pincel, borrador y lazo.
3. **Trazo sólido y de puntos** con campo de distancia.
4. **Segmentación automática** con un solo modelo y a baja resolución.
5. **Pincel inteligente** (SAM o MobileSAM), si la licencia lo permite.
6. **Estilos de trazo complejos** (papel, dibujo a mano, viento).

---

## 6. Pruebas y criterios de aceptación

- Chroma key: con un fondo verde uniforme, el resultado debe ser igual en la versión web y en la nativa, con una diferencia máxima de un 2 % de píxeles.
- Despill: en el borde de un objeto sobre verde, ningún píxel debe quedar más verde que el máximo de sus canales rojo y azul.
- Máscara pintada: una pincelada de 50 px sobre un fotograma debe generar exactamente esa forma al reproducir.
- Trazo: el anillo debe tener el grosor configurado (±1 px) en todo el contorno.
- Segmentación: a 256 px debe mantener al menos 24 FPS en el equipo de referencia, con la inferencia fuera del hilo de render.
- Rendimiento: ningún cambio de parámetro debe bloquear la interfaz más de 16 ms.

---

## 7. Decisiones pendientes

1. ¿Qué estilos de trazo son prioritarios?
2. ¿Segmentación automática con MediaPipe o con U²-Net? Depende de confirmar las licencias.
3. ¿El pincel inteligente (SAM) entra en la primera versión o después?
4. ¿Se puede usar la eliminación automática sin conexión? Todos los modelos propuestos son locales.

---

## 8. Fuentes

- BackgroundMattingV2: https://github.com/PeterL1n/BackgroundMattingV2 (MIT, verificado)
- RobustVideoMatting: https://github.com/PeterL1n/RobustVideoMatting (GPL-3.0, verificado)
- rembg: https://github.com/danielgatis/rembg (MIT; modelo por defecto con licencia de pago, verificado)
- Resto de herramientas: pendiente de verificar antes de usarlas.