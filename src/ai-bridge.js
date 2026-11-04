import {
  state,
  on,
  newProject,
  openProject,
  saveNow,
  goHome,
  setPanel,
  seek,
  play,
  pause,
  togglePlay,
  selectClip,
  addTextClip,
  addStickerClip,
  updateClip,
  splitAtPlayhead,
  deleteSelected,
  duplicateSelected,
  undo,
  redo,
  setZoom,
  projectDuration,
  selectedClip,
  findClip,
  FILTERS,
  TEXT_PRESETS,
} from './store.js';
import { analyzeProject, planEdit, applyPlan, undoAutoEdit } from './auto-edit.js';
import { STICKERS, STICKER_CATEGORIES, EFFECTS, TRANSITIONS, AVATAR_PRESETS, TRANSITION_FADES } from './library.js';
import { renderStill } from './renderer.js';
import { icon } from './icons.js';
import { openModal } from './modal.js';
import { escapeHtml, toast } from './utils.js';
import { listCapCutProjects, importCapCutProject } from './capcut.js';

export const AI_ACTIONS = [
  { action: 'getState', description: 'Estado completo: proyecto, pistas, clips, medios, selección, cursor y panel.' },
  { action: 'listProjects', description: 'Proyectos guardados en openCUP.' },
  { action: 'newProject', params: { name: 'string' }, description: 'Crea un proyecto y abre el editor.' },
  { action: 'openProject', params: { id: 'string', name: 'string (alternativa)' }, description: 'Abre un proyecto guardado.' },
  { action: 'goHome', description: 'Vuelve al inicio.' },
  { action: 'save', description: 'Guarda el proyecto actual.' },
  { action: 'listCapCutProjects', description: 'Detecta los borradores nativos de CapCut del equipo.' },
  { action: 'openCapCutProject', params: { id: 'string' }, description: 'Importa y abre un borrador de CapCut.' },
  { action: 'setPanel', params: { panel: 'media|audio|text|stickers|effects|transitions|captions|filters|adjust|templates|avatar' }, description: 'Cambia el panel de herramientas.' },
  { action: 'seek', params: { time: 'segundos' }, description: 'Mueve el cursor.' },
  { action: 'play', description: 'Reproduce.' },
  { action: 'pause', description: 'Pausa.' },
  { action: 'togglePlay', description: 'Alterna reproducción.' },
  { action: 'selectClip', params: { id: 'string|null' }, description: 'Selecciona un clip.' },
  { action: 'addText', params: { content: 'string', start: 'segundos', duration: 'segundos', size: 'número', color: '#rrggbb', weight: 'número' }, description: 'Añade un texto o subtítulo.' },
  { action: 'addSticker', params: { value: 'emoji', start: 'segundos', duration: 'segundos', size: 'número' }, description: 'Añade un sticker.' },
  { action: 'applyFilter', params: { clip: 'id (opcional)', filter: 'id de filtro' }, description: 'Aplica un filtro.' },
  { action: 'applyEffect', params: { clip: 'id (opcional)', effect: 'id de efecto' }, description: 'Aplica un efecto visual.' },
  { action: 'applyTransition', params: { clip: 'id (opcional)', transition: 'id', duration: 'segundos' }, description: 'Aplica una transición al inicio del clip.' },
  { action: 'updateClip', params: { id: 'string', patch: 'objeto' }, description: 'Modifica propiedades de un clip.' },
  { action: 'split', description: 'Divide en el cursor.' },
  { action: 'deleteClip', description: 'Elimina el clip seleccionado.' },
  { action: 'duplicateClip', description: 'Duplica el clip seleccionado.' },
  { action: 'undo', description: 'Deshace.' },
  { action: 'redo', description: 'Rehace.' },
  { action: 'setZoom', params: { value: '30-260' }, description: 'Zoom de la línea de tiempo.' },
  { action: 'listFilters', description: 'Filtros disponibles.' },
  { action: 'listStickers', description: 'Stickers disponibles por categoría.' },
  { action: 'listEffects', description: 'Efectos disponibles.' },
  { action: 'listTransitions', description: 'Transiciones disponibles.' },
  { action: 'screenshot', description: 'Devuelve un fotograma del reproductor como data URL JPEG.' },
  { action: 'autoAnalyze', description: 'Analiza los medios del proyecto (duración, resolución, escenas, silencios, ritmo).' },
  { action: 'autoPlan', params: { instruction: 'string' }, description: 'Genera un plan de montaje a partir de una instrucción en lenguaje natural.' },
  { action: 'autoApply', params: { instruction: 'string (opcional si ya hay plan)' }, description: 'Analiza, planifica y aplica un montaje automático real a la timeline.' },
  { action: 'autoUndo', description: 'Deshace el último montaje automático.' },
];

