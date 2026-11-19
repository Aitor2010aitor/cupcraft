import { createServer } from 'node:http';
import { randomBytes } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { capcutMime } from './capcut.mjs';

/* ------------------------------------------------------------------ */
/*  Puente de control para IA                                          */
/*  Permite que otra IA lea el estado de cupcraft y ejecute acciones      */
/*  sobre la aplicación abierta en el navegador.                       */
/* ------------------------------------------------------------------ */

const root = fileURLToPath(new URL('..', import.meta.url));
const tokenPath = join(root, '.openCUP-ai-token');

function loadToken() {
  if (process.env.openCUP_AI_TOKEN) return process.env.openCUP_AI_TOKEN.trim();
  try {
    const saved = readFileSync(tokenPath, 'utf8').trim();
    if (saved) return saved;
  } catch { /* se crea uno nuevo */ }
  const token = randomBytes(18).toString('hex');
  try { writeFileSync(tokenPath, token, 'utf8'); } catch { /* solo en memoria */ }
  return token;
}

export const aiToken = loadToken();

export const AI_ACTIONS = [
  { action: 'getState', description: 'Devuelve el estado completo: proyecto, clips, medios, selección, cursor y paneles.' },
  { action: 'listProjects', description: 'Lista los proyectos guardados en cupcraft.' },
  { action: 'newProject', params: { name: 'string' }, description: 'Crea un proyecto nuevo y abre el editor.' },
  { action: 'openProject', params: { id: 'string', name: 'string (alternativa)' }, description: 'Abre un proyecto guardado por id o nombre.' },
  { action: 'goHome', description: 'Vuelve a la pantalla de inicio.' },
  { action: 'save', description: 'Guarda el proyecto actual.' },
  { action: 'listCapCutProjects', description: 'Lista los borradores nativos de CapCut detectados en este equipo.' },
  { action: 'openCapCutProject', params: { id: 'string del listado' }, description: 'Importa y abre un proyecto nativo de CapCut.' },
  { action: 'setPanel', params: { panel: 'media|audio|text|stickers|effects|transitions|captions|filters|adjust|templates|avatar' }, description: 'Cambia el panel de herramientas.' },
  { action: 'seek', params: { time: 'segundos' }, description: 'Mueve el cursor de reproducción.' },
  { action: 'play', description: 'Reproduce la línea de tiempo.' },
  { action: 'pause', description: 'Pausa la reproducción.' },
  { action: 'togglePlay', description: 'Alterna reproducción y pausa.' },
  { action: 'selectClip', params: { id: 'string|null' }, description: 'Selecciona un clip por id.' },
  { action: 'addText', params: { content: 'string', start: 'segundos', duration: 'segundos', size: 'número', color: '#rrggbb', weight: '100-900' }, description: 'Añade un texto o subtítulo.' },
  { action: 'addSticker', params: { value: 'emoji', start: 'segundos', duration: 'segundos', size: 'número', scale: 'número', background: '#rrggbb', shape: 'none|circle|square' }, description: 'Añade un sticker como capa independiente.' },
  { action: 'applyFilter', params: { clip: 'id (opcional)', filter: 'id de listFilters' }, description: 'Aplica un filtro de color del catálogo local (731 filtros en 19 categorías).' },
  { action: 'applyEffect', params: { clip: 'id (opcional)', effect: 'id de listEffects' }, description: 'Aplica un efecto visual del catálogo local.' },
  { action: 'applyTransition', params: { clip: 'id (opcional)', transition: 'id de listTransitions', duration: 'segundos' }, description: 'Aplica una transición de entrada al clip (16 tipos).' },
  { action: 'updateClip', params: { id: 'string', patch: 'objeto' }, description: 'Modifica propiedades de un clip (volumen, escala, posición, rotación, velocidad, texto o sticker).' },
  { action: 'split', description: 'Divide en el cursor.' },
  { action: 'deleteClip', description: 'Elimina el clip seleccionado.' },
  { action: 'duplicateClip', description: 'Duplica el clip seleccionado.' },
  { action: 'undo', description: 'Deshace el último cambio.' },
  { action: 'redo', description: 'Rehace el último cambio.' },
  { action: 'setZoom', params: { value: 'número 30-260' }, description: 'Cambia el zoom de la línea de tiempo.' },
  { action: 'listFilters', description: 'Lista los 731 filtros disponibles con su categoría.' },
  { action: 'listStickers', description: 'Lista los 219 stickers disponibles por categoría.' },
  { action: 'listEffects', description: 'Lista los 168 efectos disponibles con su categoría.' },
  { action: 'listTransitions', description: 'Lista las transiciones disponibles con su categoría.' },
  { action: 'screenshot', description: 'Captura un fotograma del reproductor y devuelve un data URL.' },
];

