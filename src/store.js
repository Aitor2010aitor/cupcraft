import { db } from './db.js';
import { clamp, uid, debounce, toast, formatTime, downloadBlob } from './utils.js';
import { FILTER_LINE, FILTER_LINE_CATEGORIES } from './filter-line.js';
import { CAPTION_STYLES } from './library.js';

export { FILTER_LINE_CATEGORIES };

export const RESOLUTIONS = {
  '480p': { width: 854, height: 480 },
  '720p': { width: 1280, height: 720 },
  '1080p': { width: 1920, height: 1080 },
  '4K': { width: 3840, height: 2160 },
};

export const ASPECTS = {
  '16:9': 16 / 9,
  '9:16': 9 / 16,
  '1:1': 1,
  '4:3': 4 / 3,
  '21:9': 21 / 9,
  '4:5': 4 / 5,
};

/* Catálogo de filtros: la línea propia de cupcraft (src/filter-line.js).
   Cada entrada puede llevar `gl` para pintarla con el shader del motor. */
export const FILTERS = {
  none: { label: 'Ninguno', css: 'none', category: 'Esenciales' },
  ...FILTER_LINE,
};

export const TEXT_PRESETS = [
  { label: 'Título', content: 'Tu título aquí', size: 96, weight: 800, color: '#ffffff' },
  { label: 'Subtítulo', content: 'Un subtítulo para tu historia', size: 60, weight: 600, color: '#ffffff' },
  { label: 'Destacado', content: 'DESTACADO', size: 82, weight: 900, color: '#00e5d0' },
  { label: 'Cita', content: '“Cada gran vídeo empieza con una idea.”', size: 52, weight: 500, color: '#ffffff' },
  { label: 'Minimal', content: 'Menos es más', size: 48, weight: 400, color: '#f2f2f2' },
  { label: 'Impacto', content: 'ENORME', size: 118, weight: 900, color: '#ffffff' },
  { label: 'Neón', content: 'MODO NOCTURNO', size: 66, weight: 800, color: '#8ffcff' },
  { label: 'Nota', content: 'Una nota rápida', size: 44, weight: 500, color: '#ffe34d' },
];

/* Valores iniciales del texto. Los campos siguen la estructura que usa el
   material de texto de CapCut (letter_spacing, border_width, text_curve…)
   para que la importación y el panel compartan el mismo modelo. */
export const TEXT_DEFAULTS = {
  size: 72,
  weight: 800,
  color: '#ffffff',
  align: 'center',
  font: 'Inter',
  x: 0,
  y: 0,
  letterSpacing: 0,
  lineSpacing: 1.18,
  bold: false,
  italic: false,
  underline: false,
  textCase: 'none',
  opacity: 1,
  shadow: true,
  shadowColor: '#000000',
  shadowAlpha: 0.58,
  shadowBlur: 0.2,
  shadowDistance: 0.05,
  shadowAngle: 90,
  strokeColor: '#000000',
  strokeWidth: 0,
  strokeAlpha: 1,
  strokeMode: 'outer',
  strokeSmoothing: 0,
  background: 'transparent',
  backgroundAlpha: 1,
  backgroundStyle: 'rounded',
  backgroundRadius: 0.16,
  backgroundPadding: 0.22,
  backgroundBorderColor: '#000000',
  backgroundBorderWidth: 0,
  glow: false,
  glowColor: '#00e5d0',
  glowBlur: 0.35,
  curve: 0,
  flipX: false,
  flipY: false,
  animation: 'none',
  animationPhase: 'in',
  animationDuration: 0.6,
};

export const state = {
  view: 'home',
  booted: false,
  projects: [],
  project: null,
  media: new Map(),
  selection: null,
  panel: 'media',
  playing: false,
  playhead: 0,
  zoom: 78,
  snap: true,
  lastSaved: 0,
  saving: false,
  history: [],
  future: [],
  pendingFiles: 0,
  importProgress: null, // { current, total, fileName, startedAt, done }
};

const subscribers = new Map();

export function on(reason, fn) {
  if (!subscribers.has(reason)) subscribers.set(reason, new Set());
  subscribers.get(reason).add(fn);
  return () => subscribers.get(reason)?.delete(fn);
}

export function emit(reason, data) {
  subscribers.get(reason)?.forEach((fn) => fn(data));
  if (reason !== '*') subscribers.get('*')?.forEach((fn) => fn({ reason, data }));
}