const PANELS = ['media', 'audio', 'text', 'stickers', 'effects', 'transitions', 'captions', 'filters', 'adjust', 'templates', 'avatar', 'auto'];

const clientStartedAt = Date.now();

const clientId = (() => {
  const key = 'openCUP-ai-client';
  try {
    const saved = sessionStorage.getItem(key);
    if (saved) return saved;
    const value = `web_${Math.random().toString(36).slice(2, 10)}${Date.now().toString(36)}`;
    sessionStorage.setItem(key, value);
    return value;
  } catch {
    return `web_${Math.random().toString(36).slice(2, 12)}`;
  }
})();

let pollTimer = 0;
let polling = false;
let lastPollError = '';
let statusCache = null;

function clipSnapshot(clip) {
  const snapshot = {
    id: clip.id,
    kind: clip.kind,
    name: clip.name,
    start: Number((clip.start || 0).toFixed(3)),
    duration: Number((clip.duration || 0).toFixed(3)),
    mediaId: clip.mediaId || null,
    scale: clip.scale ?? 1,
    x: clip.x || 0,
    y: clip.y || 0,
    rotate: clip.rotate || 0,
    opacity: clip.opacity ?? 1,
    filter: clip.filter || 'none',
    effect: clip.effect || 'none',
    transition: clip.transition || 'none',
    fadeIn: clip.fadeIn || 0,
    fadeOut: clip.fadeOut || 0,
  };
  if (clip.text) snapshot.text = { ...clip.text };
  if (clip.sticker) snapshot.sticker = { ...clip.sticker };
  if (clip.kind === 'video' || clip.kind === 'audio') {
    snapshot.volume = clip.volume ?? 1;
    snapshot.speed = clip.speed ?? 1;
    snapshot.in = clip.in || 0;
  }
  return snapshot;
}

export function publicState() {
  const project = state.project;
  return {
    view: state.view,
    panel: state.panel,
    selection: state.selection,
    playhead: Number((state.playhead || 0).toFixed(3)),
    playing: Boolean(state.playing),
    zoom: state.zoom,
    projectCount: state.projects.length,
    project: project ? {
      id: project.id,
      name: project.name,
      ratio: project.ratio,
      resolution: project.resolution,
      fps: project.fps,
      duration: Number(projectDuration().toFixed(3)),
      thumbnail: Boolean(project.thumb),
      source: project.source || null,
      tracks: (project.tracks || []).map((track) => ({
        id: track.id,
        type: track.type,
        name: track.name,
        hidden: Boolean(track.hidden),
        muted: Boolean(track.muted),
        clips: (track.clips || []).map(clipSnapshot),
      })),
    } : null,
    media: Array.from(state.media.values()).map((media) => ({
      id: media.id,
      name: media.name,
      kind: media.kind,
      duration: Number((media.duration || 0).toFixed(3)),
      width: media.width || 0,
      height: media.height || 0,
      size: media.size || 0,
      missing: Boolean(media.missing),
    })),
    capabilities: {
      capcutImport: true,
      localBridge: true,
      cloudAI: false,
      manualCaptions: true,
      srtImport: true,
      exports: ['webm', 'png'],
    },
  };
}

