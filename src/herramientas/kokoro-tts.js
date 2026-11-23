/* Voz neuronal Kokoro (kokoro-js, Apache-2.0) ejecutada en el navegador.
   - El modelo onnx-community/Kokoro-82M-v1.0-ONNX (q8, ~92 MB) se descarga en la
     primera generación y queda en la caché del navegador.
   - En inglés se usa el propio `generate()` de kokoro-js.
   - En español (ef_dora, em_alex, em_santa) ese método lanza error porque las
     voces no están en su tabla, así que se construyen los fonemas con el
     conversor grafema→fonema propio de este módulo y se llama a
     `generate_from_ids()` (español castellano, con θ para c/z). */

const KOKORO_CDN = 'https://cdn.jsdelivr.net/npm/kokoro-js@1.2.1/+esm';
const KOKORO_MODEL = 'onnx-community/Kokoro-82M-v1.0-ONNX';

export const KOKORO_ES_VOICES = ['ef_dora', 'em_alex', 'em_santa'];
export const KOKORO_EN_VOICES = ['af_heart'];

let ttsInstance = null;
let ttsPromise = null;

export function isKokoroSpanishVoice(voice) {
  return KOKORO_ES_VOICES.includes(String(voice || ''));
}

export async function loadKokoro(onProgress) {
  if (ttsInstance) return ttsInstance;
  if (!ttsPromise) {
    ttsPromise = (async () => {
      onProgress?.('Descargando la biblioteca de voz…', 0);
      const kokoro = await import(KOKORO_CDN);
      onProgress?.('Descargando el modelo Kokoro (~92 MB)…', 0);
      ttsInstance = await kokoro.KokoroTTS.from_pretrained(KOKORO_MODEL, {
        dtype: 'q8',
        device: 'wasm',
        progress_callback: (event) => {
          if (event?.status === 'progress' && Number.isFinite(event.progress)) {
            const file = event.file ? String(event.file).split('/').pop() : 'modelo';
            onProgress?.(`Descargando ${file}… ${Math.round(event.progress)}%`, event.progress);
          } else if (event?.status === 'initiate' && event.file) {
            onProgress?.(`Descargando ${String(event.file).split('/').pop()}…`, 0);
          }
        },
      });
      return ttsInstance;
    })();
    ttsPromise.catch(() => {
      ttsPromise = null;
    });
  }
  return ttsPromise;
}

/* ------------------------- Español: texto → fonemas IPA ------------------------- */

const ACCENTS = { á: 'a', é: 'e', í: 'i', ó: 'o', ú: 'u' };

const NUMBER_UNITS = ['cero', 'uno', 'dos', 'tres', 'cuatro', 'cinco', 'seis', 'siete', 'ocho', 'nueve'];
const NUMBER_TEENS = ['diez', 'once', 'doce', 'trece', 'catorce', 'quince', 'dieciséis', 'diecisiete', 'dieciocho', 'diecinueve'];
const NUMBER_TENS = ['', '', 'veinte', 'treinta', 'cuarenta', 'cincuenta', 'sesenta', 'setenta', 'ochenta', 'noventa'];
const NUMBER_HUNDREDS = ['', 'cien', 'doscientos', 'trescientos', 'cuatrocientos', 'quinientos', 'seiscientos', 'setecientos', 'ochocientos', 'novecientos'];

function numberToWords(value) {
  const n = Math.abs(Math.trunc(Number(value) || 0));
  if (n < 10) return NUMBER_UNITS[n];
  if (n < 20) return NUMBER_TEENS[n - 10];
  if (n < 100) {
    const tens = Math.floor(n / 10);
    const unit = n % 10;
    return unit ? `${NUMBER_TENS[tens]} y ${NUMBER_UNITS[unit]}` : NUMBER_TENS[tens];
  }
  if (n < 1000) {
    const hundred = Math.floor(n / 100);
    const rest = n % 100;
    if (hundred === 1) return rest ? `ciento ${numberToWords(rest)}` : 'cien';
    return rest ? `${NUMBER_HUNDREDS[hundred]} ${numberToWords(rest)}` : NUMBER_HUNDREDS[hundred];
  }
  if (n < 1000000) {
    const thousands = Math.floor(n / 1000);
    const rest = n % 1000;
    const head = thousands === 1 ? 'mil' : `${numberToWords(thousands)} mil`;
    return rest ? `${head} ${numberToWords(rest)}` : head;
  }
  return String(n).split('').map((digit) => NUMBER_UNITS[Number(digit)]).join(' ');
}

