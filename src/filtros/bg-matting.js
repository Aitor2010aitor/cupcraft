/* Eliminación de fondo con BackgroundMattingV2 (MIT, PeterL1n).
   Modelo ONNX local ejecutado con ONNX Runtime Web dentro de WebView2.
   Requiere una imagen de fondo (background plate) sin el sujeto. */

const NET_W = 512;
const NET_H = 288; // El modelo HD exportado necesita >= ~283px de lado.

let ort = null;
let session = null;
let initPromise = null;

export async function initMatting() {
  if (session) return session;
  if (initPromise) return initPromise;
  initPromise = (async () => {
    const vendor = new URL('../vendor/ort/', import.meta.url).href;
    const model = new URL('../models/bgm_hd.onnx', import.meta.url).href;
    ort = await import('../vendor/ort/ort.bundle.min.mjs');
    ort.env.wasm.wasmPaths = vendor;
    ort.env.wasm.numThreads = 1;
    session = await ort.InferenceSession.create(model, { executionProviders: ['wasm'] });
    return session;
  })().catch((error) => {
    initPromise = null;
    throw error;
  });
  return initPromise;
}

export function mattingReady() {
  return Boolean(session);
}

function toCHW(imageData) {
  const out = new Float32Array(3 * NET_W * NET_H);
  const data = imageData.data;
  const plane = NET_W * NET_H;
  for (let p = 0; p < plane; p += 1) {
    const offset = p * 4;
    out[p] = data[offset] / 255;
    out[plane + p] = data[offset + 1] / 255;
    out[plane * 2 + p] = data[offset + 2] / 255;
  }
  return out;
}

const srcCanvas = document.createElement('canvas');
srcCanvas.width = NET_W;
srcCanvas.height = NET_H;
const bgCanvas = document.createElement('canvas');
bgCanvas.width = NET_W;
bgCanvas.height = NET_H;
const srcCtx = srcCanvas.getContext('2d', { willReadFrequently: true });
const bgCtx = bgCanvas.getContext('2d', { willReadFrequently: true });

/* Postproceso del alpha inspirado en el pipeline observado en CapCut:
   - limpieza de borde: blur pequeño + curva suave, como su pase DilateErode;
   - estabilidad temporal: mezcla con el fotograma vecino, como aproximación
     local a su modelo recurrente GRU.
   Es código propio: no usa modelos, shaders ni parámetros de CapCut. */

const temporalFrames = new Map();
const TEMPORAL_HISTORY = 10;
const TEMPORAL_MAX_DELTA = 0.22; // segundos

function clampAlpha(value) {
  return value < 0 ? 0 : value > 1 ? 1 : value;
}

function cleanAlpha(alpha, strength) {
  if (!(strength > 0.001)) return alpha;
  const width = NET_W;
  const height = NET_H;
  const spread = Math.min(0.5, strength * 0.5);
  const contrast = strength * 0.35;
  const horizontal = new Float32Array(alpha.length);
  const out = new Float32Array(alpha.length);
  for (let y = 0; y < height; y += 1) {
    const row = y * width;
    for (let x = 0; x < width; x += 1) {
      const i = row + x;
      const left = alpha[row + (x > 0 ? x - 1 : 0)];
      const right = alpha[row + (x < width - 1 ? x + 1 : width - 1)];
      horizontal[i] = alpha[i] * (1 - spread) + (left + right) * 0.5 * spread;
    }
  }
  for (let y = 0; y < height; y += 1) {
    const row = y * width;
    for (let x = 0; x < width; x += 1) {
      const i = row + x;
      const up = horizontal[(y > 0 ? y - 1 : 0) * width + x];
      const down = horizontal[(y < height - 1 ? y + 1 : height - 1) * width + x];
      const smooth = horizontal[i] * (1 - spread) + (up + down) * 0.5 * spread;
      const shaped = smooth * smooth * (3 - 2 * smooth);
      out[i] = clampAlpha(smooth * (1 - contrast) + shaped * contrast);
    }
  }
  return out;
}

function blendTemporal(alpha, key, time, strength) {
  if (!key || !(strength > 0.001) || !Number.isFinite(time)) return alpha;
  let frames = temporalFrames.get(key);
  if (!frames) {
    frames = [];
    temporalFrames.set(key, frames);
  }
  let nearest = null;
  let best = Infinity;
  for (const frame of frames) {
    const delta = Math.abs(frame.time - time);
    if (delta < best) {
      best = delta;
      nearest = frame;
    }
  }
  let out = alpha;
  if (nearest && best <= TEMPORAL_MAX_DELTA) {
    const weight = strength * (1 - best / TEMPORAL_MAX_DELTA) * 0.45;
    if (weight > 0.001) {
      out = new Float32Array(alpha.length);
      for (let i = 0; i < alpha.length; i += 1) {
        out[i] = alpha[i] * (1 - weight) + nearest.alpha[i] * weight;
      }
    }
  }
  // Se guarda el alpha recién inferido, no el mezclado: así el resultado no
  // depende del orden en que se calculen los fotogramas al precachear.
  frames.push({ time, alpha });
  if (frames.length > TEMPORAL_HISTORY) frames.shift();
  return out;
}

/* Pinta `source` recortado a NET_W x NET_H (letterbox no: el modelo espera
   la imagen completa redimensionada; se conserva la relación al aplicar la
   máscara sobre la resolución original). */
export async function matteSource(source, background, output = null, options = {}) {
  const width = source.videoWidth || source.naturalWidth || source.width;
  const height = source.videoHeight || source.naturalHeight || source.height;
  if (!width || !height || !background) return null;

  await initMatting();
  srcCtx.drawImage(source, 0, 0, NET_W, NET_H);
  bgCtx.drawImage(background, 0, 0, NET_W, NET_H);
  const srcTensor = new ort.Tensor('float32', toCHW(srcCtx.getImageData(0, 0, NET_W, NET_H)), [1, 3, NET_H, NET_W]);
  const bgTensor = new ort.Tensor('float32', toCHW(bgCtx.getImageData(0, 0, NET_W, NET_H)), [1, 3, NET_H, NET_W]);
  const result = await session.run({ src: srcTensor, bgr: bgTensor });
  const cleaned = cleanAlpha(result.pha.data, Math.min(1, Math.max(0, Number(options.edge) || 0)));
  const alpha = blendTemporal(
    cleaned,
    options.temporalKey,
    options.time,
    Math.min(1, Math.max(0, Number(options.temporal) || 0)),
  );

  const canvas = output || document.createElement('canvas');
  // La alpha del modelo ya viene a 512x288; limitar el canvas cacheado evita
  // cientos de MB al precachear todos los fotogramas de una exportación.
  const scale = Math.min(1, 960 / Math.max(width, height));
  const outW = Math.max(2, Math.round(width * scale));
  const outH = Math.max(2, Math.round(height * scale));
  canvas.width = outW;
  canvas.height = outH;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.clearRect(0, 0, outW, outH);
  ctx.drawImage(source, 0, 0, outW, outH);
  const frame = ctx.getImageData(0, 0, outW, outH);
  const data = frame.data;
  for (let y = 0; y < outH; y += 1) {
    const sy = Math.min(NET_H - 1, Math.floor((y * NET_H) / outH));
    for (let x = 0; x < outW; x += 1) {
      const sx = Math.min(NET_W - 1, Math.floor((x * NET_W) / outW));
      const value = alpha[sy * NET_W + sx];
      data[(y * outW + x) * 4 + 3] = Math.round(Math.max(0, Math.min(1, value)) * 255);
    }
  }
  ctx.putImageData(frame, 0, 0);
  return canvas;
}