function requireProject() {
  if (!state.project) throw new Error('No hay ningún proyecto abierto');
  return state.project;
}

function targetClip(params = {}) {
  if (params.clip) return findClip(String(params.clip));
  return selectedClip();
}

async function projectByNameOrId(params = {}) {
  let target = null;
  if (params.id) target = state.projects.find((project) => project.id === params.id);
  if (!target && params.name) target = state.projects.find((project) => project.name === params.name);
  if (!target && params.name) {
    const query = String(params.name).toLowerCase();
    target = state.projects.find((project) => String(project.name).toLowerCase().includes(query));
  }
  return target;
}

const HANDLERS = {
  getState() {
    return publicState();
  },

  listProjects() {
    return state.projects.map((project) => ({
      id: project.id,
      name: project.name,
      ratio: project.ratio,
      resolution: project.resolution,
      updatedAt: project.updatedAt,
      createdAt: project.createdAt,
    }));
  },

  async newProject(params = {}) {
    if (state.project) await saveNow();
    await newProject(String(params.name || 'Proyecto de IA').slice(0, 80));
    return { id: state.project.id, name: state.project.name, view: state.view };
  },

  async openProject(params = {}) {
    const target = await projectByNameOrId(params);
    if (!target) throw new Error('Proyecto no encontrado');
    if (state.project && state.project.id !== target.id) await saveNow();
    await openProject(target.id);
    return { id: state.project?.id, name: state.project?.name, view: state.view };
  },

  goHome() {
    goHome();
    return { view: state.view };
  },

  async save() {
    requireProject();
    await saveNow();
    return { savedAt: state.lastSaved, name: state.project.name };
  },

  async listCapCutProjects() {
    const result = await listCapCutProjects();
    return result;
  },

  async openCapCutProject(params = {}) {
    if (!params.id) throw new Error('Falta el id del proyecto de CapCut');
    if (state.project) await saveNow();
    const project = await importCapCutProject(params.id);
    return { id: state.project?.id, name: state.project?.name, view: state.view, project };
  },

  setPanel(params = {}) {
    const panel = String(params.panel || '');
    if (!PANELS.includes(panel)) throw new Error(`Panel no válido: ${panel}`);
    if (state.view !== 'editor') throw new Error('Abre un proyecto para cambiar de panel');
    setPanel(panel);
    return { panel: state.panel };
  },

  seek(params = {}) {
    requireProject();
    const time = Number(params.time);
    if (!Number.isFinite(time)) throw new Error('time debe ser un número');
    seek(time);
    return { playhead: state.playhead };
  },

  play() {
    requireProject();
    play();
    return { playing: state.playing };
  },

  pause() {
    pause();
    return { playing: state.playing };
  },

  togglePlay() {
    requireProject();
    togglePlay();
    return { playing: state.playing };
  },

  selectClip(params = {}) {
    requireProject();
    const id = params.id ?? null;
    if (id && !findClip(String(id))) throw new Error('Clip no encontrado');
    selectClip(id ? String(id) : null);
    return { selection: state.selection };
  },

  addText(params = {}) {
    requireProject();
    const content = String(params.content ?? '').trim();
    if (!content) throw new Error('Falta content');
    const duration = Math.max(0.2, Number(params.duration) || 5);
    const start = Number.isFinite(Number(params.start)) ? Number(params.start) : state.playhead;
    const preset = {
      size: Math.max(8, Math.min(320, Number(params.size) || 72)),
      weight: Math.max(100, Math.min(900, Number(params.weight) || 700)),
      color: /^#[0-9a-fA-F]{6}$/.test(params.color || '') ? params.color : '#ffffff',
    };
    const clip = addTextClip(content, { start, duration, preset });
    return { clip: clip ? clipSnapshot(clip) : null };
  },

  addSticker(params = {}) {
    requireProject();
    const value = String(params.value ?? '✨').slice(0, 8);
    if (!value) throw new Error('Falta value');
    const duration = Math.max(0.2, Number(params.duration) || 4);
    const start = Number.isFinite(Number(params.start)) ? Number(params.start) : state.playhead;
    const clip = addStickerClip(value, {
      start,
      duration,
      scale: Number(params.scale) || 1,
      x: Number(params.x) || 0,
      y: Number(params.y) || 0,
      preset: {
        size: Math.max(24, Math.min(700, Number(params.size) || 220)),
        background: /^#[0-9a-fA-F]{6}$/.test(params.background || '') ? params.background : 'transparent',
        shape: params.shape === 'square' ? 'square' : 'none',
      },
    });
    return { clip: clip ? clipSnapshot(clip) : null };
  },

  applyFilter(params = {}) {
    requireProject();
    const found = targetClip(params);
    if (!found) throw new Error('No hay clip seleccionado');
    const filter = String(params.filter || 'none');
    if (!FILTERS[filter]) throw new Error(`Filtro no válido: ${filter}`);
    updateClip(found.clip.id, { filter });
    return { clip: found.clip.id, filter };
  },

  applyEffect(params = {}) {
    requireProject();
    const found = targetClip(params);
    if (!found) throw new Error('No hay clip seleccionado');
    const effect = String(params.effect || 'none');
    if (!EFFECTS[effect]) throw new Error(`Efecto no válido: ${effect}`);
    updateClip(found.clip.id, { effect, effectStrength: Number(params.strength) || 1 });
    return { clip: found.clip.id, effect };
  },

  applyTransition(params = {}) {
    requireProject();
    const found = targetClip(params);
    if (!found) throw new Error('No hay clip seleccionado');
    const type = String(params.transition || 'fade');
    if (!TRANSITIONS.some((item) => item.id === type)) throw new Error(`Transición no válida: ${type}`);
    const fades = TRANSITION_FADES[type] || { fadeIn: 0.2, fadeOut: 0.2 };
    updateClip(found.clip.id, {
      transition: type,
      transitionDuration: Math.max(0.1, Math.min(3, Number(params.duration) || 0.6)),
      ...fades,
    });
    return { clip: found.clip.id, transition: type };
  },

  updateClip(params = {}) {
    requireProject();
    const id = String(params.id || state.selection || '');
    if (!id) throw new Error('Falta id');
    const found = findClip(id);
    if (!found) throw new Error('Clip no encontrado');
    const patch = params.patch && typeof params.patch === 'object' ? params.patch : {};
    const allowed = ['start', 'duration', 'in', 'speed', 'volume', 'opacity', 'scale', 'x', 'y', 'rotate', 'filter', 'effect', 'transition', 'transitionDuration', 'fadeIn', 'fadeOut', 'text', 'adjust', 'sticker', 'name'];
    const clean = {};
    for (const key of allowed) {
      if (patch[key] !== undefined) clean[key] = patch[key];
    }
    if (!Object.keys(clean).length) throw new Error('patch no contiene propiedades válidas');
    updateClip(id, clean);
    return { clip: clipSnapshot(found.clip) };
  },

  split() {
    requireProject();
    splitAtPlayhead();
    return { playhead: state.playhead, clips: state.project.tracks.reduce((sum, track) => sum + track.clips.length, 0) };
  },

  deleteClip() {
    const found = selectedClip();
    if (!found) throw new Error('No hay clip seleccionado');
    const id = found.clip.id;
    deleteSelected();
    return { deleted: id };
  },

  duplicateClip() {
    const found = selectedClip();
    if (!found) throw new Error('No hay clip seleccionado');
    duplicateSelected();
    return { selection: state.selection };
  },

  undo() {
    requireProject();
    undo();
    return { history: state.history.length, future: state.future.length };
  },

  redo() {
    requireProject();
    redo();
    return { history: state.history.length, future: state.future.length };
  },

  setZoom(params = {}) {
    const value = Number(params.value);
    if (!Number.isFinite(value)) throw new Error('value debe ser un número');
    setZoom(value);
    return { zoom: state.zoom };
  },

  listFilters() {
    return Object.entries(FILTERS).map(([id, value]) => ({ id, label: value.label, category: value.category }));
  },

  listStickers() {
    return { total: STICKERS.length, categories: STICKER_CATEGORIES.map((category) => ({ name: category.name, items: category.items })) };
  },

  listEffects() {
    return Object.entries(EFFECTS).map(([id, value]) => ({ id, label: value.label, category: value.category }));
  },

  listTransitions() {
    return TRANSITIONS.map(({ id, label, category }) => ({ id, label, category }));
  },

  async screenshot(params = {}) {
    requireProject();
    const width = Math.max(320, Math.min(1600, Number(params.width) || 960));
    const height = Math.round(width * 9 / 16);
    const canvas = await renderStill(state.project, state.media, state.playhead, width, height);
    return {
      playhead: state.playhead,
      width,
      height,
      dataUrl: canvas.toDataURL('image/jpeg', 0.85),
    };
  },

  async autoAnalyze() {
    requireProject();
    return await analyzeProject();
  },

  async autoPlan(params = {}) {
    requireProject();
    const analysis = await analyzeProject();
    const plan = await planEdit(params.instruction || '', analysis);
    return { instruction: params.instruction || '', plan };
  },

  async autoApply(params = {}) {
    requireProject();
    const instruction = params.instruction || '';
    const analysis = await analyzeProject();
    const plan = await planEdit(instruction, analysis);
    await applyPlan(plan, (i, total, op) => { /* progress */ });
    return { applied: true, operations: plan.operations.map((o) => o.op) };
  },

  async autoUndo() {
    const ok = undoAutoEdit();
    return { undone: ok };
  },
};

