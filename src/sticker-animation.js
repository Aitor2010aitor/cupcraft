// Decodifica stickers animados (WebP/GIF) a fotogramas y los sincroniza con la
// línea de tiempo. Sin esto el canvas dibuja siempre el primer fotograma, aunque
// la miniatura del panel sí se mueva.

const MAX_CACHE = 32;
const MAX_FRAMES = 120;
const MAX_SIZE = 640;

const cache = new Map();

function closeEntry(entry) {
  for (const frame of entry.frames || []) {
    try { frame.close?.(); } catch { /* ya liberado */ }
  }
  entry.frames = [];
}

function trimCache() {
  while (cache.size > MAX_CACHE) {
    let victim = null;
    for (const entry of cache) {
      if (entry[1].status !== 'loading') { victim = entry; break; }
    }
    if (!victim) break;
    cache.delete(victim[0]);
    closeEntry(victim[1]);
  }
}

function mimeFromSrc(src) {
  const match = /^data:(image\/[a-z0-9.+-]+)[;,]/i.exec(src);
  return match ? match[1].toLowerCase() : '';
}

async function fetchImageBytes(url) {
  const response = await fetch(url, { cache: 'force-cache' });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const type = (response.headers.get('content-type') || '').split(';')[0].trim();
  return { data: await response.arrayBuffer(), type };
}

/** Descarga `src` y, si falla, la URL remota de respaldo (a través del proxy). */
async function fetchImageBytesWithFallback(src, fallback) {
  try {
    return { ...(await fetchImageBytes(src)), usedFallback: false };
  } catch (error) {
    if (!fallback || fallback === src) throw error;
    return { ...(await fetchImageBytes(`/api/img?u=${encodeURIComponent(fallback)}`)), usedFallback: true };
  }
}

async function sourceIsAnimated(src) {
  try {
    const { data, type: rawType } = await fetchImageBytes(src);
    const type = (rawType || mimeFromSrc(src)).toLowerCase();
    if (!type.startsWith('image/') || typeof globalThis.ImageDecoder !== 'function') {
      return /gif|webp/i.test(type);
    }
    if (!(await globalThis.ImageDecoder.isTypeSupported(type))) return false;
    const decoder = new globalThis.ImageDecoder({ data, type });
    await decoder.tracks.ready;
    try { await decoder.completed; } catch { /* algunos recursos no completan */ }
    const count = decoder.tracks.selectedTrack?.frameCount || 1;
    decoder.close();
    return count > 1;
  } catch {
    return /\.(gif|awebp)(\?|$)/i.test(src);
  }
}

/**
 * true si la imagen tiene más de un fotograma. Se comprueban tanto el archivo
 * local como la URL remota de respaldo: las miniaturas locales pueden ser
 * estáticas aunque el sticker original de CapCut sí esté animado.
 */
export async function stickerIsAnimated(src, fallback = null) {
  if (await sourceIsAnimated(src)) return true;
  if (fallback && fallback !== src) return sourceIsAnimated(fallback);
  return false;
}

async function decodeFrames(entry, src, fallback) {
  const { data, type: rawType, usedFallback } = await fetchImageBytesWithFallback(src, fallback);
  const type = (rawType || mimeFromSrc(src)).toLowerCase();
  if (!type.startsWith('image/') || typeof globalThis.ImageDecoder !== 'function') {
    throw new Error('sin decodificador de imágenes');
  }
  if (!(await globalThis.ImageDecoder.isTypeSupported(type))) {
    throw new Error(`tipo no soportado: ${type}`);
  }
  const decoder = new globalThis.ImageDecoder({ data, type });
  await decoder.tracks.ready;
  try { await decoder.completed; } catch { /* seguimos con los fotogramas disponibles */ }
  const track = decoder.tracks.selectedTrack;
  const count = Math.max(1, track?.frameCount || 1);
  const step = Math.max(1, Math.ceil(count / MAX_FRAMES));
  const frames = [];
  const durations = [];
  let total = 0;

  try {
    for (let index = 0; index < count; index += step) {
      const { image } = await decoder.decode({ frameIndex: index });
      const width = image.displayWidth || image.codedWidth || 0;
      const height = image.displayHeight || image.codedHeight || 0;
      const scale = width && height ? Math.min(1, MAX_SIZE / Math.max(width, height)) : 1;
      const resized = scale < 1
        ? await createImageBitmap(image, {
          resizeWidth: Math.max(1, Math.round(width * scale)),
          resizeHeight: Math.max(1, Math.round(height * scale)),
          resizeQuality: 'high',
        })
        : await createImageBitmap(image);
      const duration = Math.max(0.01, (Number(image.duration) || 0) / 1_000_000) * step;
      image.close();
      frames.push(resized);
      durations.push(duration);
      total += duration;
    }
  } catch (error) {
    for (const frame of frames) {
      try { frame.close?.(); } catch { /* ya liberado */ }
    }
    throw error;
  } finally {
    decoder.close();
  }

  if (!frames.length) throw new Error('sin fotogramas');
  closeEntry(entry);
  entry.frames = frames;
  entry.durations = durations;
  entry.total = total;
  entry.width = frames[0].width;
  entry.height = frames[0].height;
  entry.animated = frames.length > 1;
  entry.status = 'ready';
  return usedFallback;
}

/**
 * Devuelve la entrada de animación de un sticker (se carga en segundo plano).
 * `status` pasa por loading → ready | failed.
 *
 * Si el archivo local resulta estático pero hay una URL remota de respaldo, se
 * vuelve a intentar con ella: así los stickers animados de CapCut siguen
 * animándose aunque su miniatura local sea un WebP de un solo fotograma.
 */
export function stickerAnimation(src, fallback = null) {
  if (!src && !fallback) return null;
  const key = `${src || ''}|${fallback || ''}`;
  const existing = cache.get(key);
  if (existing) {
    cache.delete(key);
    cache.set(key, existing);
    return existing;
  }
  const entry = {
    status: 'loading',
    frames: [],
    durations: [],
    total: 0,
    width: 0,
    height: 0,
    animated: false,
  };
  cache.set(key, entry);
  trimCache();

  const load = async () => {
    const primary = src || fallback;
    const usedFallback = await decodeFrames(entry, primary, src ? fallback : null);
    // Solo merece la pena reintentar si el primer intento usó el archivo local
    // (posiblemente una miniatura estática) en lugar del respaldo remoto.
    if (!entry.animated && !usedFallback && src && fallback && fallback !== src) {
      try {
        await decodeFrames(entry, fallback, null);
      } catch {
        // El respaldo no sirve: nos quedamos con el fotograma local.
      }
    }
    window.dispatchEvent(new CustomEvent('opencup:glfx'));
  };

  load().catch((error) => {
    entry.status = 'failed';
    entry.error = error;
  });
  return entry;
}

/** Fotograma que toca para un tiempo local del clip (en segundos). */
export function stickerAnimationFrame(entry, seconds) {
  if (!entry || entry.status !== 'ready' || !entry.frames.length) return null;
  if (!entry.animated || !(entry.total > 0)) return entry.frames[0];
  let time = Number(seconds) || 0;
  time %= entry.total;
  if (time < 0) time += entry.total;
  let accumulated = 0;
  for (let index = 0; index < entry.frames.length; index += 1) {
    accumulated += entry.durations[index];
    if (time < accumulated) return entry.frames[index];
  }
  return entry.frames[entry.frames.length - 1];
}
