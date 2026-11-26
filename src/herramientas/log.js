// Registro de la app: errores, avisos y eventos relevantes, visibles desde el editor.
import { openModal } from '../modal.js';
import { downloadBlob, toast } from '../utils.js';
import { logEvent, getLog, clearLog } from './log-core.js';

export { logEvent, getLog, clearLog } from './log-core.js';

/* Registra un fallo de importación con el nombre del archivo/recurso.
   Se usa tanto desde los capturadores globales como desde los diálogos. */
export function logImportError(what, error) {
  const detail = error && error.message ? error.message : (error ? String(error) : 'error desconocido');
  logEvent('import', `No se pudo importar ${what}: ${detail}`);
}

function urlOf(input) {
  if (typeof input === 'string') return input;
  if (input instanceof Request) return input.url;
  try { return String(input?.url || input); } catch { return '?' }
}

export function installLogCapture() {
  /* capture=true también detecta errores de recursos (imágenes, fuentes,
     vídeo, audio, scripts) que no burbujean y antes pasaban desapercibidos. */
  window.addEventListener('error', (event) => {
    const target = event.target;
    if (target && target !== window && typeof target.tagName === 'string') {
      const src = target.src || target.href || target.currentSrc || '(sin ruta)';
      logEvent('import', `No se pudo importar ${target.tagName.toLowerCase()}: ${src}`);
      return;
    }
    const stack = event.error?.stack ? `\n${event.error.stack}` : '';
    logEvent('error', `${event.message || 'Error desconocido'} (${event.filename ?? '?'}:${event.lineno ?? 0})${stack}`);
  }, true);

  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason;
    const detail = reason && reason.stack ? reason.stack : (reason && reason.message ? reason.message : String(reason));
    logEvent('promise', detail);
  });

  const originalError = console.error.bind(console);
  console.error = (...args) => {
    logEvent('console', args.map((a) => (a && a.message) ? (a.stack || a.message) : String(a)).join(' '));
    originalError(...args);
  };
  const originalWarn = console.warn.bind(console);
  console.warn = (...args) => {
    logEvent('warn', args.map((a) => (a && a.message) ? a.message : String(a)).join(' '));
    originalWarn(...args);
  };

  /* Fallos HTTP y de red de fetch (API local de fuentes, proyectos, TTS…). */
  const originalFetch = window.fetch?.bind(window);
  if (originalFetch) {
    window.fetch = async (...args) => {
      const url = urlOf(args[0]);
      try {
        const response = await originalFetch(...args);
        if (!response.ok) logEvent('http', `HTTP ${response.status} al cargar ${url}`);
        return response;
      } catch (error) {
        logEvent('error', `Fallo de red al cargar ${url}: ${error?.message || error}`);
        throw error;
      }
    };
  }

  logEvent('app', 'Registro iniciado');
}

const fmt = (ms) => new Date(ms).toLocaleTimeString('es-ES', { hour12: false });
const ERROR_KINDS = new Set(['error', 'promise', 'console', 'import', 'http']);

export function openLogDialog() {
  const entries = getLog();
  const hasErrors = entries.some((e) => ERROR_KINDS.has(e.kind));

  const rowsFor = (filter = '', onlyErrors = false) => {
    const term = filter.trim().toLowerCase();
    const list = entries.filter((entry) => (
      (!onlyErrors || ERROR_KINDS.has(entry.kind))
      && (!term || `${entry.kind} ${entry.text}`.toLowerCase().includes(term))
    ));
    if (!list.length) return '<div class="panel-empty">No hay eventos que coincidan.</div>';
    return list.map((entry) => `<div class="log-row log-${entry.kind}"><span class="log-time">${fmt(entry.at)}</span><span class="log-kind">${entry.kind}</span><span class="log-text">${escape(entry.text)}</span></div>`).join('');
  };

  const textOf = () => getLog().map((e) => `${new Date(e.at).toLocaleString('es-ES')} [${e.kind}] ${e.text}`).join('\n');

  const modal = openModal({
    title: `Registro (${entries.length})`,
    width: 860,
    body: `
      <div class="log-tools">
        <input class="input" data-log-filter placeholder="Buscar errores, archivos, rutas…" />
        <label class="log-only"><input type="checkbox" data-log-errors /> Solo errores</label>
        <span class="log-count" data-log-count>${entries.length} eventos</span>
      </div>
      <div class="log-list" data-log-list>${rowsFor()}</div>`,
    footer: `
      <button class="btn" data-log-download>Descargar .txt</button>
      <button class="btn" data-log-copy style="${hasErrors ? 'background:#c0392b;color:#fff;border-color:#c0392b' : ''}">Copiar${hasErrors ? ' (hay errores)' : ''}</button>
      <button class="btn" data-log-clear>Vaciar</button>
      <button class="btn btn-primary" data-modal-close>Cerrar</button>`,
    onMount: (modal) => {
      const list = modal.body.querySelector('[data-log-list]');
      const count = modal.body.querySelector('[data-log-count]');
      const filter = modal.body.querySelector('[data-log-filter]');
      const onlyErrors = modal.body.querySelector('[data-log-errors]');
      const refresh = () => {
        const current = getLog();
        list.innerHTML = rowsFor(filter?.value || '', Boolean(onlyErrors?.checked));
        const shown = list.querySelectorAll('.log-row').length;
        if (count) count.textContent = `${shown} de ${current.length} eventos`;
      };
      filter?.addEventListener('input', refresh);
      onlyErrors?.addEventListener('change', refresh);

      modal.foot.querySelector('[data-log-download]')?.addEventListener('click', () => {
        downloadBlob(new Blob([textOf()], { type: 'text/plain;charset=utf-8' }), `cupcraft-registro-${Date.now()}.txt`);
        logEvent('app', 'Registro descargado');
        toast('Registro descargado', 'success', 1500);
      });
      modal.foot.querySelector('[data-log-copy]')?.addEventListener('click', async () => {
        const text = textOf();
        let ok = false;
        try {
          if (navigator.clipboard?.writeText) {
            await navigator.clipboard.writeText(text);
            ok = true;
          }
        } catch { ok = false; }
        if (!ok) {
          try {
            const ta = document.createElement('textarea');
            ta.value = text;
            ta.style.position = 'fixed';
            ta.style.left = '-9999px';
            document.body.appendChild(ta);
            ta.select();
            ok = document.execCommand('copy');
            ta.remove();
          } catch { ok = false; }
        }
        if (ok) logEvent('app', 'Registro copiado al portapapeles');
        toast(ok ? 'Registro copiado' : 'No se pudo copiar', ok ? 'success' : 'error', 1500);
      });
      modal.foot.querySelector('[data-log-clear]')?.addEventListener('click', () => {
        clearLog();
        openLogDialog();
      });
    },
  });
  return modal;
}

function escape(text) {
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