export async function runCommand(action, params = {}) {
  const handler = HANDLERS[action];
  if (!handler) throw new Error(`Acción desconocida: ${action}`);
  return handler(params && typeof params === 'object' ? params : {});
}

export async function localCommand(action, params = {}) {
  return runCommand(action, params);
}

async function sendState() {
  const response = await fetch('/api/ai/poll', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ clientId, startedAt: clientStartedAt, state: publicState() }),
  });
  if (!response.ok) throw new Error(`Puente IA: HTTP ${response.status}`);
  const data = await response.json();
  lastPollError = '';
  if (data.command) {
    const command = data.command;
    let payload;
    try {
      const result = await runCommand(command.action, command.params || {});
      payload = { ok: true, result };
    } catch (error) {
      payload = { ok: false, error: error?.message || 'Error al ejecutar la orden' };
    }
    await fetch('/api/ai/result', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ clientId, id: command.id, ...payload }),
    }).catch(() => {});
  }
}

async function pollLoop() {
  if (polling) return;
  polling = true;
  try {
    if (document.visibilityState !== 'hidden') await sendState();
  } catch (error) {
    lastPollError = error?.message || String(error);
  } finally {
    polling = false;
    clearTimeout(pollTimer);
    pollTimer = setTimeout(pollLoop, 850);
  }
}