const clients = new Map();
const commands = new Map();
const pending = [];
const results = new Map();
const waiters = new Map();
let sequence = 0;
let bridgePort = 0;

const now = () => Date.now();
const trimCommands = () => {
  if (commands.size < 300) return;
  const entries = [...commands.values()].sort((a, b) => a.createdAt - b.createdAt);
  for (const entry of entries.slice(0, 100)) {
    commands.delete(entry.id);
    results.delete(entry.id);
    waiters.delete(entry.id);
  }
};

function json(res, status, body, headers = {}) {
  const payload = JSON.stringify(body, null, 2);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(payload),
    'Cache-Control': 'no-store',
    ...headers,
  });
  res.end(payload);
}

function text(res, status, body) {
  res.writeHead(status, { 'Content-Type': 'text/plain; charset=utf-8', 'Cache-Control': 'no-store' });
  res.end(body);
}

function cors(res) {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-openCUP-Token');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
}

function authorized(req, url) {
  const header = String(req.headers.authorization || '');
  const bearer = header.toLowerCase().startsWith('bearer ') ? header.slice(7).trim() : '';
  const token = bearer || String(req.headers['x-openCUP-token'] || '') || url.searchParams.get('token') || '';
  return token === aiToken;
}

function readBody(req, limit = 1_000_000) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', (chunk) => {
      body += chunk;
      if (body.length > limit) {
        reject(new Error('Cuerpo demasiado grande'));
        req.destroy();
      }
    });
    req.on('end', () => {
      if (!body) return resolve({});
      try { resolve(JSON.parse(body)); } catch { reject(new Error('JSON no válido')); }
    });
    req.on('error', reject);
  });
}

function activeClients() {
  const cutoff = now() - 20_000;
  for (const [id, client] of clients) {
    if (client.lastSeen < cutoff) clients.delete(id);
  }
  return clients;
}

function newestClient() {
  const cutoff = now() - 5_000;
  const live = [...activeClients().values()].filter((client) => client.lastSeen >= cutoff);
  return live.sort((a, b) => (
    (b.firstSeen || 0) - (a.firstSeen || 0) || (b.lastSeen || 0) - (a.lastSeen || 0)
  ))[0] || null;
}

export function bridgeSnapshot() {
  const target = newestClient();
  return {
    ok: true,
    client: target?.id || null,
    at: target?.stateAt || 0,
    clients: activeClients().size,
    state: target?.state ?? null,
  };
}

function enqueue(action, params = {}) {
  const id = `cmd_${Date.now().toString(36)}_${(sequence += 1).toString(36)}`;
  const entry = {
    id,
    action,
    params: params && typeof params === 'object' ? params : {},
    status: 'pending',
    createdAt: now(),
    client: null,
  };
  commands.set(id, entry);
  pending.push(entry);
  trimCommands();
  return entry;
}

function finish(id, payload) {
  const entry = commands.get(id);
  if (!entry) return false;
  entry.status = payload.ok ? 'done' : 'error';
  entry.finishedAt = now();
  entry.result = payload.result;
  entry.error = payload.error || '';
  results.set(id, {
    id,
    ok: Boolean(payload.ok),
    result: payload.result ?? null,
    error: payload.error || '',
    finishedAt: entry.finishedAt,
  });
  const pendingIndex = pending.findIndex((item) => item.id === id);
  if (pendingIndex >= 0) pending.splice(pendingIndex, 1);
  const waiter = waiters.get(id);
  if (waiter) {
    clearTimeout(waiter.timer);
    waiters.delete(id);
    waiter.resolve(results.get(id));
  }
  return true;
}

