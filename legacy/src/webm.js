/**
 * MediaRecorder produce WebM en modo «streaming»: el elemento Duration no se
 * escribe y muchos reproductores muestran una duración desconocida. Este
 * parche inserta el elemento Duration (EBML 0x4489) dentro de Info y ajusta el
 * tamaño de Info, sin volver a codificar el video.
 */
const ID_DURATION = 0x4489;
const ID_INFO = 0x1549a966;
const ID_TIMECODE_SCALE = 0x2ad7b1;

function readId(bytes, offset) {
  if (offset >= bytes.length) return null;
  const first = bytes[offset];
  let length = 1;
  while (length <= 4 && !(first & (0x80 >> (length - 1)))) length += 1;
  if (length > 4 || offset + length > bytes.length) return null;
  let value = 0;
  for (let i = 0; i < length; i += 1) value = value * 256 + bytes[offset + i];
  return { value, length };
}

function readSize(bytes, offset) {
  if (offset >= bytes.length) return null;
  const first = bytes[offset];
  if (first === 0) return null;
  let length = 1;
  while (length <= 8 && !(first & (0x80 >> (length - 1)))) length += 1;
  if (length > 8 || offset + length > bytes.length) return null;
  let value = first & (0xff >> length);
  for (let i = 1; i < length; i += 1) value = value * 256 + bytes[offset + i];
  const unknown = value === 2 ** (7 * length) - 1;
  return { value, length, unknown };
}

function encodeSize(value, length) {
  let size = length;
  while (size < 8 && value > 2 ** (7 * size) - 2) size += 1;
  if (size > 8) return null;
  const bytes = new Uint8Array(size);
  let remaining = value;
  for (let i = size - 1; i >= 0; i -= 1) {
    bytes[i] = remaining % 256;
    remaining = Math.floor(remaining / 256);
  }
  bytes[0] |= 0x80 >> (size - 1);
  return bytes;
}

function readUint(bytes, start, end) {
  let value = 0;
  for (let i = start; i < end; i += 1) value = value * 256 + bytes[i];
  return value;
}

function findInfo(bytes) {
  let offset = 0;
  const header = readId(bytes, offset);
  if (!header) return null;
  offset += header.length;
  const headerSize = readSize(bytes, offset);
  if (!headerSize) return null;
  offset += headerSize.length + headerSize.value;

  const segment = readId(bytes, offset);
  if (!segment) return null;
  offset += segment.length;
  const segmentSize = readSize(bytes, offset);
  if (!segmentSize) return null;
  const segmentDataStart = offset + segmentSize.length;
  const segmentDataEnd = segmentSize.unknown ? bytes.length : segmentDataStart + segmentSize.value;
  offset = segmentDataStart;

  while (offset < segmentDataEnd) {
    const id = readId(bytes, offset);
    if (!id) return null;
    const size = readSize(bytes, offset + id.length);
    if (!size) return null;
    const dataStart = offset + id.length + size.length;
    const dataEnd = size.unknown ? segmentDataEnd : dataStart + size.value;
    if (id.value === ID_INFO) {
      return {
        sizeOffset: offset + id.length,
        sizeLength: size.length,
        dataStart,
        dataEnd,
        nextOffset: dataEnd,
      };
    }
    if (id.value === 0x1f43b675) break;
    offset = dataEnd;
  }
  return null;
}

export function fixWebmDuration(blob, durationSeconds) {
  if (!blob || !(blob instanceof Blob) || !(durationSeconds > 0)) return blob;
  return blob.arrayBuffer().then((buffer) => {
    const bytes = new Uint8Array(buffer);
    const info = findInfo(bytes);
    if (!info) return blob;

    let scale = 1_000_000;
    let existing = null;
    let cursor = info.dataStart;
    while (cursor < info.dataEnd) {
      const id = readId(bytes, cursor);
      if (!id) break;
      const size = readSize(bytes, cursor + id.length);
      if (!size) break;
      const dataStart = cursor + id.length + size.length;
      const dataEnd = dataStart + size.value;
      if (id.value === ID_TIMECODE_SCALE) scale = readUint(bytes, dataStart, dataEnd) || scale;
      if (id.value === ID_DURATION) existing = { dataStart, dataEnd };
      cursor = dataEnd;
    }

    const ticks = Math.max(1, durationSeconds * 1e9 / scale);

    if (existing) {
      const out = bytes.slice();
      new DataView(out.buffer).setFloat64(existing.dataStart, ticks);
      return new Blob([out], { type: blob.type || 'video/webm' });
    }

    const duration = new Uint8Array(11);
    duration[0] = 0x44;
    duration[1] = 0x89;
    duration[2] = 0x88;
    new DataView(duration.buffer).setFloat64(3, ticks);

    const newInfoSize = info.dataEnd - info.dataStart + duration.length;
    const encoded = encodeSize(newInfoSize, info.sizeLength);
    if (!encoded) return blob;

    const out = new Uint8Array(
      bytes.length + duration.length + (encoded.length - info.sizeLength),
    );
    let at = 0;
    out.set(bytes.subarray(0, info.sizeOffset), at);
    at += info.sizeOffset;
    out.set(encoded, at);
    at += encoded.length;
    out.set(bytes.subarray(info.dataStart, info.dataEnd), at);
    at += info.dataEnd - info.dataStart;
    out.set(duration, at);
    at += duration.length;
    out.set(bytes.subarray(info.nextOffset), at);
    return new Blob([out], { type: blob.type || 'video/webm' });
  }).catch(() => blob);
}
