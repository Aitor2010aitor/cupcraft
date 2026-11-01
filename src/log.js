// Registro de la app: errores, avisos y eventos relevantes, visibles desde el editor.
import { openModal } from './modal.js';

const MAX_EVENTS = 400;
const events = [];

export function logEvent(kind, message) {
  const text = typeof message === 'string' ? message : (() => {
    try { return JSON.stringify(message); } catch { return String(message); }
  })();
  events.push({ at: Date.now(), kind, text });
  if (events.length > MAX_EVENTS) events.shift();
}

export function getLog() {
  return events.slice();
}

export function clearLog() {
  events.length = 0;
}

export function installLogCapture() {
  window.addEventListener('error', (event) => {
    logEvent('error', `${event.message} (${event.filename ?? '?'}:${event.lineno ?? 0})`);
  });
  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason;
    logEvent('promise', reason && reason.message ? reason.message : String(reason));
  });
  const originalError = console.error.bind(console);
  console.error = (...args) => {
    logEvent('console', args.map((a) => (a && a.message) ? a.message : String(a)).join(' '));
    originalError(...args);
  };
  const originalWarn = console.warn.bind(console);
  console.warn = (...args) => {
    logEvent('warn', args.map((a) => (a && a.message) ? a.message : String(a)).join(' '));
    originalWarn(...args);
  };
  logEvent('app', 'Registro iniciado');
}

const fmt = (ms) => new Date(ms).toLocaleTimeString('es-ES', { hour12: false });

export function openLogDialog() {
  const entries = getLog();
  const rows = entries.length
    ? entries.slice(-200).map((entry) => `<div class="log-row log-${entry.kind}"><span class="log-time">${fmt(entry.at)}</span><span class="log-kind">${entry.kind}</span><span class="log-text">${escape(entry.text)}</span></div>`).join('')
    : '<div class="panel-empty">Todavía no hay eventos.</div>';
  openModal({
    title: `Registro (${entries.length})`,
    width: 760,
    body: `<div class="log-list">${rows}</div>`,
    footer: `
      <button class="btn" data-log-copy>Copiar</button>
      <button class="btn" data-log-clear>Vaciar</button>
      <button class="btn btn-primary" data-modal-close>Cerrar</button>`,
    onMount: (modal) => {
      modal.body.querySelector('[data-log-copy]')?.addEventListener('click', async () => {
        const text = getLog().map((e) => `${fmt(e.at)} [${e.kind}] ${e.text}`).join('\n');
        try {
          await navigator.clipboard.writeText(text);
          logEvent('app', 'Registro copiado al portapapeles');
        } catch { /* sin permisos de portapapeles */ }
      });
      modal.body.querySelector('[data-log-clear]')?.addEventListener('click', () => {
        clearLog();
        openLogDialog();
      });
    },
  });
}

function escape(text) {
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