function waitForResult(id, timeout = 25_000) {
  if (results.has(id)) return Promise.resolve(results.get(id));
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      waiters.delete(id);
      resolve({ id, ok: false, pending: true, error: 'Sin respuesta del navegador. Abre cupcraft y activa el puente IA.' });
    }, timeout);
    waiters.set(id, { resolve, timer });
  });
}

export function aiStatus({ includeToken = false } = {}) {
  const live = activeClients();
  const target = newestClient();
  return {
    ok: true,
    name: 'cupcraft AI Bridge',
    version: '1.0.0',
    port: bridgePort,
    enabled: true,
    connected: live.size > 0,
    clients: live.size,
    target: target?.id || null,
    pending: pending.length,
    lastStateAt: target?.stateAt || 0,
    token: includeToken ? aiToken : undefined,
    endpoint: `http://127.0.0.1:${bridgePort}/api/ai`,
    docs: `http://127.0.0.1:${bridgePort}/api/ai/actions`,
  };
}

export function bridgeCommand(action, params = {}) {
  const entry = enqueue(action, params);
  return { id: entry.id, status: entry.status, createdAt: entry.createdAt };
}

export function bridgeTake(clientId) {
  const client = clients.get(clientId) || { id: clientId, firstSeen: now(), state: null, stateAt: 0 };
  client.lastSeen = now();
  clients.set(clientId, client);
  const target = newestClient();
  if (target && target.id !== clientId) return null;
  const entry = pending.find((item) => item.status === 'pending');
  if (!entry) return null;
  entry.status = 'taken';
  entry.client = clientId;
  entry.takenAt = now();
  return { id: entry.id, action: entry.action, params: entry.params, createdAt: entry.createdAt };
}

export function bridgeResult(clientId, id, payload) {
  const client = clients.get(clientId);
  if (client) client.lastSeen = now();
  return finish(id, payload);
}

export function bridgeState(clientId, value, startedAt = 0) {
  const started = Number.isFinite(startedAt) && startedAt > 0 ? startedAt : now();
  const client = clients.get(clientId) || { id: clientId, firstSeen: started, state: null, stateAt: 0 };
  client.lastSeen = now();
  client.state = value;
  client.stateAt = now();
  clients.set(clientId, client);
  return true;
}

export function bridgeLog() {
  return [...commands.values()]
    .sort((a, b) => b.createdAt - a.createdAt)
    .slice(0, 50)
    .map((entry) => ({
      id: entry.id,
      action: entry.action,
      status: entry.status,
      client: entry.client,
      createdAt: entry.createdAt,
      finishedAt: entry.finishedAt,
      error: entry.error,
    }));
}

function welcomePage() {
  return `<!doctype html><html lang="es"><meta charset="utf-8"><title>cupcraft AI Bridge</title>
<style>body{background:#0c1112;color:#e8f4f4;font:14px/1.6 system-ui,sans-serif;padding:40px;max-width:860px;margin:auto}code{background:#132b2e;padding:2px 6px;border-radius:4px;color:#7deff5}pre{background:#101a1c;border:1px solid #1f3a3d;border-radius:8px;padding:16px;overflow:auto}a{color:#57e5ef}h1{font-size:24px}</style>
<h1>cupcraft AI Bridge</h1>
<p>Puerto <code>${bridgePort}</code>. Requiere el token que aparece en cupcraft → Menú → Puente IA.</p>
<pre>curl -H "Authorization: Bearer $openCUP_AI_TOKEN" http://127.0.0.1:${bridgePort}/api/ai/status
curl -H "Authorization: Bearer $openCUP_AI_TOKEN" http://127.0.0.1:${bridgePort}/api/ai/state
curl -X POST -H "Authorization: Bearer $openCUP_AI_TOKEN" -H "Content-Type: application/json" \\
  -d '{"action":"addText","params":{"content":"Hola desde una IA","start":0,"duration":3},"wait":true}' \\
  http://127.0.0.1:${bridgePort}/api/ai/command</pre>
<p><a href="/api/ai/actions">Ver todas las acciones</a></p>
<p>Este servidor escucha únicamente en 127.0.0.1. No se expone a Internet.</p>`;
}