export function startAiBridge() {
  if (!/^(127\.0\.0\.1|localhost|\[::1\])$/.test(window.location.hostname)) return;
  clearTimeout(pollTimer);
  pollTimer = setTimeout(pollLoop, 500);
  window.addEventListener('online', () => {
    clearTimeout(pollTimer);
    pollTimer = setTimeout(pollLoop, 200);
  });
}

export async function fetchAiStatus() {
  try {
    const response = await fetch('/api/ai/status');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    statusCache = await response.json();
    return statusCache;
  } catch (error) {
    return { ok: false, error: error?.message || 'No se pudo consultar el puente' };
  }
}

function copyText(text) {
  if (navigator.clipboard?.writeText) {
    return navigator.clipboard.writeText(text).then(() => true).catch(() => false);
  }
  try {
    const area = document.createElement('textarea');
    area.value = text;
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.append(area);
    area.select();
    const ok = document.execCommand('copy');
    area.remove();
    return Promise.resolve(ok);
  } catch {
    return Promise.resolve(false);
  }
}

const EXAMPLE = `curl -H "Authorization: Bearer TOKEN" http://127.0.0.1:8790/api/ai/status

curl -X POST -H "Authorization: Bearer TOKEN" -H "Content-Type: application/json" \\
  -d '{"action":"addText","params":{"content":"Hola desde una IA","start":0,"duration":3}}' \\
  http://127.0.0.1:8790/api/ai/command`;