export function expandSpanishNumbers(text) {
  return String(text || '')
    .replace(/(\d+)\.(\d+)/g, (_, whole, decimal) => (
      `${numberToWords(whole)} punto ${decimal.split('').map((digit) => NUMBER_UNITS[Number(digit)]).join(' ')}`
    ))
    .replace(/\d+/g, (match) => numberToWords(match));
}

/* Convierte una palabra española a fonemas IPA (reglas de castellano estándar). */
function spanishWordToIpa(raw) {
  const lower = String(raw).toLowerCase();
  if (!/[a-z]/.test(lower)) return '';

  let accentIndex = -1;
  for (let i = 0; i < lower.length; i += 1) {
    if (Object.prototype.hasOwnProperty.call(ACCENTS, lower[i])) {
      accentIndex = i;
      break;
    }
  }
  const word = lower.replace(/[áéíóú]/g, (letter) => ACCENTS[letter]).replace(/ü/g, 'u');

  const out = [];
  const vowelOut = []; // { src, pos }
  const pushVowel = (src, symbol = word[src]) => {
    vowelOut.push({ src, pos: out.length });
    out.push(symbol);
  };

  for (let i = 0; i < word.length; i += 1) {
    const ch = word[i];
    const next = word[i + 1];
    const prev = word[i - 1];

    /* Digrafos: se resuelven antes que las letras sueltas. */
    if (ch === 'c' && next === 'h') { out.push('tʃ'); i += 1; continue; }
    if (ch === 'l' && next === 'l') { out.push('ʝ'); i += 1; continue; }
    if (ch === 'q' && next === 'u' && word[i + 2] && 'ei'.includes(word[i + 2])) { out.push('k'); i += 2; continue; }
    if (ch === 'g' && next === 'u' && word[i + 2] && 'ei'.includes(word[i + 2])) { out.push('g'); i += 2; continue; }

    if (ch === 'c' && next && 'ei'.includes(next)) { out.push('θ'); continue; }
    if (ch === 'z') { out.push('θ'); continue; }
    if (ch === 'c') { out.push('k'); continue; }
    if (ch === 'x') { out.push(next && 'ei'.includes(next) ? 'x' : 'ks'); continue; }
    if (ch === 'g') { out.push(next && 'ei'.includes(next) ? 'x' : 'g'); continue; }
    if (ch === 'h') continue;
    if (ch === 'j') { out.push('x'); continue; }
    if (ch === 'w') { out.push('w'); continue; }
    if (ch === 'k') { out.push('k'); continue; }
    if (ch === 'n') { out.push('n'); continue; }
    if (ch === 'm') { out.push('m'); continue; }
    if (ch === 'f') { out.push('f'); continue; }
    if (ch === 'l') { out.push('l'); continue; }
    if (ch === 'p') { out.push('p'); continue; }
    if (ch === 't') { out.push('t'); continue; }
    if (ch === 's') { out.push('s'); continue; }
    if (ch === 'd') { out.push('d'); continue; }
    if (ch === 'b') { out.push('b'); continue; }
    if (ch === 'r') { out.push(i === 0 || 'lns'.includes(prev) ? 'r' : 'ɾ'); continue; }
    if (ch === 'ñ') { out.push('ɲ'); continue; }
    if ('aeiou'.includes(ch)) { pushVowel(i); continue; }
    if (ch === 'y') {
      if (i === 0 || (next && 'aeiou'.includes(next))) out.push('ʝ');
      else pushVowel(i, 'i');
      continue;
    }
  }

  if (!vowelOut.length) return out.join('');

  /* Grupos de vocales contiguas ≈ sílabas con vocal. */
  const groups = [];
  for (const entry of vowelOut) {
    const last = groups[groups.length - 1];
    if (last && entry.src === last[last.length - 1].src + 1) last.push(entry);
    else groups.push([entry]);
  }

  let target = null;
  if (accentIndex >= 0) {
    target = vowelOut.find((entry) => entry.src === accentIndex) || null;
  }
  if (!target) {
    const endsLoose = /[aeiouns]$/.test(word);
    const index = endsLoose ? Math.max(0, groups.length - 2) : groups.length - 1;
    const group = groups[Math.max(0, index)] || groups[0];
    target = group[group.length - 1];
  }
  if (target) {
    const previous = vowelOut.filter((entry) => entry.pos < target.pos).pop();
    out.splice(previous ? previous.pos + 1 : 0, 0, 'ˈ');
  }
  return out.join('');
}