export function createProject(name = 'Proyecto sin título') {
  return {
    id: uid('prj'),
    name,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    ratio: '16:9',
    resolution: '1080p',
    fps: 30,
    media: [],
    thumb: null,
    tracks: [
      { id: 'track_text', type: 'text', name: 'Texto', clips: [] },
      { id: 'track_video', type: 'video', name: 'Video 1', clips: [] },
      { id: 'track_audio', type: 'audio', name: 'Audio 1', clips: [] },
    ],
  };
}

export function addTrack(type = 'video', { history = true } = {}) {
  if (!state.project) return null;
  const before = snapshot();
  const count = state.project.tracks.filter((track) => track.type === type).length;
  const labels = { video: 'Vídeo', text: 'Texto', audio: 'Audio', overlay: 'Superposición' };
  const base = labels[type] || 'Capa';
  let name = count === 0 ? base : `${base} ${count + 1}`;
  while (state.project.tracks.some((track) => track.name === name)) name = `${name} ${count + 2}`;
  const track = { id: uid('track'), type, name, clips: [] };
  state.project.tracks.push(track);
  if (history) pushHistory(before);
  queueSave();
  emit('project');
  toast(`Pista “${track.name}” añadida`, 'success', 1800);
  return track;
}

export function projectDuration() {
  if (!state.project) return 0;
  let end = 0;
  for (const track of state.project.tracks) {
    for (const clip of track.clips) end = Math.max(end, clip.start + clip.duration);
  }
  return end;
}

export function timelineLength() {
  return Math.max(10, projectDuration() + 4);
}

export function trackById(id) {
  return state.project?.tracks.find((track) => track.id === id) || null;
}

export function findClip(id) {
  if (!state.project || !id) return null;
  for (const track of state.project.tracks) {
    const clip = track.clips.find((item) => item.id === id);
    if (clip) return { clip, track };
  }
  return null;
}

export function selectedClip() {
  return findClip(state.selection);
}

export function trackEnd(track, excludeId = null) {
  return track.clips.reduce((end, clip) => (
    clip.id === excludeId ? end : Math.max(end, clip.start + clip.duration)
  ), 0);
}

function stripMedia(media) {
  const { blob, url, ...meta } = media;
  return meta;
}

function mediaList() {
  return Array.from(state.media.values()).map(stripMedia);
}

function snapshot() {
  return JSON.stringify(state.project);
}

function pushHistory(before) {
  if (!before) return;
  state.history.push(before);
  if (state.history.length > 80) state.history.shift();
  state.future.length = 0;
}

function mutate(fn, { reason = 'project', history = true } = {}) {
  if (!state.project) return undefined;
  const before = history ? snapshot() : null;
  const result = fn();
  if (result === false) return false;
  if (history) pushHistory(before);
  state.project.updatedAt = Date.now();
  if (!state.project.thumb) {
    const first = state.media.values().next().value;
    if (first?.thumb) state.project.thumb = first.thumb;
  }
  queueSave();
  emit(reason);
  if (reason !== 'selection') emit('selection');
  return result;
}

export const queueSave = debounce(async () => {
  await saveNow();
}, 650);

export async function saveNow() {
  if (!state.project) return;
  state.saving = true;
  emit('saving');
  const payload = {
    ...state.project,
    media: mediaList(),
    updatedAt: Date.now(),
  };
  state.project.updatedAt = payload.updatedAt;
  try {
    await db.saveProject(JSON.parse(JSON.stringify(payload)));
    state.lastSaved = Date.now();
    state.projects = await db.listProjects();
    emit('projects');
  } catch (error) {
    console.error(error);
    toast('No se pudo guardar el proyecto', 'error');
  } finally {
    state.saving = false;
    emit('saving');
  }
}

export async function refreshProjects() {
  state.projects = await db.listProjects();
  emit('projects');
}

export async function initStore() {
  await refreshProjects();
  state.booted = true;
  emit('booted');
}

export async function newProject(name = 'Proyecto sin título') {
  const project = createProject(name || 'Proyecto sin título');
  state.project = project;
  state.media = new Map();
  state.selection = null;
  state.playhead = 0;
  state.playing = false;
  state.history = [];
  state.future = [];
  state.panel = 'media';
  state.view = 'editor';
  state.projects = [project, ...state.projects.filter((item) => item.id !== project.id)];
  await db.saveProject(JSON.parse(JSON.stringify({ ...project, media: [] })));
  emit('view');
  emit('project');
}