export function startAiBridge(preferredPort = Number(process.env.openCUP_AI_PORT || 8790), attempts = 20) {
  const server = createServer(async (req, res) => {
    cors(res);
    if (req.method === 'OPTIONS') { res.writeHead(204); res.end(); return; }
    const url = new URL(req.url, `http://${req.headers.host || '127.0.0.1'}`);
    try {
      if (req.method === 'GET' && (url.pathname === '/' || url.pathname === '/api/ai')) {
        if (!authorized(req, url)) { text(res, 401, 'Token requerido. Consulta cupcraft → Menú → Puente IA.'); return; }
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' });
        res.end(welcomePage());
        return;
      }
      if (req.method === 'GET' && url.pathname === '/api/ai/actions') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        json(res, 200, { ok: true, actions: AI_ACTIONS });
        return;
      }
      if (req.method === 'GET' && url.pathname === '/api/ai/status') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        json(res, 200, aiStatus());
        return;
      }
      if (req.method === 'GET' && url.pathname === '/api/ai/state') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        json(res, 200, bridgeSnapshot());
        return;
      }
      if (req.method === 'GET' && url.pathname === '/api/ai/result') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        const id = url.searchParams.get('id') || '';
        const result = results.get(id);
        if (!result) { json(res, 200, { ok: false, id, pending: true }); return; }
        json(res, 200, result);
        return;
      }
      if (req.method === 'GET' && url.pathname === '/api/ai/commands') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        json(res, 200, { ok: true, commands: bridgeLog() });
        return;
      }
      if (req.method === 'POST' && url.pathname === '/api/ai/command') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        const body = await readBody(req);
        const action = String(body.action || '').trim();
        if (!action) { json(res, 400, { ok: false, error: 'Falta action' }); return; }
        if (!AI_ACTIONS.some((item) => item.action === action)) {
          json(res, 400, { ok: false, error: `Acción desconocida: ${action}` });
          return;
        }
        const entry = enqueue(action, body.params || {});
        if (body.wait === false) {
          json(res, 202, { ok: true, id: entry.id, status: 'pending' });
          return;
        }
        const timeout = Math.max(1000, Math.min(30_000, Number(body.timeout) || 25_000));
        const result = await waitForResult(entry.id, timeout);
        json(res, result.ok ? 200 : result.pending ? 202 : 500, result);
        return;
      }
      if (req.method === 'POST' && url.pathname === '/api/ai/command/cancel') {
        if (!authorized(req, url)) { json(res, 401, { ok: false, error: 'Token requerido' }); return; }
        const body = await readBody(req);
        const id = String(body.id || '');
        const entry = commands.get(id);
        if (!entry) { json(res, 404, { ok: false, error: 'Comando no encontrado' }); return; }
        finish(id, { ok: false, error: 'Cancelado' });
        json(res, 200, { ok: true, id, status: 'error' });
        return;
      }
      json(res, 404, { ok: false, error: 'No encontrado' });
    } catch (error) {
      json(res, 500, { ok: false, error: error?.message || 'Error interno' });
    }
  });

  return new Promise((resolve, reject) => {
    let attempt = 0;
    const tryListen = (port) => {
      server.once('error', (error) => {
        if (error?.code === 'EADDRINUSE' && attempt < attempts) {
          attempt += 1;
          tryListen(preferredPort + attempt);
        } else {
          reject(error);
        }
      });
      server.listen(port, '127.0.0.1', () => {
        bridgePort = port;
        resolve({ port, server, token: aiToken, close: () => server.close() });
      });
    };
    tryListen(preferredPort);
  });
}

export function mediaContentType(path) {
  return capcutMime(path);
}
