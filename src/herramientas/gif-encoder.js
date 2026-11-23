// Codificador GIF89a propio, sin dependencias. Convierte fotogramas RGBA en un
// GIF animado con paleta global y bucle infinito (extensión Netscape 2.0).
// Se usa para la pestaña «GIF» del diálogo de exportación.

const WEB_PALETTE = (() => {
  const colors = [];
  for (let r = 0; r < 6; r += 1) {
    for (let g = 0; g < 6; g += 1) {
      for (let b = 0; b < 6; b += 1) {
        colors.push([r * 51, g * 51, b * 51]);
      }
    }
  }
  for (let i = 0; i < 40; i += 1) {
    const v = Math.round((i * 255) / 39);
    colors.push([v, v, v]);
  }
  return colors;
})();

const NEAREST_CACHE = new Map();

function paletteIndex(r, g, b) {
  const key = `${r >> 3},${g >> 3},${b >> 3}`;
  const cached = NEAREST_CACHE.get(key);
  if (cached !== undefined) return cached;
  let best = 0;
  let bestDistance = Infinity;
  for (let i = 0; i < WEB_PALETTE.length; i += 1) {
    const color = WEB_PALETTE[i];
    const dr = r - color[0];
    const dg = g - color[1];
    const db = b - color[2];
    const distance = dr * dr * 3 + dg * dg * 4 + db * db * 2;
    if (distance < bestDistance) {
      bestDistance = distance;
      best = i;
      if (distance === 0) break;
    }
  }
  NEAREST_CACHE.set(key, best);
  return best;
}

function toIndexed(imageData, width, height) {
  const source = imageData.data;
  const out = new Uint8Array(width * height);
  for (let i = 0, p = 0; i < source.length; i += 4, p += 1) {
    const alpha = source[i + 3];
    if (alpha < 128) {
      out[p] = 0;
      continue;
    }
    out[p] = paletteIndex(source[i], source[i + 1], source[i + 2]);
  }
  return out;
}

/** LZW GIF: códigos de 9 a 12 bits, empaquetados little-endian en bloques de 255 bytes. */
function lzwEncode(indexed, minCodeSize) {
  const clear = 1 << minCodeSize;
  const end = clear + 1;
  let codeSize = minCodeSize + 1;
  let nextCode = end + 1;
  let dictionary = new Map();

  const bytes = [];
  let bitBuffer = 0;
  let bitCount = 0;
  const emit = (code) => {
    bitBuffer |= code << bitCount;
    bitCount += codeSize;
    while (bitCount >= 8) {
      bytes.push(bitBuffer & 0xff);
      bitBuffer >>>= 8;
      bitCount -= 8;
    }
  };

  emit(clear);
  if (indexed.length) {
    let prefix = indexed[0];
    for (let i = 1; i < indexed.length; i += 1) {
      const value = indexed[i];
      const key = (prefix << 8) | value;
      const found = dictionary.get(key);
      if (found !== undefined) {
        prefix = found;
        continue;
      }
      emit(prefix);
      if (nextCode < 4096) {
        dictionary.set(key, nextCode);
        nextCode += 1;
        // El decodificador va un código por detrás: el codificador aumenta
        // el tamaño de código justo después de asignar el código 2^n.
        if (nextCode > (1 << codeSize) && codeSize < 12) codeSize += 1;
      } else {
        emit(clear);
        dictionary = new Map();
        codeSize = minCodeSize + 1;
        nextCode = end + 1;
      }
      prefix = value;
    }
    emit(prefix);
  }
  emit(end);
  if (bitCount > 0) bytes.push(bitBuffer & 0xff);

  const chunks = [];
  for (let offset = 0; offset < bytes.length; offset += 255) {
    const chunk = bytes.slice(offset, offset + 255);
    chunks.push(chunk.length, ...chunk);
  }
  chunks.push(0);
  return Uint8Array.from(chunks);
}

function u16(value) {
  return [value & 0xff, (value >> 8) & 0xff];
}

/**
 * @param {Array<{data: Uint8ClampedArray, delay: number}>} frames
 *        delay en centésimas de segundo.
 */
export function encodeGif(frames, width, height) {
  if (!frames.length) throw new Error('No hay fotogramas para el GIF');
  const parts = [];
  const push = (...values) => parts.push(Uint8Array.from(values));

  // Cabecera + descriptor de pantalla (paleta global de 256 colores).
  push(...Array.from('GIF89a', (c) => c.charCodeAt(0)));
  push(...u16(width), ...u16(height), 0xf7, 0, 0);
  for (const color of WEB_PALETTE) push(...color);

  // Bucle infinito.
  push(0x21, 0xff, 0x0b, ...Array.from('NETSCAPE2.0', (c) => c.charCodeAt(0)), 0x03, 0x01, 0x00, 0x00, 0x00);

  for (const frame of frames) {
    const delay = Math.max(2, Math.min(65535, Math.round(frame.delay || 10)));
    // Graphic Control Extension: sin transparencia, disposición 1.
    push(0x21, 0xf9, 0x04, 0x00, ...u16(delay), 0x00, 0x00);
    // Image Descriptor + tamaño mínimo de código LZW antes de los subbloques.
    push(0x2c, ...u16(0), ...u16(0), ...u16(width), ...u16(height), 0x00, 8);
    parts.push(lzwEncode(toIndexed(frame, width, height), 8));
  }
  push(0x3b);

  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return new Blob([result], { type: 'image/gif' });
}