export async function openProject(id) {
  const project = await db.getProject(id);
  if (!project) {
    toast('El proyecto ya no existe', 'error');
    await refreshProjects();
    return;
  }
  const media = new Map();
  for (const meta of project.media || []) {
    const record = await db.getMedia(meta.id);
    media.set(meta.id, {
      ...meta,
      blob: record?.blob || null,
      url: record?.blob ? URL.createObjectURL(record.blob) : '',
    });
  }
  state.project = project;
  state.media = media;
  state.selection = null;
  state.playhead = 0;
  state.playing = false;
  state.history = [];
  state.future = [];
  state.zoom = 78;
  state.view = 'editor';
  emit('view');
  emit('project');
}

const PROJECT_FILE_KIND = 'openCUP-project';
const PROJECT_FILE_VERSION = 1;
const PROJECT_FILE_EMBED_LIMIT = 256 * 1024 * 1024;

function blobToDataURL(blob) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error || new Error('No se pudo leer el medio'));
    reader.readAsDataURL(blob);
  });
}

function dataUrlToBlob(dataUrl, fallbackMime = '') {
  const match = /^data:([^;,]+)?(;base64)?,([\s\S]*)$/.exec(String(dataUrl));
  if (!match) throw new Error('Medio corrupto dentro del archivo de proyecto');
  const mime = match[1] || fallbackMime || 'application/octet-stream';
  const payload = match[3];
  if (!match[2]) return new Blob([decodeURIComponent(payload)], { type: mime });
  const binary = atob(payload);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return new Blob([bytes], { type: mime });
}

/* Guarda el proyecto como archivo descargable (.opencup.json),
   incluyendo los medios incrustados cuando no superan el límite. */