/* Texto completo → cadena de fonemas IPA separada por espacios. */
export function spanishToIpa(text) {
  const expanded = expandSpanishNumbers(String(text || ''));
  const pieces = [];
  for (const match of expanded.toLowerCase().matchAll(/[a-záéíóúüñ]+|[.,;:!?…]/g)) {
    const piece = match[0];
    pieces.push(piece.length === 1 && !/[a-z]/.test(piece) ? piece : spanishWordToIpa(piece));
  }
  return pieces.filter(Boolean).join(' ');
}

/* ------------------------------- Codificación WAV ------------------------------- */

export function encodeWav(samples, sampleRate) {
  const length = samples.length;
  const buffer = new ArrayBuffer(44 + length * 2);
  const view = new DataView(buffer);
  const writeText = (offset, value) => {
    for (let i = 0; i < value.length; i += 1) view.setUint8(offset + i, value.charCodeAt(i));
  };
  writeText(0, 'RIFF');
  view.setUint32(4, 36 + length * 2, true);
  writeText(8, 'WAVE');
  writeText(12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  writeText(36, 'data');
  view.setUint32(40, length * 2, true);
  let offset = 44;
  for (let i = 0; i < length; i += 1) {
    const value = Math.max(-1, Math.min(1, samples[i]));
    view.setInt16(offset, value < 0 ? value * 0x8000 : value * 0x7fff, true);
    offset += 2;
  }
  return new Blob([buffer], { type: 'audio/wav' });
}

/* --------------------------- Generación de la voz --------------------------- */

export async function generateKokoroAudio({ text, voice = 'ef_dora', speed = 1, onProgress = null } = {}) {
  const content = String(text || '').trim();
  if (!content) throw new Error('Escribe el texto que quieres convertir en voz');
  const tts = await loadKokoro(onProgress);
  onProgress?.('Convirtiendo el texto en fonemas…');
  let audio = null;
  if (isKokoroSpanishVoice(voice)) {
    const phonemes = spanishToIpa(content);
    if (!phonemes) throw new Error('No se pudieron convertir el texto a fonemas');
    const { input_ids } = tts.tokenizer(phonemes, { truncation: true });
    onProgress?.('Generando el audio con la voz…');
    audio = await tts.generate_from_ids(input_ids, { voice, speed });
  } else {
    onProgress?.('Generando el audio con la voz…');
    audio = await tts.generate(content, { voice, speed });
  }
  const samples = audio?.audio || audio?.data || audio?.waveform?.data;
  const sampleRate = audio?.sampling_rate || audio?.samplingRate || 24000;
  if (!samples || !samples.length) throw new Error('La generación no devolvió audio');
  return {
    blob: encodeWav(samples, sampleRate),
    seconds: samples.length / sampleRate,
    sampleRate,
    voice,
  };
}