function bridgeMarkup(status) {
  if (!status?.ok) {
    return `
      <div class="bridge-offline">${icon('alert', 22)}<strong>Puente no disponible</strong>
        <span>${escapeHtml(status?.error || 'Inicia openCUP con node server.mjs.')}</span>
      </div>`;
  }
  const endpoint = `http://127.0.0.1:${status.port}`;
  return `
    <div class="bridge-hero">
      <div class="bridge-pulse"></div>
      <div class="bridge-hero-text">
        <strong>openCUP AI Bridge</strong>
        <span>${status.connected ? 'Aplicación conectada y escuchando órdenes' : 'Servidor activo · esperando a la aplicación'}</span>
      </div>
      <span class="bridge-badge">${status.connected ? 'CONECTADO' : 'ESPERANDO'}</span>
    </div>
    <div class="bridge-grid">
      <div class="bridge-stat"><span>Puerto</span><b>${status.port}</b></div>
      <div class="bridge-stat"><span>Clientes</span><b>${status.clients ?? 0}</b></div>
      <div class="bridge-stat"><span>En cola</span><b>${status.pending ?? 0}</b></div>
    </div>
    <div class="bridge-field">
      <label>Endpoint</label>
      <div class="bridge-copy-row">
        <code data-bridge-endpoint>${endpoint}</code>
        <button class="btn btn-small" data-bridge-copy="endpoint">${icon('copy', 13)} Copiar</button>
      </div>
    </div>
    <div class="bridge-field">
      <label>Token de acceso</label>
      <div class="bridge-copy-row">
        <code data-bridge-token>${escapeHtml(status.token || 'No disponible')}</code>
        <button class="btn btn-small" data-bridge-copy="token">${icon('copy', 13)} Copiar</button>
        <button class="btn btn-small" data-bridge-reveal title="Mostrar u ocultar">${icon('eye', 13)}</button>
      </div>
      <span class="bridge-hint">Solo se escucha en 127.0.0.1. Guarda el token en la variable <code>openCUP_AI_TOKEN</code>.</span>
    </div>
    <div class="bridge-field">
      <label>Ejemplo para otra IA</label>
      <div class="bridge-copy-row">
        <pre class="bridge-code">${escapeHtml(EXAMPLE)}</pre>
        <button class="btn btn-small" data-bridge-copy="example">${icon('copy', 13)} Copiar</button>
      </div>
    </div>
    <div class="bridge-docs-head">
      <span>Acciones disponibles</span>
      <a href="${endpoint}/api/ai/actions?token=${encodeURIComponent(status.token || '')}" target="_blank" rel="noreferrer">Abrir documentación JSON</a>
    </div>
    <div class="bridge-actions">
      ${AI_ACTIONS.map((item) => `<div class="bridge-action"><code>${item.action}</code><span>${escapeHtml(item.description)}</span></div>`).join('')}
    </div>`;
}

