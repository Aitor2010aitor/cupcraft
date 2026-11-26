// Almacén del registro de la app. Sin dependencias para poder usarlo desde
// cualquier módulo (incluido utils.js) sin importaciones circulares.

const KEY = 'cupcraft.log.v1';
const MAX_EVENTS = 600;
let events = [];

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
  events.push({ at: Date.now(), kind, text });
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