export async function exportProjectFile(sourceProject = null) {
  const isCurrent = !sourceProject;
  const project = sourceProject || state.project;
  if (!project) throw new Error('No hay ningún proyecto abierto');
  const metas = isCurrent ? mediaList() : (project.media || []);
  const media = [];
  let embeddedBytes = 0;
  let skipped = 0;
  for (const meta of metas) {
    const entry = { ...meta };
    const blob = isCurrent
      ? state.media.get(meta.id)?.blob || null
      : (await db.getMedia(meta.id))?.blob || null;
    if (blob && blob.size && embeddedBytes + blob.size <= PROJECT_FILE_EMBED_LIMIT) {
      entry.mime = blob.type || '';
      entry.data = await blobToDataURL(blob);
      embeddedBytes += blob.size;
    } else if (blob) {
      skipped += 1;
    }
    media.push(entry);
  }
  const cleanProject = JSON.parse(JSON.stringify(project));
  delete cleanProject.media;
  const payload = {
    kind: PROJECT_FILE_KIND,
    fileVersion: PROJECT_FILE_VERSION,
    app: 'openCUP',
    exportedAt: Date.now(),
    project: cleanProject,
    media,
  };
  const base = String(project.name || 'proyecto')
    .replace(/[\\/:*?"<>|]+/g, '-')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 60) || 'proyecto';
  downloadBlob(new Blob([JSON.stringify(payload)], { type: 'application/json' }), `${base}.opencup.json`);
  return { media: media.length, embedded: media.length - skipped, skipped, bytes: embeddedBytes };
}

/* Abre un proyecto guardado como archivo y lo añade a la biblioteca local. */
export async function importProjectFile(file) {
  if (!file) throw new Error('No se recibió ningún archivo');
  let payload = null;
  try {
    payload = JSON.parse(await file.text());
  } catch {
    payload = null;
  }
  if (!payload || payload.kind !== PROJECT_FILE_KIND || !payload.project?.tracks) {
    throw new Error('El archivo no es un proyecto válido de cupcraft');
  }
  queueSave.cancel();
  const project = payload.project;
  const metas = [];
  let missing = 0;
  for (const entry of payload.media || []) {
    const { data, mime, ...meta } = entry;
    if (!meta?.id) continue;
    metas.push(meta);
    if (!data) {
      missing += 1;
      continue;
    }
    try {
      await db.saveMedia({ id: meta.id, blob: dataUrlToBlob(data, mime), meta });
    } catch (error) {
      console.warn('Medio ilegible en el archivo', error);
      missing += 1;
    }
  }
  const record = { ...project, media: metas, updatedAt: Date.now() };
  await db.saveProject(JSON.parse(JSON.stringify(record)));
  await refreshProjects();
  await openProject(record.id);
  if (missing) {
    toast(`${missing} medio(s) venían sin archivo y se omitieron`, 'warning', 4200);
  }
  return { id: record.id, name: record.name, media: metas.length, missing };
}

export function goHome() {
  if (state.playing) pause();
  state.view = 'home';
  state.playing = false;
  queueSave.flush();
  emit('view');
}

export async function deleteProject(id) {
  const project = await db.getProject(id);
  if (project) {
    for (const media of project.media || []) await db.deleteMedia(media.id);
    await db.deleteProject(id);
  }
  if (state.project?.id === id) {
    state.project = null;
    state.media = new Map();
    state.view = 'home';
    emit('view');
  }
  await refreshProjects();
}

export async function renameProject(id, name) {
  const project = state.project?.id === id ? state.project : await db.getProject(id);
  if (!project) return;
  project.name = name;
  project.updatedAt = Date.now();
  await db.saveProject(JSON.parse(JSON.stringify(project)));
  await refreshProjects();
  if (state.project?.id === id) {
    state.project.name = name;
    emit('project');
  }
}

/* ----------------------------- Media import ----------------------------- */

function waitForEvent(target, event, timeout = 20000) {
  return new Promise((resolve, reject) => {
    let timer = 0;
    const cleanup = () => {
      target.removeEventListener(event, onOk);
      target.removeEventListener('error', onErr);
      clearTimeout(timer);
    };
    const onOk = () => { cleanup(); resolve(); };
    const onErr = () => { cleanup(); reject(new Error('No se pudo leer el archivo')); };
    timer = setTimeout(() => { cleanup(); reject(new Error('Tiempo de espera agotado')); }, timeout);
    target.addEventListener(event, onOk, { once: true });
    target.addEventListener('error', onErr, { once: true });
  });
}

function drawThumb(source, width, height, sw, sh) {
  const canvas = document.createElement('canvas');
  canvas.width = 320;
  canvas.height = 180;
  const ctx = canvas.getContext('2d');
  ctx.fillStyle = '#111';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  const scale = Math.max(canvas.width / (sw || width), canvas.height / (sh || height));
  const dw = (sw || width) * scale;
  const dh = (sh || height) * scale;
  ctx.drawImage(source, (canvas.width - dw) / 2, (canvas.height - dh) / 2, dw, dh);
  return canvas.toDataURL('image/jpeg', 0.72);
}

async function computeAudioInfo(blob, buckets = 220) {
  try {
    const arrayBuffer = await blob.arrayBuffer();
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    if (!AudioCtx) return null;
    const ctx = new AudioCtx();
    const buffer = await ctx.decodeAudioData(arrayBuffer.slice(0));
    const channel = buffer.getChannelData(0);
    const step = Math.max(1, Math.floor(channel.length / buckets));
    const peaks = [];
    for (let i = 0; i < buckets; i += 1) {
      let peak = 0;
      const from = i * step;
      const to = Math.min(channel.length, from + step);
      for (let j = from; j < to; j += 1) peak = Math.max(peak, Math.abs(channel[j]));
      peaks.push(Number(peak.toFixed(3)));
    }
    await ctx.close();
    return { peaks, duration: buffer.duration };
  } catch (error) {
    console.warn('No se pudo calcular la forma de onda', error);
    return null;
  }
}

async function analyzeFile(file) {
  if (file.size > 2 * 1024 * 1024 * 1024) {
    throw new Error('El archivo supera los 2 GB');
  }
  const mime = file.type || '';
  const name = file.name || 'archivo';
  let kind = null;
  if (mime.startsWith('video/')) kind = 'video';
  else if (mime.startsWith('audio/')) kind = 'audio';
  else if (mime.startsWith('image/')) kind = 'image';
  if (!kind) throw new Error('Formato no soportado');

  const url = URL.createObjectURL(file);
  const base = {
    id: uid('med'),
    name,
    kind,
    mime,
    size: file.size,
    duration: 0,
    width: 0,
    height: 0,
    thumb: null,
    peaks: null,
  };

  if (kind === 'image') {
    const img = new Image();
    img.src = url;
    await img.decode().catch(() => waitForEvent(img, 'load'));
    base.width = img.naturalWidth;
    base.height = img.naturalHeight;
    base.duration = 0;
    base.thumb = drawThumb(img, base.width, base.height, base.width, base.height);
  } else if (kind === 'video') {
    const video = document.createElement('video');
    video.preload = 'auto';
    video.muted = true;
    video.src = url;
    await waitForEvent(video, 'loadeddata');
    base.duration = Number.isFinite(video.duration) ? video.duration : 0;
    base.width = video.videoWidth;
    base.height = video.videoHeight;
    try {
      video.currentTime = Math.min(Math.max(0.1, base.duration * 0.1), Math.max(0, base.duration - 0.1));
      await waitForEvent(video, 'seeked', 8000);
    } catch { /* frame 0 */ }
    base.thumb = drawThumb(video, base.width, base.height, video.videoWidth, video.videoHeight);
    video.removeAttribute('src');
    video.load();
  } else {
    const audio = document.createElement('audio');
    audio.preload = 'metadata';
    audio.src = url;
    await waitForEvent(audio, 'loadedmetadata');
    base.duration = Number.isFinite(audio.duration) ? audio.duration : 0;
    base.thumb = null;
    const audioInfo = await computeAudioInfo(file);
    base.peaks = audioInfo?.peaks || null;
    if ((!base.duration || !Number.isFinite(base.duration)) && audioInfo?.duration) {
      base.duration = audioInfo.duration;
    }
    audio.removeAttribute('src');
    audio.load();
  }
  base.url = url;
  base.blob = file;
  return base;
}

export async function importFiles(fileList, { addToTimeline = true, silent = false } = {}) {
  const files = Array.from(fileList || []);
  if (!files.length || !state.project) return [];
  const before = snapshot();
  const imported = [];
  state.pendingFiles += files.length;
  state.importProgress = { current: 0, total: files.length, fileName: '', startedAt: Date.now() };
  emit('importing');
  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    state.importProgress = { ...state.importProgress, current: i, fileName: file.name };
    emit('importing');
    try {
      if (!silent) toast(`Procesando ${file.name}…`, 'info', 1600);
      const meta = await analyzeFile(file);
      await db.saveMedia({ id: meta.id, blob: file, meta: stripMedia(meta) });
      state.media.set(meta.id, meta);
      state.project.media = mediaList();
      imported.push(meta);
    } catch (error) {
      console.error(error);
      toast(`${file.name}: ${error.message || 'error al importar'}`, 'error', 4200);
    } finally {
      state.pendingFiles -= 1;
      state.importProgress = { ...state.importProgress, current: i + 1, fileName: file.name };
      emit('importing');
    }
  }
  state.importProgress = null;
  emit('importing');
  if (imported.length) {
    pushHistory(before);
    if (!state.project.thumb && imported[0]?.thumb) state.project.thumb = imported[0].thumb;
    if (addToTimeline) imported.forEach((media) => addMediaToTimeline(media.id, { history: false }));
    state.project.media = mediaList();
    state.project.updatedAt = Date.now();
    queueSave();
    emit('project');
    emit('selection');
    emit('panel');
  }
  return imported;
}

export async function importExternalMedia(file, { silent = true } = {}) {
  if (!state.project) throw new Error('No hay proyecto abierto');
  if (!silent) toast(`Procesando ${file.name}…`, 'info', 1600);
  const meta = await analyzeFile(file);
  await db.saveMedia({ id: meta.id, blob: file, meta: stripMedia(meta) });
  state.media.set(meta.id, meta);
  state.project.media = mediaList();
  if (!state.project.thumb && meta.thumb) state.project.thumb = meta.thumb;
  state.project.updatedAt = Date.now();
  queueSave();
  emit('project');
  return meta;
}

/* ------------------------------- Timeline ------------------------------- */

export function addMediaToTimeline(mediaId, { trackId, start, history = true } = {}) {
  const media = state.media.get(mediaId);
  if (!media || !state.project) return null;
  const type = media.kind === 'audio' ? 'audio' : 'video';
  const requested = trackId ? trackById(trackId) : null;
  const track = requested?.type === type
    ? requested
    : state.project.tracks.find((item) => item.type === type);
  if (!track) return null;
  const duration = media.kind === 'image' ? 5 : Math.max(0.1, media.duration || 5);
  const desired = start ?? trackEnd(track);
  const finalStart = resolveStart(track, null, desired, duration);
  const clip = {
    id: uid('clip'),
    mediaId: media.id,
    kind: media.kind,
    name: media.name,
    start: finalStart,
    duration,
    in: 0,
    speed: 1,
    volume: 1,
    opacity: 1,
    scale: 1,
    x: 0,
    y: 0,
    filter: 'none',
    adjust: { brightness: 100, contrast: 100, saturate: 100 },
    fadeIn: 0,
    fadeOut: 0,
  };
  if (history) {
    mutate(() => { track.clips.push(clip); });
  } else {
    track.clips.push(clip);
  }
  selectClip(clip.id);
  return clip;
}

export function addTextClip(content = 'Tu texto aquí', options = {}) {
  if (!state.project) return null;
  let track = state.project.tracks.find((item) => item.type === 'text');
  if (!track) return null;
  const newTrack = options.newTrack
    ? {
        id: uid('track'),
        type: 'text',
        name: `Texto ${state.project.tracks.filter((item) => item.type === 'text').length + 1}`,
        clips: [],
      }
    : null;
  if (newTrack) track = newTrack;
  const duration = options.duration || 5;
  const start = options.allowOverlap
    ? Math.max(0, options.start ?? state.playhead)
    : resolveStart(track, null, options.start ?? state.playhead, duration);
  const preset = options.preset || {};
  const style = { ...TEXT_DEFAULTS, ...(preset.text || {}) };
  const text = {
    ...style,
    content,
    size: preset.size ?? style.size,
    weight: preset.weight ?? style.weight,
    color: preset.color ?? style.color,
    align: preset.align ?? style.align,
    font: preset.font ?? style.font,
    x: preset.x ?? style.x,
    y: preset.y ?? style.y,
    shadow: preset.shadow ?? style.shadow,
    background: preset.background ?? style.background,
  };
  if (preset.radius !== undefined) text.backgroundRadius = preset.radius;
  if (options.words) text.words = options.words;
  const clip = {
    id: uid('clip'),
    kind: 'text',
    mediaId: null,
    name: content.slice(0, 22),
    start,
    duration,
    in: 0,
    speed: 1,
    volume: 1,
    opacity: 1,
    scale: 1,
    x: 0,
    y: 0,
    filter: 'none',
    adjust: { brightness: 100, contrast: 100, saturate: 100 },
    fadeIn: 0,
    fadeOut: 0,
    text,
  };
  const addToTrack = () => {
    if (newTrack) {
      const textTracks = state.project.tracks.filter((item) => item.type === 'text');
      const anchor = textTracks[textTracks.length - 1];
      const index = anchor ? state.project.tracks.indexOf(anchor) + 1 : 0;
      state.project.tracks.splice(index, 0, newTrack);
    }
    track.clips.push(clip);
  };
  if (options.history === false) {
    addToTrack();
  } else {
    mutate(addToTrack);
  }
  selectClip(clip.id);
  return clip;
}

export function addSubtitleClips(items, { captionStyleId = 'clean', replace = false } = {}) {
  if (!state.project || !items || !items.length) return [];
  const style = CAPTION_STYLES.find((item) => item.id === captionStyleId) || CAPTION_STYLES[0];
  let track = state.project.tracks.find((item) => item.type === 'text');
  if (!track) {
    track = {
      id: uid('track'),
      type: 'text',
      name: 'Subtítulos',
      clips: [],
    };
    state.project.tracks.push(track);
  }

  const added = [];
  mutate(() => {
    if (replace) {
      track.clips = track.clips.filter((c) => c.kind !== 'text');
    }
    for (const item of items) {
      const content = String(item.text || '').trim();
      if (!content) continue;
      const start = Math.max(0, Number(item.start) || 0);
      const duration = Math.max(0.4, Number(item.duration || (item.end - item.start)) || 2);
      const text = {
        ...TEXT_DEFAULTS,
        content,
        size: style.size ?? 56,
        weight: style.weight ?? 600,
        color: style.color ?? '#ffffff',
        background: style.background ?? 'transparent',
        shadow: style.shadow ?? true,
        highlight: style.highlight ?? '',
        align: 'center',
        y: 28,
      };
      if (item.words && item.words.length) {
        text.words = item.words;
      }
      const clip = {
        id: uid('clip'),
        kind: 'text',
        mediaId: null,
        name: content.slice(0, 24),
        start,
        duration,
        in: 0,
        speed: 1,
        volume: 1,
        opacity: 1,
        scale: 1,
        x: 0,
        y: 0,
        filter: 'none',
        adjust: { brightness: 100, contrast: 100, saturate: 100 },
        fadeIn: 0,
        fadeOut: 0,
        text,
      };
      track.clips.push(clip);
      added.push(clip);
    }
  });

  if (added.length) {
    selectClip(added[0].id);
  }
  return added;
}

export function clearSubtitleClips() {
  if (!state.project) return 0;
  let track = state.project.tracks.find((item) => item.type === 'text');
  if (!track) return 0;
  const count = track.clips.filter((c) => c.kind === 'text').length;
  if (!count) return 0;
  mutate(() => {
    track.clips = track.clips.filter((c) => c.kind !== 'text');
    state.selection = null;
  });
  return count;
}

export function addStickerClip(content = '✨', options = {}) {
  if (!state.project) return null;
  const track = state.project.tracks.find((item) => item.type === 'text');
  if (!track) return null;
  const duration = options.duration || 4;
  const start = resolveStart(track, null, options.start ?? state.playhead, duration);
  const preset = options.preset || {};
  const clip = {
    id: uid('clip'),
    kind: 'sticker',
    mediaId: null,
    name: String(content || 'Sticker'),
    start,
    duration,
    in: 0,
    speed: 1,
    volume: 1,
    opacity: 1,
    scale: options.scale ?? 1,
    x: options.x ?? 0,
    y: options.y ?? 0,
    filter: 'none',
    adjust: { brightness: 100, contrast: 100, saturate: 100 },
    fadeIn: 0,
    fadeOut: 0,
    sticker: {
      content: String(content || '✨'),
      src: preset.src || null,
      srcFallback: preset.srcFallback || null,
      size: preset.size || 220,
      background: preset.background || 'transparent',
      shape: preset.shape || 'none',
      color: preset.color || '#ffffff',
      rotate: options.rotate || 0,
    },
  };
  mutate(() => { track.clips.push(clip); });
  selectClip(clip.id);
  return clip;
}

export function resolveStart(track, clipId, desired, duration) {
  const others = track.clips
    .filter((clip) => clip.id !== clipId)
    .sort((a, b) => a.start - b.start);
  let start = Math.max(0, desired);
  for (let pass = 0; pass < 6; pass += 1) {
    let moved = false;
    for (const other of others) {
      const otherEnd = other.start + other.duration;
      if (start < otherEnd && start + duration > other.start) {
        const preferAfter = desired + duration / 2 > other.start + other.duration / 2;
        start = preferAfter ? otherEnd : Math.max(0, other.start - duration);
        moved = true;
      }
    }
    if (!moved) break;
  }
  return Math.max(0, start);
}

export function updateClip(clipId, patch, { history = true } = {}) {
  const found = findClip(clipId);
  if (!found) return;
  const apply = () => {
    const next = { ...patch };
    if (patch.text) next.text = { ...(found.clip.text || {}), ...patch.text };
    if (patch.sticker) next.sticker = { ...(found.clip.sticker || {}), ...patch.sticker };
    if (patch.adjust) next.adjust = { ...(found.clip.adjust || {}), ...patch.adjust };
    Object.assign(found.clip, next);
    found.clip.start = Math.max(0, found.clip.start);
    found.clip.duration = Math.max(0.1, found.clip.duration);
  };
  if (history) mutate(apply, { reason: 'project' });
  else { apply(); emit('project'); }
}

export function moveClip(clipId, { start, trackId }) {
  const found = findClip(clipId);
  if (!found) return;
  const destination = trackId && trackId !== found.track.id ? trackById(trackId) : found.track;
  if (!destination || destination.type !== found.track.type) return;
  mutate(() => {
    const finalStart = resolveStart(destination, clipId, start, found.clip.duration);
    if (destination.id !== found.track.id) {
      found.track.clips = found.track.clips.filter((clip) => clip.id !== clipId);
      destination.clips.push(found.clip);
    }
    found.clip.start = finalStart;
  });
}

export function trimClip(clipId, edge, deltaSeconds) {
  const found = findClip(clipId);
  if (!found) return;
  const { clip } = found;
  const media = clip.mediaId ? state.media.get(clip.mediaId) : null;
  const sourceDuration = clip.kind === 'image' || clip.kind === 'text' || clip.kind === 'sticker'
    ? Number.POSITIVE_INFINITY
    : Math.max(0.1, (media?.duration || clip.in + clip.duration) - clip.in);
  mutate(() => {
    if (edge === 'left') {
      const nextStart = clamp(clip.start + deltaSeconds, 0, clip.start + clip.duration - 0.1);
      const shift = nextStart - clip.start;
      clip.start = nextStart;
      clip.duration = Math.max(0.1, clip.duration - shift);
      if (clip.kind !== 'image' && clip.kind !== 'text') clip.in = Math.max(0, clip.in + shift * clip.speed);
    } else {
      clip.duration = clamp(clip.duration + deltaSeconds, 0.1, sourceDuration);
    }
  });
}

export function selectClip(clipId) {
  state.selection = clipId || null;
  emit('selection');
  emit('tick');
}

export function deleteSelected() {
  const found = selectedClip();
  if (!found) return;
  mutate(() => {
    found.track.clips = found.track.clips.filter((clip) => clip.id !== found.clip.id);
    state.selection = null;
  });
  emit('selection');
}

export function duplicateSelected() {
  const found = selectedClip();
  if (!found) return;
  mutate(() => {
    const clone = JSON.parse(JSON.stringify(found.clip));
    clone.id = uid('clip');
    clone.start = resolveStart(found.track, null, found.clip.start + found.clip.duration, clone.duration);
    found.track.clips.push(clone);
    state.selection = clone.id;
  });
  emit('selection');
}

export function splitAtPlayhead() {
  if (!state.project) return;
  const selected = selectedClip();
  const targets = selected ? [selected] : state.project.tracks
    .flatMap((track) => track.clips.map((clip) => ({ clip, track })));
  const time = state.playhead;
  const splittable = targets.filter(({ clip }) => (
    time > clip.start + 0.05 && time < clip.start + clip.duration - 0.05
  ));
  if (!splittable.length) {
    toast('Coloca el cursor dentro de un clip para dividirlo', 'warning');
    return;
  }
  mutate(() => {
    for (const { clip, track } of splittable) {
      const right = JSON.parse(JSON.stringify(clip));
      right.id = uid('clip');
      right.start = time;
      right.duration = clip.start + clip.duration - time;
      right.in = clip.in + (time - clip.start) * (clip.speed || 1);
      clip.duration = time - clip.start;
      track.clips.push(right);
    }
  });
}

export function updateProject(patch) {
  if (!state.project) return;
  mutate(() => { Object.assign(state.project, patch); }, { reason: 'project' });
}

/* ----------------------------- Undo / redo ------------------------------ */

export function undo() {
  if (!state.history.length || !state.project) return;
  state.future.push(snapshot());
  state.project = JSON.parse(state.history.pop());
  if (state.selection && !findClip(state.selection)) state.selection = null;
  queueSave();
  emit('project');
  emit('selection');
}

export function redo() {
  if (!state.future.length || !state.project) return;
  state.history.push(snapshot());
  state.project = JSON.parse(state.future.pop());
  if (state.selection && !findClip(state.selection)) state.selection = null;
  queueSave();
  emit('project');
  emit('selection');
}

/* ------------------------------- Playback ------------------------------- */

let rafId = 0;
let wallStart = 0;
let headStart = 0;

export function play() {
  if (!state.project) return;
  const total = projectDuration();
  if (total <= 0.05) {
    toast('Añade material a la línea de tiempo para reproducir', 'warning');
    return;
  }
  if (state.playhead >= total - 0.02) state.playhead = 0;
  state.playing = true;
  wallStart = performance.now();
  headStart = state.playhead;
  emit('playing');
  emit('tick');
  const loop = () => {
    if (!state.playing) return;
    const next = headStart + (performance.now() - wallStart) / 1000;
    const end = projectDuration();
    if (next >= end) {
      state.playhead = end;
      state.playing = false;
      emit('tick');
      emit('playing');
      return;
    }
    state.playhead = next;
    emit('tick');
    rafId = requestAnimationFrame(loop);
  };
  rafId = requestAnimationFrame(loop);
}

export function pause() {
  if (!state.playing) return;
  state.playing = false;
  cancelAnimationFrame(rafId);
  emit('playing');
  emit('tick');
}

export function togglePlay() {
  if (state.playing) pause(); else play();
}

export function seek(time) {
  const total = Math.max(projectDuration(), timelineLength() - 1, 0);
  state.playhead = clamp(time, 0, total || 0);
  if (state.playing) {
    wallStart = performance.now();
    headStart = state.playhead;
  }
  emit('tick');
}

export function stepFrame(direction) {
  const fps = state.project?.fps || 30;
  seek(state.playhead + direction / fps);
}

export function skipToEdge(edge) {
  seek(edge === 'start' ? 0 : projectDuration());
}

export function setZoom(value) {
  state.zoom = clamp(value, 30, 260);
  emit('zoom');
}

export function toggleSnap() {
  state.snap = !state.snap;
  emit('snap');
}

export function setPanel(panel) {
  state.panel = panel;
  emit('panel');
}

export { formatTime };