export async function openAiBridgeDialog() {
  const api = openModal({
    title: 'Puente IA',
    width: 720,
    className: 'bridge-modal',
    body: `<div class="bridge-loading">${icon('link', 22)} Consultando el puente local…</div>`,
    footer: `<button class="btn" data-bridge-refresh>${icon('rotate', 14)} Actualizar</button><button class="btn" data-bridge-docs>Documentación</button><button class="btn btn-primary" data-bridge-close>Listo</button>`,
    onMount(modal) {
      const render = async () => {
        const status = await fetchAiStatus();
        if (!modal.body.isConnected) return;
        modal.body.innerHTML = bridgeMarkup(status);
        modal.body.querySelector('[data-bridge-copy="endpoint"]')?.addEventListener('click', () => {
          const endpoint = modal.body.querySelector('[data-bridge-endpoint]')?.textContent || '';
          void copyText(endpoint).then((ok) => toast(ok ? 'Endpoint copiado' : 'No se pudo copiar', ok ? 'success' : 'error', 1500));
        });
        modal.body.querySelector('[data-bridge-copy="token"]')?.addEventListener('click', () => {
          const token = modal.body.querySelector('[data-bridge-token]')?.textContent || '';
          void copyText(token).then((ok) => toast(ok ? 'Token copiado' : 'No se pudo copiar', ok ? 'success' : 'error', 1500));
        });
        modal.body.querySelector('[data-bridge-copy="example"]')?.addEventListener('click', () => {
          const example = EXAMPLE.replace(/TOKEN/g, status.token || 'TOKEN');
          void copyText(example).then((ok) => toast(ok ? 'Ejemplo copiado con el token' : 'No se pudo copiar', ok ? 'success' : 'error', 1800));
        });
        const tokenEl = modal.body.querySelector('[data-bridge-token]');
        const reveal = modal.body.querySelector('[data-bridge-reveal]');
        if (tokenEl && reveal) {
          const full = tokenEl.textContent;
          const masked = '•'.repeat(Math.min(28, full.length));
          tokenEl.textContent = masked;
          reveal.addEventListener('click', () => {
            tokenEl.textContent = tokenEl.textContent === masked ? full : masked;
          });
        }
        modal.body.querySelector('[data-bridge-docs]')?.addEventListener('click', () => {
          window.open(`http://127.0.0.1:${status.port}/api/ai/actions?token=${encodeURIComponent(status.token || '')}`, '_blank', 'noopener');
        });
      };
      modal.foot.querySelector('[data-bridge-close]').addEventListener('click', modal.close);
      modal.foot.querySelector('[data-bridge-refresh]').addEventListener('click', render);
      modal.foot.querySelector('[data-bridge-docs]').addEventListener('click', () => {
        const status = statusCache;
        if (!status?.port) {
          toast('El puente todavía no está disponible', 'warning');
          return;
        }
        window.open(`http://127.0.0.1:${status.port}/api/ai/actions?token=${encodeURIComponent(status.token || '')}`, '_blank', 'noopener');
      });
      void render();
      const timer = setInterval(render, 2500);
      const observer = new MutationObserver(() => {
        if (!document.body.contains(modal.backdrop)) {
          clearInterval(timer);
          observer.disconnect();
        }
      });
      observer.observe(document.body, { childList: true });
    },
  });
  return api;
}

export function installAiBridgeApi() {
  const api = window.openCUP || {};
  api.state = state;
  api.version = '0.4.3';
  api.command = (action, params) => localCommand(action, params);
  api.getState = () => publicState();
  api.actions = AI_ACTIONS;
  api.bridge = fetchAiStatus;
  api.openBridge = openAiBridgeDialog;
  api.capcut = { list: listCapCutProjects, import: importCapCutProject };
  api.library = { filters: FILTERS, effects: EFFECTS, stickers: STICKERS, transitions: TRANSITIONS, avatars: AVATAR_PRESETS, textPresets: TEXT_PRESETS };
  window.openCUP = api;
}
