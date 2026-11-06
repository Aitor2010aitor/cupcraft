// Prepara las portadas de stickers de CapCut para usarlas como capa:
// sus miniaturas traen un fondo plano oscuro, así que se recorta por color
// desde las esquinas y se devuelve un PNG con transparencia.

function loadImage(src) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('no se pudo cargar ' + src));
    img.src = src;
  });
}

function isNear(data, index, color, tolerance) {
  return Math.abs(data[index] - color[0]) <= tolerance
    && Math.abs(data[index + 1] - color[1]) <= tolerance
    && Math.abs(data[index + 2] - color[2]) <= tolerance;
}

export async function prepareStickerSrc(src) {
  try {
    const img = await loadImage(src);
    const width = img.naturalWidth || img.width;
    const height = img.naturalHeight || img.height;
    if (!width || !height) return src;

    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    ctx.drawImage(img, 0, 0);
    const frame = ctx.getImageData(0, 0, width, height);
    const data = frame.data;

    // Color de las cuatro esquinas: si coinciden, es un fondo plano.
    const corners = [
      [0, 0],
      [width - 1, 0],
      [0, height - 1],
      [width - 1, height - 1],
    ].map(([x, y]) => {
      const i = (y * width + x) * 4;
      return [data[i], data[i + 1], data[i + 2]];
    });
    const base = corners[0];
    const same = corners.every((c) => Math.abs(c[0] - base[0]) <= 12
      && Math.abs(c[1] - base[1]) <= 12
      && Math.abs(c[2] - base[2]) <= 12);
    if (!same) return src;

    const tolerance = 26;
    let removed = 0;
    for (let i = 0; i < data.length; i += 4) {
      if (isNear(data, i, base, tolerance)) {
        data[i + 3] = 0;
        removed += 1;
      }
    }
    const total = data.length / 4;
    // Si se comería casi toda la imagen, mejor dejarla como está.
    if (removed > total * 0.95) return src;

    ctx.putImageData(frame, 0, 0);
    return canvas.toDataURL('image/png');
  } catch (error) {
    console.warn('[cupcraft] sticker sin recorte de fondo:', error.message);
    return src;
  }
}
