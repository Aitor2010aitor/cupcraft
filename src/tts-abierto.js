/* Integración con TTS-Abierto (Piper TTS local) para Cupcraft.
   Ejecuta modelos neuronales en local a máxima velocidad y privacidad. */

export const DEFAULT_TTS_VOICES = [
  { id: 'es_ES-davefx-medium', name: 'Davefx · Masculina', description: 'Alta calidad', lang: 'es' },
  { id: 'es_ES-sharvard-medium', name: 'Sharvard · Neutral', description: 'Alta calidad', lang: 'es' },
  { id: 'es_ES-carlfm-x_low', name: 'Carlfm · Femenina', description: 'Ultrarrápida', lang: 'es' },
];

let cachedVoices = null;

export async function getTtsVoices() {
  if (cachedVoices) return cachedVoices;
  try {
    const res = await fetch('/api/tts/voices');
    if (res.ok) {
      const data = await res.json();
      if (Array.isArray(data.voices) && data.voices.length > 0) {
        cachedVoices = data.voices;
        return cachedVoices;
      }
    }
  } catch (err) {
    console.warn('No se pudieron obtener voces de TTS-Abierto del servidor:', err);
  }
  return DEFAULT_TTS_VOICES;
}

export async function generateTtsAbiertoAudio({ text, voice = 'es_ES-davefx-medium', speed = 1, onProgress = null } = {}) {
  const content = String(text || '').trim();
  if (!content) throw new Error('Escribe el texto que quieres convertir en voz');

  onProgress?.('Sintetizando voz neuronal con TTS Abierto…', 30);

  const res = await fetch('/api/tts/generate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      text: content,
      voice,
      speed: Math.max(0.5, Math.min(2.5, Number(speed) || 1)),
    }),
  });

  if (!res.ok) {
    let msg = 'Error generando la voz';
    try {
      const errJson = await res.json();
      if (errJson?.error) msg = errJson.error;
    } catch {
      // Ignorar fallo de parseo
    }
    throw new Error(msg);
  }

  onProgress?.('Procesando audio generado…', 85);
  const blob = await res.blob();
  if (!blob || blob.size === 0) {
    throw new Error('El motor de voz no devolvió datos de audio');
  }

  // Obtener duración exacta con AudioContext
  let seconds = 2;
  try {
    const arrayBuffer = await blob.arrayBuffer();
    const audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    const decoded = await audioCtx.decodeAudioData(arrayBuffer.slice(0));
    seconds = decoded.duration;
    void audioCtx.close();
  } catch (err) {
    console.warn('No se pudo calcular la duración exacta del audio:', err);
    seconds = Math.max(0.5, blob.size / 44100);
  }

  return {
    blob,
    seconds,
    voice,
  };
}
