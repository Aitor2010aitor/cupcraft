// Almacén del registro de la app. Sin dependencias para poder usarlo desde
// cualquier módulo (incluido utils.js) sin importaciones circulares.

const KEY = 'cupcraft.log.v1';
const MAX_EVENTS = 600;
let events = [];

/* Avisos de librerías de terceros (ONNX Runtime, decodificadores, etc.).
   Son ruido para la persona que edita: se conservan, pero como `info`. */
const LIBRARY_NOISE = [
  /onnxruntime/i,
  /removing initializer/i,
  /unable to determine content-length/i,
];

function sanitizeKind(kind, text) {
  return kind === 'warn' && LIBRARY_NOISE.some((pattern) => pattern.test(text)) ? 'info' : kind;
}

function restore() {
  try {
    if (typeof localStorage === 'undefined') return;
    const raw = localStorage.getItem(KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) events = parsed.slice(-MAX_EVENTS);
  } catch {
    events = [];
  }
}

function persist() {
  try {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(KEY, JSON.stringify(events.slice(-MAX_EVENTS)));
  } catch {
    // Cuota llena o almacenamiento bloqueado: el registro sigue vivo en memoria.
  }
}

restore();

export function logEvent(kind, message) {
  const text = typeof message === 'string' ? message : (() => {
    try { return JSON.stringify(message); } catch { return String(message); }
  })();
  const safeKind = sanitizeKind(kind, text);
  const last = events[events.length - 1];
  // Los fallos de miniaturas/red se repiten mucho: los agrupamos en el momento.
  if (last && last.kind === safeKind && last.text === text && Date.now() - last.at < 1500) {
    last.count = (last.count || 1) + 1;
    last.at = Date.now();
  } else {
    events.push({ at: Date.now(), kind: safeKind, text, count: 1 });
  }
  if (events.length > MAX_EVENTS) events.shift();
  persist();
}

export function getLog() {
  return events.slice();
}

export function clearLog() {
  events.length = 0;
  persist();
}
