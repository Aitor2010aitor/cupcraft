import {
  state,
  on,
  setPanel,
  selectedClip,
  addTextClip,
  clearSubtitleClips,
  addStickerClip,
  addMediaToTimeline,
  updateClip,
  duplicateSelected,
  deleteSelected,
  selectClip,
  updateProject,
  newProject,
  saveNow,
  importFiles,
  openProject,
  goHome,
  projectDuration,
  seek,
  FILTERS,
  CAPCUT_FILTER_CATEGORIES,
  TEXT_DEFAULTS,
  RESOLUTIONS,
  ASPECTS,
} from './store.js';
import {
  transformAt,
  hasKeyframes,
  keyframeCount,
  keyframeIndexAt,
  keyframeValue,
  nextKeyframeStore,
  withoutKeyframe,
  neighborKeyframe,
} from './keyframes.js';
import { clipLayerBox } from './renderer.js';
import { icon } from './icons.js';
import { openModal, confirmDialog, promptDialog } from './modal.js';
import { openExportDialog } from './export.js';
import { openShortcutsDialog } from './dialogs.js';
import { TEMPLATES } from './templates.js';
import {
  isKokoroSpanishVoice,
  loadKokoro,
  generateKokoroAudio,
} from './kokoro-tts.js';
import {
  STICKERS,
  STICKER_CATEGORIES,
  EFFECTS,
  EFFECT_CATEGORIES,
  TRANSITIONS,
  TRANSITION_FADES,
  CAPTION_STYLES,
  AVATAR_PRESETS,
  ADJUST_PRESETS,
  ADJUST_CATEGORIES,
} from './library.js';
import {
  CAPCUT_EFFECT_ITEMS,
  CAPCUT_EFFECT_CATEGORIES,
  CAPCUT_FILTER_ITEMS,
  CAPCUT_FILTER_CATEGORIES as CAPCUT_FILTER_PANEL_CATEGORIES,
  CAPCUT_TRANSITION_ITEMS,
  CAPCUT_TRANSITION_CATEGORIES as CAPCUT_TRANSITION_PANEL_CATEGORIES,
  CAPCUT_STICKER_ITEMS,
  CAPCUT_STICKER_CATEGORIES,
  CAPCUT_TEXT_ITEMS,
  CAPCUT_TEXT_CATEGORIES,
} from './capcut-panels.js';
import { capcutEffectToGL } from './gl-post.js';
import { prepareStickerSrc } from './sticker-image.js';

const CAPCUT_TRANSITION_MAP = [
  [/negro/i, 'black'],
  [/blanco/i, 'white'],
  [/fundid|fade/i, 'fade'],
  [/desliz|slide|barrid/i, 'slide-left'],
  [/acerc|zoom in/i, 'zoom-in'],
  [/alej|zoom out/i, 'zoom-out'],
  [/gir|rotac|spin/i, 'spin'],
  [/desenfoq|blur/i, 'blur'],
  [/flash|destello/i, 'flash'],
  [/disolv|dissolv/i, 'dissolve'],
  [/cortinilla|wipe/i, 'wipe-left'],
];

function capcutTransitionToLocal(name) {
  const hit = CAPCUT_TRANSITION_MAP.find(([re]) => re.test(name));
  return hit ? hit[1] : 'fade';
}

function capcutFilterToLocal(item) {
  const sameCategory = Object.entries(FILTERS).find(([key, value]) => key !== 'none' && value.category === item.cat);
  if (sameCategory) return sameCategory[0];
  const words = String(item.name || '').toLowerCase().split(/\s+/).filter((w) => w.length > 4);
  const byName = Object.entries(FILTERS).find(([key, value]) => key !== 'none'
    && words.some((w) => value.label.toLowerCase().includes(w)));
  return byName ? byName[0] : 'none';
}

function capcutCard(item, attr) {
  const fallback = item.url ? ` onerror="if(!this.dataset.fb){this.dataset.fb=1;this.src='${item.url.replace(/'/g, '%27')}'}else{this.onerror=null}"` : '';
  return `<button class="effect-card" ${attr}="${item.id}" data-capcut-name="${escapeHtml(item.name)}" title="${escapeHtml(item.name)}">
              <img class="effect-thumb capcut" src="${item.img}"${fallback} alt="${escapeHtml(item.name)}" loading="lazy" decoding="async" />
              <div class="filter-name"><span>${escapeHtml(item.name)}</span></div>
            </button>`;
}

const CAPCUT_LOCAL_MAP = [
  [/desenfoque|blur|bokeh|difumin|suaviz/i, 'esp_dreamy'],
  [/glitch|falla|error|distorsi|corrupt/i, 'tnd_horror'],
  [/mosaico|mosaic|pixelad|pixel/i, 'tnd_mosaic'],
  [/sepia|vintage|retro|vhs|film/i, 'ret_vhs'],
  [/zoom|pulso|latido/i, 'zoomPulse'],
  [/temblor|shake|vibra|tremor/i, 'tnd_retroShake'],
  [/brillo|glow|flash|neon|luz/i, 'luz_flash'],
  [/coraz|heart|amor/i, 'tnd_heart'],
  [/confeti|confetti|glitter|chispa|particul/i, 'fst_confetti'],
  [/garabato|doodle|dibuj/i, 'tnd_doodle'],
  [/niebla|fog|humo|smoke|vapor/i, 'hal_fog'],
  [/sangre|blood/i, 'hal_blood'],
  [/calabaza|pumpkin|halloween/i, 'hal_pumpkin'],
  [/fantasma|ghost/i, 'hal_ghost'],
  [/cromatic|chroma|rgb|aberraci/i, 'tnd_chroma'],
  [/grano|grain|ruido|noise/i, 'grain'],
  [/vi[nñ]eta|vignette/i, 'vignette'],
];

function capcutToLocal(name) {
  const hit = CAPCUT_LOCAL_MAP.find(([re]) => re.test(name));
  return hit ? hit[1] : 'none';
}
import { adjustCss } from './renderer.js';
import {
  TEXT_STYLES,
  TEXT_BUBBLES,
  TEXT_EFFECTS,
  TEXT_ANIMATION_GROUPS,
  TEXT_ANIMATIONS,
  TEXT_FONTS,
  getFontList,
  loadSystemFonts,
  TEXT_CASES,
  TEXT_ALIGNS,
  TEXT_WEIGHTS,
} from './text-catalog.js';
import { openCapCutImportDialog } from './capcut.js';
import { autoEditPanelHtml, wireAutoEditPanel, isWhisperModelReady, ensureWhisperPipeline, generateSubtitlesForVideo } from './auto-edit.js';
import { fixWebmDuration } from './webm.js';
import { escapeHtml, formatTime, formatBytes, toast, clamp, uid } from './utils.js';

const PANEL_TITLES = {
  media: 'Archivos multimedia',
  audio: 'Audio',
  text: 'Texto',
  stickers: 'Stickers',
  effects: 'Efectos',
  transitions: 'Transiciones',
  captions: 'Subtítulos',
  filters: 'Filtros',
  adjust: 'Ajuste',
  templates: 'Plantillas',
  avatar: 'Avatar de IA',
  auto: 'Editor automático con IA',
};

const PANEL_HELP = {
  media: 'Importa videos, imágenes y audio desde tu equipo. También puedes arrastrarlos directamente al panel o a la línea de tiempo. Los archivos se guardan en el almacenamiento local del navegador.',
  audio: 'Añade música y efectos de sonido, o graba tu voz con el micrófono. El volumen y los fundidos se ajustan desde el panel Detalles.',
  text: 'Añade títulos y subtítulos. Haz clic en un estilo para insertarlo en el cursor y edítalo desde el panel Detalles.',
  stickers: 'Stickers organizados por categorías. Se insertan como capas independientes que puedes escalar, mover, rotar y animar con fundidos.',
  effects: 'Efectos visuales en tiempo real: desenfoque, luz, grano, viñeta, espejo, vibración y más. Puedes combinarlos con filtros y ajustes.',
  transitions: 'Transiciones de entrada para el clip seleccionado. Se aplican al inicio y se mezclan con el clip anterior de la misma pista.',
  captions: 'Añade subtítulos manuales con estilos, o importa un archivo SRT. La transcripción automática no está disponible sin conexión.',
  filters: 'Los filtros cambian el color del clip seleccionado. Se agrupan por familias y se incluyen al exportar.',
  adjust: 'Ajusta brillo, contraste, saturación, opacidad, escala y rotación del clip seleccionado.',
  templates: 'Crea un proyecto nuevo con una composición inicial de texto y relación de aspecto. Después añade tus propios medios.',
  avatar: 'Añade un avatar local como capa con fondo de color o sube una foto. La generación de avatares animados con IA requiere conexión.',
  auto: 'Analiza el material, interpreta tu instrucción y monta la timeline real. Usa modelos locales (Ollama) o externos (OpenAI) como opción.',
};

const arrayBufferToFile = (blob, name) => new File([blob], name, { type: blob.type || 'audio/webm' });

function mediaCard(media) {
  const badge = media.kind === 'video' ? 'VIDEO' : media.kind === 'audio' ? 'AUDIO' : 'IMAGEN';
  const thumb = media.thumb
    ? `<img src="${media.thumb}" alt="" draggable="false" />`
    : `<span class="thumb-empty">${icon(media.kind === 'audio' ? 'music' : 'image', 26)}</span>`;
  const duration = media.kind === 'image' ? 'Imagen' : formatTime(media.duration || 0);
  return `
    <button class="media-card" draggable="true" data-media-id="${media.id}" data-name="${escapeHtml((media.name || '').toLowerCase())}" title="${escapeHtml(media.name || '')}">
      ${thumb}
      <span class="media-badge">${badge}</span>
      <span class="media-add">${icon('plus', 12)}</span>
      <span class="media-name">${escapeHtml(media.name || 'Archivo')}</span>
      <span class="media-duration">${duration}</span>
    </button>`;
}

function panelEmpty(title, text, iconName = 'layers') {
  return `<div class="panel-empty">${icon(iconName, 28)}<strong>${escapeHtml(title)}</strong>${escapeHtml(text)}</div>`;
}

let filterSearch = '';
let effectSearch = '';
let effectSource = 'capcut';
let capcutCategory = 'Todos';
let capcutSearch = '';
let stickerSearch = '';
let filterSource = 'capcut';
let capcutFilterCategory = 'Todos';
let capcutFilterSearch = '';
let transitionSource = 'capcut';
let capcutTransitionCategory = 'Todas';
let capcutTransitionSearch = '';
let stickerSource = 'capcut';
let capcutStickerCategory = 'Todos';
let capcutStickerSearch = '';
let capcutTextCategory = 'Todos';
let capcutTextSearch = '';
const CUSTOM_ADJUST_KEY = 'openCUP.adjust.presets';

function loadCustomAdjusts() {
  try {
    const raw = localStorage.getItem(CUSTOM_ADJUST_KEY);
    const list = raw ? JSON.parse(raw) : [];
    return Array.isArray(list) ? list.filter((item) => item && item.id && item.adjust) : [];
  } catch {
    return [];
  }
}

function saveCustomAdjusts(list) {
  try {
    localStorage.setItem(CUSTOM_ADJUST_KEY, JSON.stringify(list));
  } catch {
    /* sin almacenamiento: el preset vive solo en esta sesión */
  }
}

let customAdjusts = loadCustomAdjusts();

function defaultAdjustValue(key) {
  return key === 'brightness' || key === 'contrast' || key === 'saturate' ? 100 : 0;
}

function allAdjustPresets() {
  const saved = customAdjusts.map((item) => ({ id: item.id, label: item.label, category: 'Tuyos', adjust: item.adjust, custom: true }));
  return [...saved, ...ADJUST_PRESETS];
}

function filterGallery(activeFilter, includeNone, category = 'Destacado') {
  if (filterSource === 'capcut') {
    const q = capcutFilterSearch.trim().toLowerCase();
    const list = CAPCUT_FILTER_ITEMS.filter((item) => (q
      ? item.name.toLowerCase().includes(q)
      : capcutFilterCategory === 'Todos' || item.cats.includes(capcutFilterCategory))).slice(0, 5000);
    return `
    <div class="cat-rail category-chips">
      ${CAPCUT_FILTER_PANEL_CATEGORIES.map((name) => `<button class="category-chip${capcutFilterCategory === name ? ' active' : ''}" data-capcut-filter-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
    </div>
    <div class="cat-main">
      <div class="fx-sources">
        <button class="fx-source active" data-filter-source="capcut">CapCut</button>
        <button class="fx-source" data-filter-source="local">Local</button>
      </div>
      <label class="search-box filter-search">${icon('search', 13)}<input data-capcut-filter-search placeholder="Buscar filtro CapCut" value="${escapeHtml(capcutFilterSearch)}" /><span class="filter-count">${list.length}</span></label>
      <div class="effect-grid">
        ${list.map((item) => capcutCard(item, 'data-capcut-filter')).join('')}
      </div>
      <div class="panel-empty" style="padding-top:2px">Filtros reales de CapCut (${CAPCUT_FILTER_ITEMS.length}). Al elegir uno se aplica el equivalente local.</div>
    </div>`;
  }
  const categories = ['Todos', ...CAPCUT_FILTER_CATEGORIES];
  const query = filterSearch.trim().toLowerCase();
  const entries = Object.entries(FILTERS).filter(([key, value]) => {
    if (!includeNone && key === 'none') return false;
    if (query) return value.label.toLowerCase().includes(query);
    return category === 'Todos' || value.category === category;
  });
  return `
    <div class="cat-rail category-chips">
      ${categories.map((name) => `<button class="category-chip${category === name ? ' active' : ''}" data-filter-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
    </div>
    <div class="cat-main">
      <div class="fx-sources">
        <button class="fx-source" data-filter-source="capcut">CapCut</button>
        <button class="fx-source active" data-filter-source="local">Local</button>
      </div>
      <label class="search-box filter-search">${icon('search', 13)}<input data-filter-search placeholder="Buscar filtro" value="${escapeHtml(filterSearch)}" /><span class="filter-count">${entries.length}</span></label>
      <div class="filter-grid">
        ${entries.map(([key, value]) => `
          <button class="filter-card${activeFilter === key ? ' active' : ''}" data-filter="${key}" data-filter-label="${escapeHtml(value.label.toLowerCase())}">
            <div class="filter-preview" style="filter:${value.css}"></div>
            <div class="filter-name"><span>${value.label}</span>${activeFilter === key ? icon('check', 11) : ''}</div>
          </button>`).join('')}
      </div>
      <div class="panel-empty" style="padding-top:2px">
        ${activeFilter ? 'Filtro aplicado al clip seleccionado.' : 'Selecciona un clip de video o imagen para aplicarle un filtro.'}
      </div>
    </div>`;
}

function adjustReadout(key, value) {
  if (key === 'adjust.brightness' || key === 'adjust.contrast' || key === 'adjust.saturate') {
    return `${Math.round(value)}%`;
  }
  if (key === 'opacity' || key === 'volume' || key === 'scale') return `${Math.round(value * 100)}%`;
  if (key === 'rotate') return `${Math.round(value)}°`;
  if (key === 'speed') return `${value.toFixed(2)}×`;
  return String(Math.round(value * 100) / 100);
}

function adjustRow(label, key, inputValue, min, max, step, factor) {
  const stored = Number(inputValue) * factor;
  return `
    <div class="adj-row">
      <div class="adj-head"><span>${label}</span><b data-adj-readout="${key}">${adjustReadout(key, stored)}</b></div>
      <input type="range" data-adjust-key="${key}" data-factor="${factor}" min="${min}" max="${max}" step="${step}" value="${inputValue}" />
    </div>`;
}

async function recordVoice() {
  if (!navigator.mediaDevices?.getUserMedia || !window.MediaRecorder) {
    toast('Tu navegador no permite grabar audio', 'error');
    return;
  }
  let stream;
  try {
    stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  } catch (error) {
    console.error(error);
    toast('No se pudo acceder al micrófono', 'error');
    return;
  }
  const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' : '';
  const recorder = new MediaRecorder(stream, mime ? { mimeType: mime } : undefined);
  const chunks = [];
  let closed = false;
  let keep = false;
  let recordStartedAt = 0;
  const startedAt = performance.now();
  recorder.addEventListener('dataavailable', (event) => {
    if (event.data?.size) chunks.push(event.data);
  });

  const modal = openModal({
    title: 'Grabar voz',
    width: 420,
    dismissible: false,
    body: `
      <div style="display:flex;flex-direction:column;align-items:center;gap:12px;padding:14px 0 6px">
        <div class="rec-dot"></div>
        <div style="font-size:28px;font-weight:800;font-variant-numeric:tabular-nums" data-rec-time>00:00</div>
        <div class="muted" style="font-size:11px;text-align:center">Habla cerca del micrófono.<br/>La grabación se añadirá a tus medios y a la línea de tiempo.</div>
      </div>`,
    footer: `<button class="btn" data-rec-cancel>Cancelar</button><button class="btn btn-primary" data-rec-stop>Detener y usar</button>`,
    onMount(api) {
      const timeEl = api.body.querySelector('[data-rec-time]');
      const timer = setInterval(() => {
        timeEl.textContent = formatTime((performance.now() - startedAt) / 1000);
      }, 250);
      const stopRecording = (use) => {
        if (closed) return;
        closed = true;
        keep = use;
        clearInterval(timer);
        if (recorder.state !== 'inactive') recorder.stop();
        for (const track of stream.getTracks()) track.stop();
        api.close();
      };
      api.root.querySelector('[data-modal-close]')?.addEventListener('click', () => stopRecording(false));
      api.foot.querySelector('[data-rec-cancel]').addEventListener('click', () => stopRecording(false));
      api.foot.querySelector('[data-rec-stop]').addEventListener('click', () => stopRecording(true));
    },
  });

  recorder.addEventListener('stop', async () => {
    const modalHost = document.getElementById('modal-root');
    if (modalHost && !keep) return;
    if (!keep || !chunks.length) return;
    const raw = new Blob(chunks, { type: mime || 'audio/webm' });
    const seconds = Math.max(0.2, (performance.now() - recordStartedAt) / 1000);
    const blob = await fixWebmDuration(raw, seconds);
    const filename = `Grabación de voz ${new Date().toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit' })}.webm`;
    setPanel('media');
    await importFiles([arrayBufferToFile(blob, filename)]);
    toast('Grabación añadida a tus medios', 'success');
  });

  recordStartedAt = performance.now();
  recorder.start(250);
}

function parseSrt(text) {
  const blocks = text.replace(/\r/g, '').trim().split(/\n{2,}/);
  const cues = [];
  for (const block of blocks) {
    const lines = block.split('\n');
    const index = lines.findIndex((line) => line.includes('-->'));
    if (index < 0) continue;
    const match = lines[index].match(/(\d+):(\d+):(\d+)[,.](\d+)\s*-->\s*(\d+):(\d+):(\d+)[,.](\d+)/);
    if (!match) continue;
    const start = Number(match[1]) * 3600 + Number(match[2]) * 60 + Number(match[3]) + Number(match[4]) / 1000;
    const end = Number(match[5]) * 3600 + Number(match[6]) * 60 + Number(match[7]) + Number(match[8]) / 1000;
    const content = lines.slice(index + 1).join('\n').trim();
    if (content && end > start) cues.push({ start, end, content });
  }
  return cues;
}

export function mountLeftPanel(host) {
  host.classList.add('panel', 'left-panel');
  host.innerHTML = `
    <aside class="left-rail">
      <button class="rail-import" data-import>Importar ${icon('chevronUp', 12)}</button>
      <div class="rail-nav">
        <button class="rail-item" data-rail="media">Archivos multimedia</button>
        <button class="rail-item" data-rail="timeline">Línea de tiempo</button>
        <button class="rail-item" data-rail="secondary">Proyectos secundarios</button>
      </div>
      <div class="rail-groups">
        <button class="rail-drop" data-rail-group="yours">Tuyos ${icon('chevronDown', 10)}</button>
        <div class="rail-sub hidden" data-rail-sub="yours">
          <button data-rail-filter="all">Todos los medios</button>
          <button data-rail-filter="video">Videos</button>
          <button data-rail-filter="image">Imágenes</button>
          <button data-rail-filter="audio">Audio</button>
        </div>
        <button class="rail-drop rail-generate" data-rail-group="generate"><span>Generar</span><span class="rail-ai-tag">AI</span><span class="rail-ai-dot"></span>${icon('chevronDown', 10)}</button>
        <div class="rail-sub hidden" data-rail-sub="generate">
          <button data-ai-tool="image">Imagen de IA</button>
          <button data-ai-tool="video">Video de IA</button>
          <button data-ai-tool="audio">Audio de IA</button>
        </div>
        <button class="rail-drop" data-rail-group="spaces">Espacios ${icon('chevronDown', 10)}</button>
        <div class="rail-sub hidden" data-rail-sub="spaces">
          <button data-space-create>Crear espacio</button>
          <button data-space-info>¿Qué son los espacios?</button>
        </div>
        <button class="rail-drop" data-rail-group="library">Biblioteca ${icon('chevronDown', 10)}</button>
        <div class="rail-sub hidden" data-rail-sub="library">
          <button data-library-media>Medios guardados</button>
          <button data-library-templates>Plantillas</button>
        </div>
      </div>
    </aside>
    <div class="left-main">
      <div class="content-head" data-head></div>
      <div class="panel-body" data-body></div>
      <div class="panel-foot" data-foot></div>
    </div>`;
  const head = host.querySelector('[data-head]');
  const body = host.querySelector('[data-body]');
  const foot = host.querySelector('[data-foot]');
  const rail = host.querySelector('.left-rail');
  const fileInput = document.createElement('input');
  fileInput.type = 'file';
  fileInput.className = 'hidden';
  host.append(fileInput);

  const subscriptions = [];
  let suppressRender = false;
  let searchQuery = '';
  let fileMode = 'media';
  let railView = 'media';
  let mediaFilter = 'all';
  let openGroup = null;
  let stickerCategory = STICKER_CATEGORIES[0].name;
  let effectCategory = 'Todos';
  let filterCategory = 'Destacado';
  let transitionCategory = 'Todas';
  let adjustCategory = 'Todos';
  let adjustSearch = '';
  let filterSearchTimer = null;
  let captionStyle = 'clean';
  let textTab = 'text';
  let textSubTab = 'basic';
  let ttsVoices = [];
  let ttsVoice = '';
  let ttsRate = 1;
  let ttsPitch = 1;
  const speech = typeof window !== 'undefined' && 'speechSynthesis' in window ? window.speechSynthesis : null;
  const onVoicesChanged = () => {
    try {
      ttsVoices = speech?.getVoices() || [];
    } catch {
      ttsVoices = [];
    }
    if (host.isConnected && state.panel === 'text' && textTab === 'tts') render();
  };
  if (speech) {
    try {
      ttsVoices = speech.getVoices();
    } catch {
      ttsVoices = [];
    }
    speech.addEventListener?.('voiceschanged', onVoicesChanged);
  }

  function pickFiles(mode, accept, multiple) {
    fileMode = mode;
    fileInput.accept = accept;
    fileInput.multiple = multiple;
    fileInput.value = '';
    fileInput.click();
  }

  function applyFilter(filter) {
    const found = selectedClip();
    if (!found || !['video', 'image'].includes(found.clip.kind)) {
      toast('Selecciona un clip de video o imagen', 'warning');
      return;
    }
    updateClip(found.clip.id, { filter });
    toast(`Filtro “${FILTERS[filter]?.label || filter}” aplicado`, 'success', 1800);
  }

  function applyAdjustPreset(id) {
    const found = selectedClip();
    if (!found || !['video', 'image'].includes(found.clip.kind)) {
      toast('Selecciona un clip de video o imagen', 'warning');
      return;
    }
    const preset = allAdjustPresets().find((item) => item.id === id);
    if (!preset) return;
    if (!preset.adjust) {
      toast('Mueve los deslizadores de Detalles para crear tu ajuste', 'info');
      return;
    }
    updateClip(found.clip.id, { adjust: { ...preset.adjust } });
    toast(`Ajuste “${preset.label}” aplicado`, 'success', 1800);
  }

  async function saveCurrentAdjust() {
    const found = selectedClip();
    if (!found || !['video', 'image'].includes(found.clip.kind)) {
      toast('Selecciona un clip de video o imagen', 'warning');
      return;
    }
    const adjust = { ...(found.clip.adjust || {}) };
    if (!Object.keys(adjust).length) {
      toast('Mueve los deslizadores de Detalles antes de guardar', 'warning');
      return;
    }
    const name = await promptDialog({
      title: 'Agregar ajuste',
      label: 'Nombre del ajuste',
      value: `Ajuste ${customAdjusts.length + 1}`,
      confirmLabel: 'Guardar',
    });
    if (!name) return;
    const item = { id: `custom_${Date.now().toString(36)}`, label: name, adjust };
    customAdjusts = [item, ...customAdjusts.filter((entry) => entry.id !== item.id)].slice(0, 24);
    saveCustomAdjusts(customAdjusts);
    adjustCategory = 'Tuyos';
    adjustSearch = '';
    render();
    toast(`Ajuste “${name}” guardado en Tuyos`, 'success', 1800);
  }

  function applyTransition(kind) {
    const found = selectedClip();
    if (!found) {
      toast('Selecciona un clip para aplicar la transición', 'warning');
      return;
    }
    if (kind === 'open') {
      updateClip(found.clip.id, { transition: 'none', transitionDuration: 0.6, fadeIn: 0.9, fadeOut: 0 });
      toast('Apertura aplicada', 'success', 1800);
      return;
    }
    if (kind === 'close') {
      updateClip(found.clip.id, { transition: 'none', transitionDuration: 0.6, fadeIn: 0, fadeOut: 0.9 });
      toast('Cierre aplicado', 'success', 1800);
      return;
    }
    const fades = TRANSITION_FADES[kind] || { fadeIn: 0.2, fadeOut: 0.2 };
    updateClip(found.clip.id, {
      transition: kind,
      transitionDuration: 0.6,
      ...fades,
    });
    const label = TRANSITIONS.find((item) => item.id === kind)?.label || kind;
    toast(kind === 'none' ? 'Transición eliminada' : `Transición “${label}” aplicada`, 'success', 1800);
  }

  async function applyTemplate(template) {
    const ok = await confirmDialog({
      title: `Plantilla “${template.name}”`,
      message: 'Se creará un proyecto nuevo con esta plantilla y se guardará el proyecto actual. Después podrás añadir tus propios medios.',
      confirmLabel: 'Crear proyecto',
    });
    if (!ok) return;
    await saveNow();
    await newProject(template.name);
    updateProject({ ratio: template.ratio, resolution: '1080p', fps: 30 });
    const clip = addTextClip(template.content, {
      start: 0.4,
      duration: 6,
      preset: { size: template.size, weight: 900, color: template.color, x: 0, y: -6 },
    });
    if (template.subtitle) {
      addTextClip(template.subtitle, {
        start: 0.4,
        duration: 6,
        allowOverlap: true,
        newTrack: true,
        preset: { size: Math.round(template.size * 0.42), weight: 500, color: template.accent || '#ffffff', x: 0, y: 32 },
      });
    }
    if (clip) seek(clip.start + 0.05);
    toast('Plantilla aplicada. Añade tus medios para completarla.', 'success', 3800);
  }

  async function importSrt(file) {
    try {
      const text = await file.text();
      const cues = parseSrt(text);
      if (!cues.length) {
        toast('No se encontraron subtítulos válidos en el archivo', 'error');
        return;
      }
      let added = 0;
      for (const cue of cues.slice(0, 200)) {
        addTextClip(cue.content, {
          start: cue.start,
          duration: clamp(cue.end - cue.start, 0.4, 30),
          preset: { size: 56, weight: 600, color: '#ffffff' },
        });
        added += 1;
      }
      toast(`${added} subtítulo${added === 1 ? '' : 's'} importado${added === 1 ? '' : 's'}`, 'success');
    } catch (error) {
      console.error(error);
      toast('No se pudo leer el archivo SRT', 'error');
    }
  }

  function unavailable(title, message) {
    openModal({
      title,
      width: 430,
      body: `<div class="modal-note" style="margin:0">${escapeHtml(message)}</div>`,
      footer: `<button class="btn btn-primary" data-close-unavailable>Entendido</button>`,
      onMount(modal) {
        modal.foot.querySelector('[data-close-unavailable]').addEventListener('click', modal.close);
      },
    });
  }

  function audioPanel(audioMedia) {
    return `
      <div class="media-search-row">
        <label class="search-box">${icon('search', 13)}<input data-media-search placeholder="Buscar audio" value="${escapeHtml(searchQuery)}" /></label>
        <span class="file-count">${audioMedia.length}</span>
      </div>
      <button class="media-drop compact" data-record>
        <div class="import-cta"><span class="import-round">${icon('mic', 14)}</span><span>Grabar voz</span></div>
        <p>Graba una narración con tu micrófono</p>
      </button>
      ${audioMedia.length
        ? `<div class="media-grid">${audioMedia.map(mediaCard).join('')}</div>`
        : panelEmpty('Sin audio todavía', 'Importa música o graba tu voz para empezar.', 'music')}`;
  }

  function mediaPanel(media) {
    const filtered = media.filter((item) => mediaFilter === 'all' || item.kind === mediaFilter);
    const ip = state.importProgress;
    const progressHtml = ip ? (() => {
      const pct = ip.total ? Math.round((ip.current / ip.total) * 100) : 0;
      const elapsed = Date.now() - (ip.startedAt || Date.now());
      const perFile = ip.current > 0 ? elapsed / ip.current : 0;
      const remaining = Math.max(0, (ip.total - ip.current) * perFile);
      const eta = remaining > 0 ? `~${Math.ceil(remaining / 1000)}s restantes` : 'Finalizando…';
      return `
        <div class="import-progress" style="margin:8px 0;padding:10px;background:var(--bg-2);border:1px solid var(--border);border-radius:8px">
          <div style="font-size:12px;font-weight:700;margin-bottom:6px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${escapeHtml(ip.fileName || 'Procesando…')}</div>
          <div style="height:6px;background:var(--bg-3);border-radius:3px;overflow:hidden">
            <div style="height:100%;width:${pct}%;background:var(--accent,#1fa7b5);transition:width .2s"></div>
          </div>
          <div style="font-size:11px;color:var(--muted);margin-top:4px;display:flex;justify-content:space-between">
            <span>${ip.current}/${ip.total}</span>
            <span>${eta}</span>
          </div>
        </div>`;
    })() : '';
    const top = `
      <div class="media-search-row">
        <label class="search-box">${icon('search', 13)}<input data-media-search placeholder="Buscar medios" value="${escapeHtml(searchQuery)}" /></label>
        <span class="file-count">${filtered.length} archivo${filtered.length === 1 ? '' : 's'}</span>
      </div>`;
    if (!state.media.size) {
      return `
        ${progressHtml}
        <div class="media-drop" data-import>
          <div class="import-cta"><span class="import-round">${icon('plus', 15)}</span><span>Importar</span></div>
          <p>Arrastra y suelta los videos, las fotos y los archivos de audio aquí</p>
        </div>
        <div class="tools-divider"><span>¿No tienes archivos multimedia? Crea con estas herramientas</span></div>
        <div class="media-tool-cards">
          <button class="media-tool-card" data-generate>${icon('wand', 20)}<span>Generar</span></button>
          <button class="media-tool-card" data-panel-link="avatar">${icon('avatar', 20)}<span>Avatares de IA</span></button>
          <button class="media-tool-card" data-record>${icon('record', 20)}<span>Grabar</span></button>
          <button class="media-tool-card" data-open-capcut>${icon('folderOpen', 20)}<span>CapCut</span></button>
        </div>`;
    }
    return `
      ${top}
      ${progressHtml}
      ${filtered.length
        ? `<div class="media-grid">${filtered.map(mediaCard).join('')}</div>`
        : panelEmpty('No hay archivos de este tipo', 'Cambia el filtro o importa un archivo nuevo.', 'film')}
      <div class="media-tool-cards small">
        <button class="media-tool-card" data-generate>${icon('wand', 17)}<span>Generar</span></button>
        <button class="media-tool-card" data-record>${icon('record', 17)}<span>Grabar</span></button>
      </div>`;
  }

  function activeTextClip() {
    const found = selectedClip();
    return found && found.clip.kind === 'text' ? found.clip : null;
  }

  function applyTextPatch(patch, message = 'Texto actualizado') {
    const clip = activeTextClip();
    if (clip) {
      updateClip(clip.id, { text: patch });
      toast(message, 'success', 1400);
      return;
    }
    addTextClip('Tu texto aquí', { start: state.playhead, preset: { text: patch } });
    toast(`${message} y añadido en el cursor`, 'success', 1600);
  }

  function applyTextAnimation(item) {
    const patch = item
      ? { animation: item.id, animationPhase: item.phase, animationDuration: item.duration }
      : { animation: 'none' };
    applyTextPatch(patch, item ? `Animación “${item.label}” aplicada` : 'Animación eliminada');
    if (!item) return;
    // Coloca el cursor donde la animación es visible: su inicio (entrada),
    // justo antes del final (salida) o en pleno bucle.
    const found = selectedClip();
    const clip = found?.clip?.kind === 'text' ? found.clip : null;
    if (!clip) return;
    const duration = Math.max(0.1, item.duration || 0.6);
    let target = clip.start;
    if (item.phase === 'out') {
      target = clip.start + Math.max(0, clip.duration - Math.min(duration, clip.duration));
    } else if (item.phase === 'loop') {
      target = clip.start + Math.min(0.35, Math.max(0.05, clip.duration / 2));
    }
    seek(target);
  }

  function speakTts() {
    if (!speech || typeof SpeechSynthesisUtterance === 'undefined') {
      toast('Este sistema no ofrece voces locales para la vista previa', 'warning');
      return;
    }
    speech.cancel();
    const area = body.querySelector('[data-tts-text]');
    const content = (area?.value || activeTextClip()?.text?.content || '').trim();
    if (!content) {
      toast('Escribe el texto que quieres escuchar', 'warning');
      return;
    }
    const utterance = new SpeechSynthesisUtterance(content);
    utterance.lang = 'es-ES';
    utterance.rate = ttsRate;
    utterance.pitch = ttsPitch;
    const voice = ttsVoices.find((item) => item.voiceURI === ttsVoice);
    if (voice) {
      utterance.voice = voice;
      utterance.lang = voice.lang || utterance.lang;
    }
    utterance.onerror = (event) => {
      if (event.error && !['canceled', 'interrupted'].includes(event.error)) {
        toast('No se pudo reproducir la voz del sistema', 'warning');
      }
    };
    speech.speak(utterance);
    toast('Reproduciendo vista previa de voz', 'info', 1300);
  }

  /* ------------------------- Kokoro TTS (IA local) ------------------------- */
  let kokoroBusy = false;

  function setKokoroProgress(label, value) {
    const wrap = body.querySelector('[data-tts-progress]');
    if (!wrap) return;
    if (!label) {
      wrap.classList.add('hidden');
      wrap.classList.remove('indeterminate');
      return;
    }
    wrap.classList.remove('hidden');
    const text = wrap.querySelector('[data-tts-progress-label]');
    const bar = wrap.querySelector('[data-tts-progress-bar]');
    if (text) text.textContent = label;
    if (bar && Number.isFinite(value)) {
      bar.style.width = `${Math.max(0, Math.min(100, value))}%`;
      wrap.classList.remove('indeterminate');
    } else if (bar) {
      wrap.classList.add('indeterminate');
    }
  }

  function kokoroButtonState(busy) {
    const button = body.querySelector('[data-tts-generate]');
    if (!button) return;
    button.disabled = busy;
    button.innerHTML = busy
      ? `${icon('rotate', 13)} Generando…`
      : `${icon('sparkles', 13)} Generar voz e importar a la línea de tiempo`;
  }

  async function generateKokoro() {
    if (kokoroBusy) return;
    const area = body.querySelector('[data-tts-text]');
    const text = (area?.value || activeTextClip()?.text?.content || '').trim();
    if (!text) {
      toast('Escribe el texto que quieres convertir en voz', 'warning');
      return;
    }
    const voice = body.querySelector('[data-kokoro-voice]')?.value || 'ef_dora';
    kokoroBusy = true;
    kokoroButtonState(true);
    try {
      const result = await generateKokoroAudio({
        text,
        voice,
        speed: ttsRate,
        onProgress: (label, value) => setKokoroProgress(label, value),
      });
      setKokoroProgress('Importando el audio al proyecto…', 100);
      const stamp = new Date().toISOString().slice(11, 19).replace(/:/g, '-');
      const file = new File([result.blob], `Kokoro ${voice} ${stamp}.wav`, { type: 'audio/wav' });
      await importFiles([file], { silent: true });
      toast(`Voz generada (${result.seconds.toFixed(1)} s) e importada a la línea de tiempo`, 'success', 4200);
    } catch (error) {
      console.error('Kokoro TTS', error);
      const message = error?.message || 'no se pudo generar la voz';
      toast(`Kokoro: ${message}`, 'error', 5400);
    } finally {
      kokoroBusy = false;
      setKokoroProgress(null);
      kokoroButtonState(false);
    }
  }

  function textStyleSample(style) {
    const patch = style.patch || {};
    const background = patch.background && patch.background !== 'transparent' ? patch.background : '#343434';
    const color = patch.color || '#ffffff';
    const weight = patch.weight || 700;
    const font = patch.font ? `'${patch.font}', sans-serif` : 'inherit';
    const size = clamp((patch.size || 72) * 0.17, 11, 22);
    const stroke = patch.strokeWidth
      ? `-webkit-text-stroke:${Math.max(0.5, patch.strokeWidth * 9)}px ${patch.strokeColor || '#000000'};`
      : '';
    const glow = patch.glow ? `text-shadow:0 0 8px ${patch.glowColor || '#00e5d0'};` : '';
    const radius = clamp((patch.backgroundRadius ?? 0.16) * 30, 2, 14);
    return `background:${background};color:${color};font-weight:${weight};font-family:${font};font-size:${size}px;border-radius:${radius}px;${stroke}${glow}`;
  }

  function bubbleSample(bubble) {
    const patch = bubble.patch || {};
    const alpha = Number.isFinite(Number(patch.backgroundAlpha)) ? Number(patch.backgroundAlpha) : 1;
    const background = patch.background === 'transparent'
      ? 'transparent'
      : rgbaColor(patch.background || '#ffffff', alpha);
    const border = patch.backgroundBorderWidth
      ? `${Math.max(1, patch.backgroundBorderWidth * 520)}px solid ${patch.backgroundBorderColor || '#ffffff'}`
      : '1px dashed #5a5a5a';
    const radius = clamp((patch.backgroundRadius ?? 0.16) * 46, 2, 22);
    return `background:${background};color:${patch.color || '#ffffff'};border:${border};border-radius:${radius}px;`;
  }

  function effectSample(effect) {
    const patch = effect.patch || {};
    const color = patch.color || '#ffffff';
    const stroke = patch.strokeWidth
      ? `-webkit-text-stroke:${Math.max(0.5, patch.strokeWidth * 22)}px ${patch.strokeColor || '#000000'};`
      : '';
    const shadows = [];
    if (patch.glow) shadows.push(`0 0 ${8 + (patch.glowBlur || 0.3) * 16}px ${patch.glowColor || '#00e5d0'}`);
    if (patch.shadow) {
      const distance = (patch.shadowDistance || 0.05) * 18;
      shadows.push(`${distance}px ${distance}px ${(patch.shadowBlur || 0.2) * 10}px ${patch.shadowColor || '#000000'}`);
    }
    return `color:${color};${stroke}${shadows.length ? `text-shadow:${shadows.join(',')};` : ''}`;
  }

  function textBasicPanel() {
    return `
      <button class="text-default-card" data-add-text>
        <span class="text-default-plus">${icon('plus', 15)}</span>
        <span class="text-default-copy"><strong>Texto predeterminado</strong></span>
      </button>
      <div class="text-panel-section-head"><span>Estilo preestablecido</span><b>${TEXT_STYLES.length}</b></div>
      <div class="text-style-grid">
        ${TEXT_STYLES.map((style, index) => `
          <button class="text-style-card" data-preset="${index}" title="Añadir texto con estilo ${style.label}">
            <span class="text-style-sample" style="${textStyleSample(style)}">Aa</span>
            <span class="text-style-name">${escapeHtml(style.label)}</span>
          </button>`).join('')}
      </div>
      <div class="panel-empty" style="padding-top:10px">Haz clic en un estilo para añadirlo en el cursor. Puedes ajustar todos los detalles en el panel derecho.</div>`;
  }

  function textBubblePanel() {
    const clip = activeTextClip();
    return `
      <div class="text-panel-hint">${clip ? `Se aplicará a “${escapeHtml((clip.text?.content || '').slice(0, 34))}”.` : 'Elige una burbuja: se creará un texto nuevo con ese estilo.'}</div>
      <div class="text-bubble-grid">
        ${TEXT_BUBBLES.map((bubble) => `
          <button class="text-bubble-card" data-text-bubble="${bubble.id}">
            <span class="text-bubble-sample" style="${bubbleSample(bubble)}">Aa</span>
            <span class="text-style-name">${escapeHtml(bubble.label)}</span>
          </button>`).join('')}
      </div>
      <div class="modal-note" style="margin:10px 12px 16px">${icon('info', 13)} Las burbujas son fondos redondeados con borde opcional; se dibujan en el preview y en la exportación.</div>`;
  }

  function textEffectPanel() {
    const clip = activeTextClip();
    const active = clip?.text?.strokeWidth > 0 || clip?.text?.glow || clip?.text?.shadow;
    return `
      <div class="text-panel-hint">${clip ? 'Aplica un efecto al texto seleccionado.' : 'Elige un efecto: se creará un texto nuevo con ese acabado.'}</div>
      <div class="text-effect-grid">
        ${TEXT_EFFECTS.map((effect) => `
          <button class="text-effect-card" data-text-effect="${effect.id}">
            <span class="text-effect-sample" style="${effectSample(effect)}">Aa</span>
            <span class="text-style-name">${escapeHtml(effect.label)}</span>
          </button>`).join('')}
      </div>
      <div class="panel-empty" style="padding-top:10px">${active ? 'Los efectos se combinan con el color, el trazo y la sombra del panel derecho.' : 'Sin efectos adicionales en el texto seleccionado.'}</div>`;
  }

  function textAnimationPanel() {
    const clip = activeTextClip();
    const activeId = clip?.text?.animation || 'none';
    return `
      <div class="text-panel-hint">${clip ? 'Selecciona una animación para el texto.' : 'Elige una animación: se creará un texto nuevo con ella.'}</div>
      ${clip ? `<button class="btn btn-small btn-block" data-text-animation="none" style="margin:0 12px 8px;width:calc(100% - 24px)">${icon('rotate', 13)} Quitar animación</button>` : ''}
      ${TEXT_ANIMATION_GROUPS.map((group) => `
        <div class="text-panel-section-head"><span>${group.label}</span><b>${group.items.length}</b></div>
        <div class="text-animation-grid">
          ${group.items.map((item) => `
            <button class="text-animation-card${activeId === item.id ? ' active' : ''}" data-text-animation="${item.id}">
              <span class="anim-preview anim-${item.id}">Aa</span>
              <span class="text-style-name">${escapeHtml(item.label)}</span>
              ${activeId === item.id ? `<span class="text-animation-check">${icon('check', 10)}</span>` : ''}
            </button>`).join('')}
        </div>`).join('')}
      <div class="modal-note" style="margin:10px 12px 16px">${icon('info', 13)} Las animaciones de entrada y salida se reproducen al inicio y al final del clip. Los bucles se repiten mientras el texto está visible.</div>`;
  }

  function textTrackingPanel() {
    return `
      <div class="text-track-hero">
        <span class="text-track-art">${icon('text', 30)}</span>
        <strong>Seguimiento de movimiento</strong>
        <p>El seguimiento de CapCut analiza el video en la nube para pegar el texto a una persona u objeto. openCUP es un editor local y no incluye ese análisis.</p>
      </div>
      <div class="text-info-card">
        <strong>Alternativa sin conexión</strong>
        <p>Puedes mover el texto manualmente en el preview y ajustar la posición con fotogramas clave en la línea de tiempo. El resultado se mantiene editable y se exporta igual.</p>
        <button class="btn btn-block" data-add-text>${icon('plus', 14)} Añadir texto y colocarlo a mano</button>
      </div>
      <div class="text-info-card muted-card">La detección automática de caras, objetos y puntos de seguimiento necesita un servicio externo. No se envía ningún archivo desde openCUP.</div>`;
  }

  function ttsVoiceOptions() {
    if (!ttsVoices.length) return '<option value="">Voz del sistema</option>';
    const sorted = [...ttsVoices].sort((a, b) => {
      const aEs = a.lang?.toLowerCase().startsWith('es') ? 0 : 1;
      const bEs = b.lang?.toLowerCase().startsWith('es') ? 0 : 1;
      return aEs - bEs || a.name.localeCompare(b.name);
    });
    return `<option value="">Voz automática (español)</option>${sorted.map((voice) => `<option value="${escapeHtml(voice.voiceURI)}" ${ttsVoice === voice.voiceURI ? 'selected' : ''}>${escapeHtml(voice.name)} (${escapeHtml(voice.lang || 'sistema')})</option>`).join('')}`;
  }

  function textToSpeechPanel() {
    const clip = activeTextClip();
    const sample = clip?.text?.content || 'Hola, esta es una vista previa de la voz de openCUP.';
    return `
      <div class="text-panel-hint">Vista previa con las voces instaladas en Windows. El audio no se añade a la línea de tiempo.</div>
      <div class="text-tts-form">
        <div class="field">
          <label>Texto para leer</label>
          <textarea class="input" data-tts-text rows="3" maxlength="400">${escapeHtml(sample)}</textarea>
        </div>
        <div class="field">
          <label>Voz</label>
          <select class="input" data-tts-voice>${ttsVoiceOptions()}</select>
        </div>
        <div class="field-row">
          <div class="field">
            <label>Velocidad <b data-tts-rate-readout>${ttsRate.toFixed(2)}×</b></label>
            <input type="range" data-tts-rate min="0.5" max="2" step="0.05" value="${ttsRate}" />
          </div>
          <div class="field">
            <label>Tono <b data-tts-pitch-readout>${ttsPitch.toFixed(2)}</b></label>
            <input type="range" data-tts-pitch min="0.5" max="1.8" step="0.05" value="${ttsPitch}" />
          </div>
        </div>
        <div class="text-tts-actions">
          <button class="btn btn-primary" data-tts-play>${icon('play', 13)} Reproducir</button>
          <button class="btn" data-tts-stop>${icon('pause', 13)} Detener</button>
        </div>
        <button class="btn btn-ghost btn-block" data-tts-use-text>${icon('text', 13)} Usar el texto seleccionado</button>
      </div>
      <div class="text-kokoro-form">
        <div class="text-panel-section-head"><span>Voz neuronal Kokoro</span><b>IA local</b></div>
        <div class="field">
          <label>Voz</label>
          <select class="input" data-kokoro-voice>
            <option value="ef_dora">Dora · español (femenina)</option>
            <option value="em_alex">Alex · español (masculina)</option>
            <option value="em_santa">Santa · español (masculina)</option>
            <option value="af_heart">Heart · inglés (femenina)</option>
          </select>
        </div>
        <div class="tts-progress hidden" data-tts-progress>
          <div class="tts-progress-track"><div class="tts-progress-bar" data-tts-progress-bar></div></div>
          <span class="tts-progress-label" data-tts-progress-label>Preparando…</span>
        </div>
        <button class="btn btn-primary btn-block" data-tts-generate>${icon('sparkles', 13)} Generar voz e importar a la línea de tiempo</button>
        <div class="text-panel-hint" style="margin-top:6px">La primera vez descarga el modelo (~92 MB); después se usa la caché del navegador y funciona sin conexión.</div>
      </div>
      <div class="text-info-card">${icon('mic', 14)} <strong>Voces locales</strong><p>Se usan las voces de Windows mediante <code>speechSynthesis</code>. Si no aparece ninguna voz en la lista, instala un paquete de voz en Configuración → Hora e idioma → Voz.</p></div>`;
  }

  function textPanel() {
    const tabs = [
      ['text', 'Texto'],
      ['animation', 'Animación'],
      ['tracking', 'Seguimiento'],
      ['tts', 'Texto a voz'],
      ['avatar', 'Avatares de IA'],
      ['capcut', `${CAPCUT_TEXT_ITEMS.length} de CapCut`],
    ];
    let content = '';
    if (textTab === 'capcut') {
      const q = capcutTextSearch.trim().toLowerCase();
      const list = CAPCUT_TEXT_ITEMS.filter((item) => (q
        ? item.name.toLowerCase().includes(q)
        : capcutTextCategory === 'Todos' || item.cats.includes(capcutTextCategory))).slice(0, 5000);
      content = `
      <div class="cat-rail category-chips">
        ${CAPCUT_TEXT_CATEGORIES.map((name) => `<button class="category-chip${name === capcutTextCategory ? ' active' : ''}" data-capcut-text-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        <label class="search-box filter-search">${icon('search', 13)}<input data-capcut-text-search placeholder="Buscar texto CapCut" value="${escapeHtml(capcutTextSearch)}" /><span class="filter-count">${list.length}</span></label>
        <div class="effect-grid">
          ${list.map((item) => capcutCard(item, 'data-capcut-text')).join('')}
        </div>
        <div class="panel-empty" style="padding-top:6px">Plantillas de texto reales de CapCut (${CAPCUT_TEXT_ITEMS.length}) con su categoría.</div>
      </div>`;
    } else if (textTab === 'text') {
      const subTabs = [['basic', 'Básico'], ['bubble', 'Burbuja'], ['effect', 'Efectos']];
      content = `
        <div class="text-subtabs">
          ${subTabs.map(([id, label]) => `<button class="text-subtab${textSubTab === id ? ' active' : ''}" data-text-subtab="${id}">${label}</button>`).join('')}
        </div>
        ${textSubTab === 'bubble' ? textBubblePanel() : textSubTab === 'effect' ? textEffectPanel() : textBasicPanel()}`;
    } else if (textTab === 'animation') content = textAnimationPanel();
    else if (textTab === 'tracking') content = textTrackingPanel();
    else if (textTab === 'tts') content = textToSpeechPanel();
    else content = avatarPanel();
    return `
      <div class="text-panel-tabs">
        ${tabs.map(([id, label]) => `<button class="text-panel-tab${textTab === id ? ' active' : ''}" data-text-tab="${id}">${label}</button>`).join('')}
      </div>
      <div class="text-panel-content">${content}</div>`;
  }

  function stickersPanel() {
    if (stickerSource === 'capcut') {
      const q = capcutStickerSearch.trim().toLowerCase();
      const list = CAPCUT_STICKER_ITEMS.filter((item) => (q
        ? item.name.toLowerCase().includes(q)
        : capcutStickerCategory === 'Todos' || item.cats.includes(capcutStickerCategory))).slice(0, 5000);
      return `
      <div class="cat-rail category-chips">
        ${CAPCUT_STICKER_CATEGORIES.map((name) => `<button class="category-chip${name === capcutStickerCategory ? ' active' : ''}" data-capcut-sticker-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        <div class="fx-sources">
          <button class="fx-source active" data-sticker-source="capcut">CapCut</button>
          <button class="fx-source" data-sticker-source="local">Local</button>
        </div>
        <label class="search-box filter-search">${icon('search', 13)}<input data-capcut-sticker-search placeholder="Buscar sticker CapCut" value="${escapeHtml(capcutStickerSearch)}" /><span class="filter-count">${list.length}</span></label>
        <div class="effect-grid">
          ${list.map((item) => capcutCard(item, 'data-capcut-sticker')).join('')}
        </div>
        <div class="panel-empty" style="padding-top:6px">Stickers reales de CapCut (${CAPCUT_STICKER_ITEMS.length}) con su categoría.</div>
      </div>`;
    }
    const query = stickerSearch.trim().toLowerCase();
    const category = STICKER_CATEGORIES.find((item) => item.name === stickerCategory) || STICKER_CATEGORIES[0];
    const items = query
      ? STICKERS.filter((sticker) => sticker.toLowerCase().includes(query))
      : category.items;
    return `
      <div class="cat-rail category-chips">
        ${STICKER_CATEGORIES.map((item) => `<button class="category-chip${!query && item.name === category.name ? ' active' : ''}" data-sticker-category="${escapeHtml(item.name)}">${escapeHtml(item.name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        <div class="fx-sources">
          <button class="fx-source" data-sticker-source="capcut">CapCut</button>
          <button class="fx-source active" data-sticker-source="local">Local</button>
        </div>
        <label class="search-box filter-search">${icon('search', 13)}<input data-sticker-search placeholder="Buscar sticker" value="${escapeHtml(stickerSearch)}" /><span class="filter-count">${items.length}</span></label>
        <div class="sticker-grid">
          ${items.map((sticker) => `<button class="sticker-btn" data-sticker="${escapeHtml(sticker)}" title="Añadir ${escapeHtml(sticker)}">${sticker}</button>`).join('')}
        </div>
        <div class="panel-empty" style="padding-top:6px">Los stickers son capas independientes. Escálalos, muévelos y rótalos desde Detalles.</div>
      </div>`;
  }

  function effectPanel() {
    const found = selectedClip();
    const isVisual = found && ['video', 'image'].includes(found.clip.kind);
    const active = isVisual ? found.clip.effect || 'none' : 'none';
    const sourceTabs = `
      <div class="fx-sources">
        <button class="fx-source${effectSource === 'capcut' ? ' active' : ''}" data-effect-source="capcut">CapCut</button>
        <button class="fx-source${effectSource === 'local' ? ' active' : ''}" data-effect-source="local">Local</button>
      </div>`;

    if (effectSource === 'capcut') {
      const q = capcutSearch.trim().toLowerCase();
      const matching = CAPCUT_EFFECT_ITEMS.filter((item) => (q
        ? item.name.toLowerCase().includes(q)
        : capcutCategory === 'Todos' || item.cats.includes(capcutCategory)));
      const list = matching.slice(0, 5000);
      return `
      <div class="cat-rail category-chips">
        ${CAPCUT_EFFECT_CATEGORIES.map((name) => `<button class="category-chip${name === capcutCategory ? ' active' : ''}" data-capcut-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        ${sourceTabs}
        <label class="search-box filter-search">${icon('search', 13)}<input data-capcut-search placeholder="Buscar en CapCut" value="${escapeHtml(capcutSearch)}" /><span class="filter-count">${list.length}</span></label>
        ${(state.project?.effectClips?.length ?? 0) > 0 ? `<button class="btn btn-small fx-clear" data-clear-fx>Quitar efectos de la línea de tiempo (${state.project.effectClips.length})</button>` : ''}
        <div class="effect-grid">
          ${list.map((item) => `
            <button class="effect-card" data-capcut-effect="${item.id}" data-capcut-name="${escapeHtml(item.name)}" title="${escapeHtml(item.name)}">
              <img class="effect-thumb capcut" src="${item.img}" alt="${escapeHtml(item.name)}" loading="lazy" decoding="async" />
              <div class="filter-name"><span>${escapeHtml(item.name)}</span></div>
            </button>`).join('')}
        </div>
        <div class="panel-empty" style="padding-top:2px">Catálogo real de CapCut: ${CAPCUT_EFFECT_ITEMS.length} efectos con su portada. Al elegir uno se aplica el equivalente local más cercano.</div>
      </div>`;
    }

    const query = effectSearch.trim().toLowerCase();
    const entries = Object.entries(EFFECTS).filter(([, value]) => {
      if (query) return value.label.toLowerCase().includes(query);
      return effectCategory === 'Todos' || value.category === effectCategory;
    });
    const swatch = (value) => {
      const colors = value.colors?.length
        ? value.colors
        : value.accent
          ? [value.accent, '#8ffcff', '#ffffff']
          : ['#2f6b73', '#d3b26d', '#8c3559'];
      return `linear-gradient(135deg,${colors[0]},${colors[1 % colors.length]} 55%,${colors[2 % colors.length]})`;
    };
    return `
      <div class="cat-rail category-chips">
        ${['Todos', ...EFFECT_CATEGORIES].map((name) => `<button class="category-chip${name === effectCategory ? ' active' : ''}" data-effect-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        ${sourceTabs}
        <label class="search-box filter-search">${icon('search', 13)}<input data-effect-search placeholder="Buscar efecto" value="${escapeHtml(effectSearch)}" /><span class="filter-count">${entries.length}</span></label>
        <div class="effect-grid">
          ${entries.map(([key, value]) => `
            <button class="effect-card${active === key ? ' active' : ''}" data-effect="${key}" title="${escapeHtml(value.label)}">
              <img class="effect-thumb" src="assets/previews/effects/${key}.jpg" alt="${escapeHtml(value.label)}" loading="lazy" decoding="async" />
              <div class="filter-name"><span>${value.label}</span>${active === key ? icon('check', 11) : ''}</div>
            </button>`).join('')}
        </div>
        <div class="panel-empty" style="padding-top:2px">${active !== 'none' ? `Efecto “${EFFECTS[active]?.label || active}” aplicado al clip.` : 'Selecciona un clip y elige un efecto visual.'}</div>
      </div>`;
  }

  function transitionPanel() {
    const found = selectedClip();
    const active = found?.clip.transition || 'none';
    if (transitionSource === 'capcut') {
      const q = capcutTransitionSearch.trim().toLowerCase();
      const list = CAPCUT_TRANSITION_ITEMS.filter((item) => (q
        ? item.name.toLowerCase().includes(q)
        : capcutTransitionCategory === 'Todas' || item.cats.includes(capcutTransitionCategory))).slice(0, 5000);
      return `
      <div class="cat-rail category-chips">
        ${CAPCUT_TRANSITION_PANEL_CATEGORIES.map((name) => `<button class="category-chip${capcutTransitionCategory === name ? ' active' : ''}" data-capcut-transition-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        <div class="fx-sources">
          <button class="fx-source active" data-transition-source="capcut">CapCut</button>
          <button class="fx-source" data-transition-source="local">Local</button>
        </div>
        <label class="search-box filter-search">${icon('search', 13)}<input data-capcut-transition-search placeholder="Buscar transición CapCut" value="${escapeHtml(capcutTransitionSearch)}" /><span class="filter-count">${list.length}</span></label>
        <div class="effect-grid">
          ${list.map((item) => capcutCard(item, 'data-capcut-transition')).join('')}
        </div>
        <div class="panel-empty" style="padding-top:2px">Transiciones reales de CapCut (${CAPCUT_TRANSITION_ITEMS.length}) en su orden: Entrada, Salida, Combinado.</div>
      </div>`;
    }
    const categories = ['Todas', ...new Set(TRANSITIONS.map((item) => item.category))];
    const entries = TRANSITIONS.filter((item) => transitionCategory === 'Todas' || item.category === transitionCategory);
    return `
      <div class="cat-rail category-chips">
        ${categories.map((name) => `<button class="category-chip${name === transitionCategory ? ' active' : ''}" data-transition-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        <div class="fx-sources">
          <button class="fx-source" data-transition-source="capcut">CapCut</button>
          <button class="fx-source active" data-transition-source="local">Local</button>
        </div>
        <div class="transition-grid">
          ${entries.map((item) => `
            <button class="transition-card${active === item.id ? ' active' : ''}" data-transition="${item.id}" title="${escapeHtml(item.category)}">
              <div class="transition-preview ${item.className}"><i></i></div>
              <div class="filter-name"><span>${item.label}</span>${active === item.id ? icon('check', 11) : ''}</div>
            </button>`).join('')}
        </div>
        <div class="panel-empty" style="padding-top:2px">La transición se aplica al inicio del clip seleccionado y se mezcla con el clip anterior. Ajusta su duración desde Detalles.</div>
      </div>`;
  }

  let whisperReady = false;
  let whisperChecking = false;
  let whisperCheckedOnce = false;

  async function checkWhisperStatus() {
    if (whisperChecking) return;
    whisperChecking = true;
    try {
      whisperReady = await isWhisperModelReady();
      whisperCheckedOnce = true;
    } catch {
      whisperReady = false;
    } finally {
      whisperChecking = false;
      if (host.isConnected && state.panel === 'captions') {
        render();
      }
    }
  }

  function openSubtitleModal({ mode = 'generate' } = {}) {
    const videos = Array.from(state.media.values()).filter((m) => m.kind === 'video' || m.kind === 'audio');
    if (!videos.length && mode === 'generate') {
      toast('Importa primero un vídeo o archivo de audio a la biblioteca', 'warn', 3500);
      return;
    }

    const currentClip = selectedClip()?.clip;
    const defaultMediaId = (currentClip?.mediaId && state.media.has(currentClip.mediaId))
      ? currentClip.mediaId
      : (videos[0]?.id || '');

    let activeStyle = captionStyle;
    let abortController = null;
    let running = false;

    const bodyHtml = `
      <div data-subs-config ${mode === 'download' ? 'class="hidden"' : ''}>
        <div class="field" style="margin-bottom:12px">
          <label style="font-size:11px;font-weight:600;display:block;margin-bottom:5px">Vídeo / Audio a transcribir</label>
          <select class="input" data-subs-media style="width:100%">
            ${videos.map((v) => `<option value="${v.id}" ${v.id === defaultMediaId ? 'selected' : ''}>${escapeHtml(v.name)} (${formatTime(v.duration || 0)})</option>`).join('')}
          </select>
        </div>

        <div class="field" style="margin-bottom:12px">
          <label style="font-size:11px;font-weight:600;display:block;margin-bottom:5px">Idioma de la voz</label>
          <select class="input" data-subs-lang style="width:100%">
            <option value="es" selected>Español (es)</option>
            <option value="en">Inglés (en)</option>
            <option value="auto">Detectar automáticamente</option>
            <option value="fr">Francés (fr)</option>
            <option value="de">Alemán (de)</option>
            <option value="it">Italiano (it)</option>
            <option value="pt">Portugués (pt)</option>
          </select>
        </div>

        <div class="field" style="margin-bottom:12px">
          <label style="font-size:11px;font-weight:600;display:block;margin-bottom:5px">Estilo de subtítulo</label>
          <div class="caption-styles" style="padding:0">
            ${CAPTION_STYLES.map((style) => `
              <button type="button" class="caption-style${activeStyle === style.id ? ' active' : ''}" data-modal-style="${style.id}">
                <span style="font-weight:${style.weight};color:${style.color};${style.background !== 'transparent' ? `background:${style.background};` : ''}">${style.label}</span>
              </button>`).join('')}
          </div>
        </div>

        <div class="field" style="margin-bottom:12px">
          <label style="display:flex;align-items:center;gap:7px;font-size:11.5px;cursor:pointer">
            <input type="checkbox" data-subs-replace checked style="cursor:pointer" />
            <span>Reemplazar subtítulos actuales en la pista de texto</span>
          </label>
        </div>

        <div class="subs-notice ${whisperReady ? 'is-ready' : 'is-pending'}" style="margin-top:8px">
          <div style="display:flex;align-items:center;gap:7px;font-weight:600;font-size:11px">
            ${whisperReady ? icon('check', 14) : icon('download', 14)}
            <span>${whisperReady ? 'Modelo Whisper disponible localmente' : 'Descarga inicial requerida (~40 MB)'}</span>
          </div>
          <div style="margin-top:4px;font-size:10px;color:#9a9a9a;line-height:1.45">
            ${whisperReady
              ? 'El modelo Whisper Tiny ya está descargado. La transcripción se ejecutará de forma directa sin esperas de descarga.'
              : 'La primera vez se descargará la dependencia Whisper (~40 MB). Podrás ver la barra de progreso del 1% al 100% durante la descarga y después durante la transcripción.'}
          </div>
        </div>
      </div>

      <div data-subs-progress ${mode === 'download' ? '' : 'class="hidden"'} style="padding:6px 0">
        <div class="subs-steps" style="display:flex;gap:8px;margin-bottom:16px">
          <div class="subs-step" data-step-download style="flex:1;padding:8px 10px;border-radius:6px;background:#2a2a2a;border:1px solid #3d3d3d;display:flex;align-items:center;gap:8px">
            <span class="subs-step-num" style="width:20px;height:20px;border-radius:50%;background:#3a3a3a;display:flex;align-items:center;justify-content:center;font-size:10.5px;font-weight:700">1</span>
            <div style="min-width:0;line-height:1.3">
              <strong style="display:block;font-size:11px">Descarga de IA</strong>
              <small style="font-size:9.5px;color:#8a8a8a" data-step-download-status>${whisperReady ? 'Listo' : 'Pendiente'}</small>
            </div>
          </div>
          ${mode !== 'download' ? `
          <div class="subs-step" data-step-transcribe style="flex:1;padding:8px 10px;border-radius:6px;background:#2a2a2a;border:1px solid #3d3d3d;display:flex;align-items:center;gap:8px">
            <span class="subs-step-num" style="width:20px;height:20px;border-radius:50%;background:#3a3a3a;display:flex;align-items:center;justify-content:center;font-size:10.5px;font-weight:700">2</span>
            <div style="min-width:0;line-height:1.3">
              <strong style="display:block;font-size:11px">Transcripción</strong>
              <small style="font-size:9.5px;color:#8a8a8a" data-step-transcribe-status>En espera</small>
            </div>
          </div>` : ''}
        </div>

        <div class="subs-progress-card" style="background:#1e1e1e;border:1px solid #3a3a3a;border-radius:8px;padding:16px">
          <div style="display:flex;align-items:baseline;justify-content:space-between;margin-bottom:8px">
            <span data-subs-label style="font-size:12px;font-weight:600;color:#efefef">Iniciando…</span>
            <span data-subs-percent style="font-size:20px;font-weight:800;color:var(--accent-2);font-variant-numeric:tabular-nums">0%</span>
          </div>

          <div class="progress" style="height:8px;background:#2d2d2d;border-radius:4px;overflow:hidden">
            <div data-subs-bar style="height:100%;width:0%;background:linear-gradient(90deg, var(--accent-2), #8ffcff);border-radius:4px;transition:width 0.25s ease-out"></div>
          </div>

          <div style="display:flex;justify-content:space-between;align-items:center;margin-top:10px;font-size:10px;color:#8a8a8a">
            <span data-subs-detail style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:320px">Preparando componentes…</span>
            <span data-subs-subdetail style="white-space:nowrap;font-variant-numeric:tabular-nums"></span>
          </div>
        </div>
      </div>
    `;

    const footerHtml = mode === 'download'
      ? `<button class="btn btn-danger" data-subs-abort>Cancelar</button>`
      : `<button class="btn" data-modal-close>Cancelar</button><button class="btn btn-primary" data-subs-start>${icon('sparkles', 14)} Iniciar generación</button>`;

    const modal = openModal({
      title: mode === 'download' ? 'Descarga de modelo Whisper IA' : 'Generador de subtítulos con IA',
      width: 520,
      body: bodyHtml,
      footer: footerHtml,
      dismissible: true,
      onMount(api) {
        const configView = api.body.querySelector('[data-subs-config]');
        const progressView = api.body.querySelector('[data-subs-progress]');
        const startBtn = api.foot.querySelector('[data-subs-start]');
        const labelEl = api.body.querySelector('[data-subs-label]');
        const percentEl = api.body.querySelector('[data-subs-percent]');
        const barEl = api.body.querySelector('[data-subs-bar]');
        const detailEl = api.body.querySelector('[data-subs-detail]');
        const subdetailEl = api.body.querySelector('[data-subs-subdetail]');
        const stepDownload = api.body.querySelector('[data-step-download]');
        const stepDownloadStatus = api.body.querySelector('[data-step-download-status]');
        const stepTranscribe = api.body.querySelector('[data-step-transcribe]');
        const stepTranscribeStatus = api.body.querySelector('[data-step-transcribe-status]');

        api.body.querySelectorAll('[data-modal-style]').forEach((btn) => {
          btn.addEventListener('click', () => {
            activeStyle = btn.dataset.modalStyle;
            captionStyle = activeStyle;
            api.body.querySelectorAll('[data-modal-style]').forEach((b) => b.classList.remove('active'));
            btn.classList.add('active');
          });
        });

        const updateProgress = (info) => {
          if (!info) return;
          const pct = Math.min(100, Math.max(0, Math.round(info.percent || 0)));
          if (percentEl) percentEl.textContent = `${pct}%`;
          if (barEl) barEl.style.width = `${pct}%`;
          if (detailEl && info.detail) detailEl.textContent = info.detail;

          if (info.stage === 'download') {
            if (stepDownload) {
              stepDownload.classList.add('active');
              stepDownload.classList.remove('completed');
            }
            if (stepDownloadStatus) stepDownloadStatus.textContent = `${pct}%`;
            if (labelEl) labelEl.textContent = `Descargando modelo Whisper (~40 MB)…`;
            if (pct >= 100) {
              if (stepDownload) {
                stepDownload.classList.remove('active');
                stepDownload.classList.add('completed');
              }
              if (stepDownloadStatus) stepDownloadStatus.textContent = 'Completado';
            }
          } else if (info.stage === 'audio') {
            if (stepDownload) {
              stepDownload.classList.remove('active');
              stepDownload.classList.add('completed');
            }
            if (stepDownloadStatus) stepDownloadStatus.textContent = 'Completado';
            if (stepTranscribe) stepTranscribe.classList.add('active');
            if (labelEl) labelEl.textContent = 'Extrayendo y preparando audio…';
          } else if (info.stage === 'transcribe') {
            if (stepDownload) {
              stepDownload.classList.remove('active');
              stepDownload.classList.add('completed');
            }
            if (stepDownloadStatus) stepDownloadStatus.textContent = 'Completado';
            if (stepTranscribe) {
              stepTranscribe.classList.add('active');
              stepTranscribe.classList.remove('completed');
            }
            if (stepTranscribeStatus) stepTranscribeStatus.textContent = `${pct}%`;
            if (labelEl) labelEl.textContent = `Transcribiendo audio y sincronizando…`;
            if (info.currentChunk && info.totalChunks && subdetailEl) {
              subdetailEl.textContent = `Fragmento ${info.currentChunk}/${info.totalChunks}`;
            }
            if (pct >= 100) {
              if (stepTranscribe) {
                stepTranscribe.classList.remove('active');
                stepTranscribe.classList.add('completed');
              }
              if (stepTranscribeStatus) stepTranscribeStatus.textContent = 'Completado';
            }
          }
        };

        const executeProcess = async () => {
          running = true;
          abortController = new AbortController();
          if (configView) configView.classList.add('hidden');
          if (progressView) progressView.classList.remove('hidden');

          api.foot.innerHTML = `<button class="btn btn-danger" data-subs-abort>Cancelar proceso</button>`;
          const abortBtn = api.foot.querySelector('[data-subs-abort]');
          abortBtn?.addEventListener('click', () => {
            if (abortController) {
              abortController.abort();
              toast('Cancelando generación de subtítulos…', 'info');
            }
          });

          try {
            if (mode === 'download') {
              updateProgress({ stage: 'download', percent: 1, detail: 'Iniciando descarga de Whisper Tiny…' });
              await ensureWhisperPipeline({
                onProgress: updateProgress,
                signal: abortController.signal,
              });
              whisperReady = true;
              updateProgress({ stage: 'download', percent: 100, detail: '¡Modelo Whisper descargado y listo!' });
              if (labelEl) labelEl.textContent = '¡Descarga completada con éxito!';
              toast('Modelo Whisper IA descargado (~40 MB)', 'success');
              setTimeout(() => {
                api.close();
                render();
              }, 1200);
              return;
            }

            const mediaSelect = api.body.querySelector('[data-subs-media]');
            const langSelect = api.body.querySelector('[data-subs-lang]');
            const replaceCheck = api.body.querySelector('[data-subs-replace]');

            const selectedMediaId = mediaSelect?.value || defaultMediaId;
            const targetMedia = state.media.get(selectedMediaId) || videos[0];
            const language = langSelect?.value || 'es';
            const replace = replaceCheck ? replaceCheck.checked : true;

            const res = await generateSubtitlesForVideo({
              media: targetMedia,
              language,
              captionStyle: activeStyle,
              replace,
              onProgress: updateProgress,
              signal: abortController.signal,
            });

            whisperReady = true;
            updateProgress({ stage: 'transcribe', percent: 100, detail: `${res.count} subtítulos insertados en la línea de tiempo` });
            if (labelEl) labelEl.textContent = '¡Subtítulos generados con éxito!';
            toast(`¡${res.count} subtítulos generados!`, 'success', 3000);

            setTimeout(() => {
              api.close();
              render();
            }, 1200);
          } catch (err) {
            if (abortController?.signal?.aborted) {
              toast('Operación cancelada', 'info');
              api.close();
              return;
            }
            console.error('Error generando subtítulos:', err);
            toast('Error: ' + (err?.message || 'Fallo al generar subtítulos'), 'error', 4500);
            if (labelEl) labelEl.textContent = 'Error al generar subtítulos';
            if (detailEl) detailEl.textContent = err?.message || 'Error inesperado';
            api.foot.innerHTML = `<button class="btn btn-primary" data-close-err>Aceptar</button>`;
            api.foot.querySelector('[data-close-err]')?.addEventListener('click', api.close);
          } finally {
            running = false;
          }
        };

        if (mode === 'download') {
          void executeProcess();
        } else {
          startBtn?.addEventListener('click', () => {
            void executeProcess();
          });
        }
      },
    });
  }

  function captionsPanel() {
    const textClips = state.project?.tracks
      .filter((track) => track.type === 'text')
      .flatMap((track) => track.clips.filter((clip) => clip.kind === 'text'))
      .sort((a, b) => a.start - b.start) || [];
    return `
      <div class="details-section" style="margin:2px 12px 6px">Estilo de subtítulo</div>
      <div class="caption-styles">
        ${CAPTION_STYLES.map((style) => `
          <button class="caption-style${captionStyle === style.id ? ' active' : ''}" data-caption-style="${style.id}">
            <span style="font-weight:${style.weight};color:${style.color};${style.background !== 'transparent' ? `background:${style.background};` : ''}">${style.label}</span>
            ${captionStyle === style.id ? icon('check', 11) : ''}
          </button>`).join('')}
      </div>
      <div class="caption-ai-banner ${whisperReady ? 'is-ready' : 'is-pending'}">
        <div class="caption-ai-banner-head">
          <span class="caption-ai-tag">
            ${whisperReady ? `${icon('check', 12)} Dependencia Whisper lista` : `${icon('sparkles', 12)} Requiere descarga (~40 MB)`}
          </span>
          ${!whisperReady ? `<button class="btn btn-secondary btn-small" data-pre-download-subs style="font-size:9.5px;padding:2px 8px">${icon('download', 11)} Descargar</button>` : ''}
        </div>
        <div class="caption-ai-banner-desc">
          ${whisperReady
            ? 'Modelo Whisper Tiny disponible en tu navegador. Genera subtítulos al instante sin conexión.'
            : 'La primera vez se descargará la dependencia Whisper (~40 MB) con barra de progreso del 1 al 100%.'}
        </div>
      </div>
      <div class="caption-actions">
        <button class="btn btn-primary btn-block" data-gen-subs>${icon('sparkles', 14)} Generar subtítulos con IA</button>
        <button class="btn btn-block" data-add-subtitle>${icon('plus', 14)} Añadir subtítulo en el cursor</button>
        <button class="btn btn-block" data-import-srt>${icon('upload', 14)} Importar archivo .srt</button>
      </div>
      ${textClips.length ? `
        <div class="caption-list-head">
          <span>Subtítulos en la línea de tiempo <b>${textClips.length}</b></span>
          <button class="icon-btn" data-clear-subs title="Eliminar todos los subtítulos" style="color:#d07070;width:22px;height:22px">${icon('trash', 12)}</button>
        </div>
        <div class="caption-list">
          ${textClips.map((clip) => `
            <button class="caption-row${state.selection === clip.id ? ' active' : ''}" data-caption-select="${clip.id}">
              <span class="caption-time">${formatTime(clip.start)}</span>
              <span class="caption-text">${escapeHtml(clip.text?.content || '')}</span>
            </button>`).join('')}
        </div>` : ''}
      <div class="modal-note" style="margin:8px 12px 16px">${icon('info', 13)} La transcripción automática procesa el audio localmente con Whisper para máxima privacidad y precisión.</div>`;
  }

  function avatarPanel() {
    return `
      ${panelEmpty('Avatares locales', 'Añade un personaje como capa con fondo de color. Después puedes moverlo, escalarlo y combinarlo con tus videos.', 'avatar')}
      <div class="avatar-grid">
        ${AVATAR_PRESETS.map((preset) => `
          <button class="avatar-card" data-avatar="${preset.id}" title="Añadir ${preset.name}">
            <span class="avatar-art" style="background:${preset.color}">${preset.emoji}</span>
            <span>${preset.name}</span>
          </button>`).join('')}
      </div>
      <div class="modal-note" style="margin:8px 12px 16px">La generación de avatares animados con IA requiere conexión y no está disponible sin conexión.</div>`;
  }

  function adjustPanel() {
    const found = selectedClip();
    const clip = found?.clip;
    if (!clip) {
      return panelEmpty('Nada seleccionado', 'Selecciona un clip de la línea de tiempo para aplicarle un preset de ajuste.', 'adjust');
    }
    const visual = clip.kind === 'video' || clip.kind === 'image';
    if (!visual) {
      return panelEmpty('Clip no visual', 'Los presets de ajuste de color se aplican a clips de video o imagen. Selecciona otro clip.', 'adjust');
    }
    const current = clip.adjust || {};
    const matches = (preset) => {
      if (!preset.adjust) return false;
      const keys = new Set([...Object.keys(preset.adjust), ...Object.keys(current)]);
      for (const key of keys) {
        if (Math.round((preset.adjust[key] ?? defaultAdjustValue(key)) * 10) !== Math.round((current[key] ?? defaultAdjustValue(key)) * 10)) return false;
      }
      return true;
    };
    const all = allAdjustPresets();
    const query = adjustSearch.trim().toLowerCase();
    const activeId = all.find(matches)?.id || 'custom';
    const entries = all.filter((item) => {
      if (query) return item.label.toLowerCase().includes(query);
      return adjustCategory === 'Todos' || item.category === adjustCategory;
    });
    return `
      <div class="cat-rail category-chips">
        ${['Todos', ...ADJUST_CATEGORIES].map((name) => `<button class="category-chip${name === adjustCategory ? ' active' : ''}" data-adjust-category="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="cat-main">
        <div class="adjust-actions">
          <button class="btn btn-small" data-adjust-save>${icon('plus', 13)} Agregar ajuste</button>
          <button class="btn btn-small btn-ghost" data-reset-clip>${icon('rotate', 13)} Restablecer</button>
        </div>
        <label class="search-box filter-search">${icon('search', 13)}<input data-adjust-search placeholder="Buscar ajuste" value="${escapeHtml(adjustSearch)}" /><span class="filter-count">${entries.length}</span></label>
        <div class="adjust-grid">
          ${entries.map((item) => `
            <button class="adjust-card${activeId === item.id ? ' active' : ''}" data-adjust-preset="${item.id}">
              <div class="adjust-preview" style="filter:${item.adjust ? adjustCss(item.adjust) : 'none'}"></div>
              <div class="filter-name"><span>${escapeHtml(item.label)}</span>${activeId === item.id ? icon('check', 11) : ''}</div>
            </button>`).join('')}
        </div>
        <div class="panel-empty" style="padding-top:2px">${activeId === 'custom'
          ? 'Ajuste personalizado: mueve los deslizadores de Detalles y pulsa “Agregar ajuste” para guardarlo en Tuyos.'
          : `Preset “${escapeHtml(all.find((item) => item.id === activeId)?.label || '')}” aplicado al clip.`}</div>
      </div>`;
  }

  function timelineRail() {
    const tracks = state.project?.tracks || [];
    return `
      <div class="rail-section-head"><span>Pistas</span><b>${tracks.length}</b></div>
      <div class="rail-track-list">
        ${tracks.map((track) => {
          const meta = track.type === 'audio' ? 'music' : track.type === 'text' ? 'text' : 'film';
          return `
            <div class="rail-track-row">
              <span class="rail-track-icon">${icon(meta, 14)}</span>
              <span class="rail-track-meta"><strong>${escapeHtml(track.name)}</strong><small>${track.clips.length} clip${track.clips.length === 1 ? '' : 's'}</small></span>
              <span class="rail-track-state">${track.hidden || track.muted ? 'Oculta' : ''}</span>
            </div>`;
        }).join('')}
      </div>
      <button class="rail-action" data-rail-add-text>${icon('plus', 13)} Añadir texto en el cursor</button>
      <div class="rail-note">Arrastra los clips para moverlos, usa los tiradores de los extremos para recortar y pulsa B para dividir en el cursor.</div>`;
  }

  function secondaryRail() {
    const others = state.projects.filter((item) => item.id !== state.project?.id);
    return `
      <div class="rail-section-head"><span>Proyectos secundarios</span><b>${others.length}</b></div>
      ${others.length
        ? `<div class="rail-project-list">${others.map((item) => `
            <button class="rail-project" data-open-project="${item.id}">
              <span class="rail-project-thumb">${item.thumb ? `<img src="${item.thumb}" alt="" />` : icon('film', 18)}</span>
              <span class="rail-project-meta"><strong>${escapeHtml(item.name)}</strong><small>${item.ratio || '16:9'} · ${item.resolution || '1080p'}</small></span>
            </button>`).join('')}</div>`
        : `<div class="rail-note">No hay otros proyectos todavía. Crea uno nuevo desde el menú o vuelve al inicio.</div>`}
      <button class="rail-action" data-rail-home>${icon('home', 13)} Ir al inicio</button>`;
  }

  function bodyForPanel(panel) {
    const media = Array.from(state.media.values()).reverse();
    if (panel === 'media') return mediaPanel(media);
    if (panel === 'audio') return audioPanel(media.filter((item) => item.kind === 'audio'));
    if (panel === 'text') return textPanel();
    if (panel === 'stickers') return stickersPanel();
    if (panel === 'effects') return effectPanel();
    if (panel === 'filters') {
      const found = selectedClip();
      const active = found && ['video', 'image'].includes(found.clip.kind) ? found.clip.filter : null;
      return filterGallery(active, true, filterCategory);
    }
    if (panel === 'transitions') return transitionPanel();
    if (panel === 'captions') {
      if (!whisperCheckedOnce && !whisperChecking) checkWhisperStatus();
      return captionsPanel();
    }
    if (panel === 'adjust') return adjustPanel();
    if (panel === 'templates') {
      return `
        <div class="template-grid">
          ${TEMPLATES.map((template) => `
            <button class="template-card" data-template="${template.id}">
              <div class="template-art" style="background:${template.art};display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;text-align:center;padding:12px">
                <span style="color:#fff;font-weight:900;font-size:14px;line-height:1.15">${escapeHtml(template.content.replace('\n', ' '))}</span>
                <small style="font-size:8px;font-weight:500;color:${template.accent || '#fff'};opacity:.92">${escapeHtml(template.subtitle || '')}</small>
              </div>
              <span>${template.name}</span>
            </button>`).join('')}
        </div>
        <div class="panel-empty">Las plantillas crean un proyecto nuevo con una composición de texto y subtítulo.</div>`;
    }
    if (panel === 'avatar') return avatarPanel();
    if (panel === 'auto') return autoEditPanelHtml();
    return panelEmpty('Panel en construcción', 'Esta sección estará disponible próximamente.', 'layers');
  }

  function renderRail() {
    rail.querySelectorAll('[data-rail]').forEach((item) => {
      const value = item.dataset.rail;
      item.classList.toggle('active', value === railView && value !== 'settings');
    });
    rail.querySelectorAll('[data-rail-group]').forEach((item) => {
      item.classList.toggle('open', openGroup === item.dataset.railGroup);
    });
    rail.querySelectorAll('[data-rail-sub]').forEach((sub) => {
      sub.classList.toggle('hidden', openGroup !== sub.dataset.railSub);
    });
  }

  function renderHeader() {
    if (railView !== 'media') {
      const title = railView === 'timeline' ? 'Línea de tiempo' : 'Proyectos secundarios';
      head.innerHTML = `
        <div class="content-toolbar">
          <h2>${title}</h2>
          <div class="head-actions"><button class="icon-btn" data-help title="Ayuda">${icon('info', 15)}</button></div>
        </div>`;
      return;
    }
    const panel = state.panel;
    if (panel === 'media') {
      if (!state.media.size) {
        head.innerHTML = '';
        return;
      }
      head.innerHTML = `
        <div class="media-top">
          <button class="import-pill" data-import>Importar ${icon('chevronUp', 12)}</button>
          <button class="icon-btn" data-help title="Ayuda sobre este panel">${icon('info', 15)}</button>
        </div>`;
      return;
    }
    head.innerHTML = `
      <div class="content-toolbar">
        <h2>${PANEL_TITLES[panel] || 'Panel'}</h2>
        <div class="head-actions">
          ${['audio', 'avatar'].includes(panel) ? `<button class="icon-btn" data-import title="Importar archivos">${icon('plus', 16)}</button>` : ''}
          <button class="icon-btn" data-help title="Ayuda sobre este panel">${icon('info', 15)}</button>
        </div>
      </div>`;
  }

  function renderFoot() {
    if (railView !== 'media') { foot.innerHTML = ''; return; }
    const panel = state.panel;
    if (panel === 'audio') {
      foot.innerHTML = `
        <div style="display:flex;gap:6px">
          <button class="btn btn-primary" style="flex:1" data-record>${icon('mic', 14)} Grabar</button>
          <button class="btn" style="flex:1" data-import>${icon('import', 14)} Importar</button>
        </div>`;
    } else if (panel === 'text') {
      const canAddText = textTab === 'text';
      foot.innerHTML = canAddText
        ? `<button class="btn btn-primary btn-block" data-add-text>${icon('plus', 14)} Añadir texto</button>`
        : '';
    } else if (panel === 'avatar') {
      foot.innerHTML = `<button class="btn btn-block" data-avatar-upload>${icon('upload', 14)} Subir foto</button>`;
    } else {
      foot.innerHTML = '';
    }
  }

  function render() {
    if (suppressRender) return;
    const top = body.scrollTop;
    renderRail();
    if (railView === 'timeline') {
      head.innerHTML = `
        <div class="content-toolbar">
          <h2>Línea de tiempo</h2>
          <div class="head-actions"><button class="icon-btn" data-help title="Ayuda">${icon('info', 15)}</button></div>
        </div>`;
      body.innerHTML = timelineRail();
      body.classList.remove('cat');
      foot.innerHTML = '';
    } else if (railView === 'secondary') {
      head.innerHTML = `
        <div class="content-toolbar">
          <h2>Proyectos secundarios</h2>
          <div class="head-actions"><button class="icon-btn" data-help title="Ayuda">${icon('info', 15)}</button></div>
        </div>`;
      body.innerHTML = secondaryRail();
      body.classList.remove('cat');
      foot.innerHTML = '';
    } else {
      renderHeader();
      body.innerHTML = bodyForPanel(state.panel);
      if (state.panel === 'auto') wireAutoEditPanel(body);
      body.classList.toggle('cat', Boolean(body.querySelector(':scope > .cat-rail')));
      renderFoot();
    }
    body.scrollTop = top;
    revealActiveChip();
  }

  function debouncedCatalogRender(selector) {
    clearTimeout(filterSearchTimer);
    filterSearchTimer = setTimeout(() => {
      render();
      const input = body.querySelector(selector);
      if (input) {
        input.focus();
        input.setSelectionRange(input.value.length, input.value.length);
      }
    }, 180);
  }

  function revealActiveChip() {
    body.querySelectorAll('.category-chips').forEach((row) => {
      const active = row.querySelector('.category-chip.active');
      if (!active) return;
      const vertical = getComputedStyle(row).flexDirection === 'column';
      const rowRect = row.getBoundingClientRect();
      const chipRect = active.getBoundingClientRect();
      if (vertical) {
        const target = row.scrollTop + chipRect.top - rowRect.top - (rowRect.height - chipRect.height) / 2;
        row.scrollTop = Math.max(0, Math.min(target, row.scrollHeight - row.clientHeight));
        return;
      }
      const target = row.scrollLeft + chipRect.left - rowRect.left - (rowRect.width - chipRect.width) / 2;
      row.scrollLeft = Math.max(0, Math.min(target, row.scrollWidth - row.clientWidth));
    });
  }

  function openHelp() {
    openModal({
      title: PANEL_TITLES[state.panel] || 'Ayuda',
      width: 420,
      body: `<div class="modal-note" style="margin:0">${escapeHtml(PANEL_HELP[state.panel] || '')}</div>`,
      footer: `<button class="btn btn-primary" data-help-close>Entendido</button>`,
      onMount(modal) {
        modal.foot.querySelector('[data-help-close]').addEventListener('click', modal.close);
      },
    });
  }

  host.addEventListener('click', async (event) => {
    const railItem = event.target.closest('[data-rail]');
    if (railItem) {
      const value = railItem.dataset.rail;
      if (value === 'settings') {
        openModal({
          title: 'Ajustes',
          width: 430,
          body: `<div class="modal-note" style="margin:0">openCUP funciona sin cuenta. Los proyectos y medios se guardan en el almacenamiento local de este navegador.</div>`,
          footer: `<button class="btn btn-primary" data-close-settings>Listo</button>`,
          onMount(modal) {
            modal.foot.querySelector('[data-close-settings]').addEventListener('click', modal.close);
          },
        });
        return;
      }
      if (value === 'media') {
        if (railView !== 'media' || state.panel !== 'media') {
          railView = 'media';
          setPanel('media');
        }
        return;
      }
      railView = value;
      openGroup = null;
      render();
      return;
    }

    const group = event.target.closest('[data-rail-group]');
    if (group) {
      openGroup = openGroup === group.dataset.railGroup ? null : group.dataset.railGroup;
      renderRail();
      return;
    }

    const railFilter = event.target.closest('[data-rail-filter]');
    if (railFilter) {
      mediaFilter = railFilter.dataset.railFilter;
      railView = 'media';
      openGroup = null;
      if (state.panel !== 'media') setPanel('media'); else render();
      toast(mediaFilter === 'all' ? 'Mostrando todos los medios' : `Mostrando ${railFilter.textContent.toLowerCase()}`, 'info', 1300);
      return;
    }

    const aiTool = event.target.closest('[data-ai-tool]');
    if (aiTool) {
      unavailable(
        `${aiTool.textContent} con IA`,
        'La generación con IA necesita un servicio en la nube. openCUP es un editor local y no incluye estas herramientas; puedes importar tus propios medios o usar las plantillas y los stickers.',
      );
      return;
    }

    if (event.target.closest('[data-space-create]')) {
      unavailable('Crear espacio de trabajo', 'Los espacios compartidos requieren una cuenta en la nube. En esta versión local, cada proyecto se guarda únicamente en tu navegador.');
      return;
    }
    if (event.target.closest('[data-space-info]')) {
      unavailable('Espacios', 'Un espacio es una carpeta compartida para trabajar con otras personas. openCUP no tiene cuentas ni colaboración: todo se queda en este equipo.');
      return;
    }
    if (event.target.closest('[data-library-media]')) {
      railView = 'media';
      mediaFilter = 'all';
      openGroup = null;
      setPanel('media');
      return;
    }
    if (event.target.closest('[data-library-templates]')) {
      setPanel('templates');
      return;
    }
    if (event.target.closest('[data-rail-add-text]')) {
      addTextClip('Tu texto aquí', { start: state.playhead });
      return;
    }
    if (event.target.closest('[data-rail-home]')) {
      goHome();
      return;
    }
    const openProjectButton = event.target.closest('[data-open-project]');
    if (openProjectButton) {
      await saveNow();
      await openProject(openProjectButton.dataset.openProject);
      return;
    }
    if (event.target.closest('[data-open-capcut]')) {
      openCapCutImportDialog();
      return;
    }
    if (event.target.closest('[data-generate]')) {
      unavailable('Generar con IA', 'La generación de imágenes y videos con IA requiere un servicio en la nube. Puedes importar tus propios archivos, usar las plantillas o crear títulos y stickers.');
      return;
    }
    if (event.target.closest('[data-load-system-fonts]')) {
      toast('Detectando fuentes del sistema…', 'info', 1600);
      loadSystemFonts().then((fonts) => {
        toast(fonts.length ? `${fonts.length} fuentes nuevas detectadas` : 'No se detectaron fuentes adicionales (o el navegador no lo permite)', fonts.length ? 'success' : 'warn');
        render();
      });
      return;
    }

    const help = event.target.closest('[data-help]');
    if (help) { openHelp(); return; }

    const importButton = event.target.closest('[data-import]');
    if (importButton) { pickFiles('media', 'video/*,image/*,audio/*,.mkv,.mov,.m4v,.webm,.mp3,.wav,.m4a,.aac,.ogg', true); return; }

    const record = event.target.closest('[data-record]');
    if (record) { recordVoice(); return; }

    const link = event.target.closest('[data-panel-link]');
    if (link) { railView = 'media'; setPanel(link.dataset.panelLink); return; }

    const presetButton = event.target.closest('[data-preset]');
    if (presetButton) {
      const style = TEXT_STYLES[Number(presetButton.dataset.preset)];
      if (style) {
        addTextClip(style.sample || 'Tu texto aquí', {
          start: state.playhead,
          preset: { text: style.patch },
        });
        toast(`Texto “${style.label}” añadido en el cursor`, 'success', 1600);
      }
      return;
    }

    const textTabButton = event.target.closest('[data-text-tab]');
    if (textTabButton) {
      textTab = textTabButton.dataset.textTab;
      render();
      return;
    }

    const textSubTabButton = event.target.closest('[data-text-subtab]');
    if (textSubTabButton) {
      textSubTab = textSubTabButton.dataset.textSubtab;
      render();
      return;
    }

    const bubbleButton = event.target.closest('[data-text-bubble]');
    if (bubbleButton) {
      const bubble = TEXT_BUBBLES.find((item) => item.id === bubbleButton.dataset.textBubble);
      if (bubble) applyTextPatch(bubble.patch, `Burbuja “${bubble.label}” aplicada`);
      return;
    }

    const textEffectButton = event.target.closest('[data-text-effect]');
    if (textEffectButton) {
      const effect = TEXT_EFFECTS.find((item) => item.id === textEffectButton.dataset.textEffect);
      if (effect) applyTextPatch(effect.patch, `Efecto “${effect.label}” aplicado`);
      return;
    }

    const textAnimationButton = event.target.closest('[data-text-animation]');
    if (textAnimationButton) {
      const id = textAnimationButton.dataset.textAnimation;
      applyTextAnimation(id === 'none' ? null : TEXT_ANIMATIONS.find((item) => item.id === id));
      return;
    }

    if (event.target.closest('[data-tts-play]')) {
      speakTts();
      return;
    }
    if (event.target.closest('[data-tts-generate]')) {
      void generateKokoro();
      return;
    }
    if (event.target.closest('[data-tts-stop]')) {
      speech?.cancel();
      toast('Vista previa detenida', 'info', 1200);
      return;
    }
    if (event.target.closest('[data-tts-use-text]')) {
      const area = body.querySelector('[data-tts-text]');
      const clip = activeTextClip();
      if (area && clip) {
        area.value = clip.text?.content || '';
        area.focus();
        area.select();
      } else {
        toast('Selecciona un texto de la línea de tiempo', 'warning');
      }
      return;
    }

    const addText = event.target.closest('[data-add-text]');
    if (addText) {
      addTextClip('Tu texto aquí', { start: state.playhead });
      return;
    }

    const stickerCategoryButton = event.target.closest('[data-sticker-category]');
    if (stickerCategoryButton) {
      stickerCategory = stickerCategoryButton.dataset.stickerCategory;
      stickerSearch = '';
      render();
      return;
    }

    const effectCategoryButton = event.target.closest('[data-effect-category]');
    if (effectCategoryButton) {
      effectCategory = effectCategoryButton.dataset.effectCategory;
      effectSearch = '';
      render();
      return;
    }

    const effectSourceButton = event.target.closest('[data-effect-source]');
    if (effectSourceButton) {
      effectSource = effectSourceButton.dataset.effectSource;
      render();
      return;
    }

    const capcutCategoryButton = event.target.closest('[data-capcut-category]');
    if (capcutCategoryButton) {
      capcutCategory = capcutCategoryButton.dataset.capcutCategory;
      capcutSearch = '';
      render();
      return;
    }

    const clearFxButton = event.target.closest('[data-clear-fx]');
    if (clearFxButton) {
      if (state.project) {
        state.project.effectClips = [];
        saveNow();
        window.dispatchEvent(new CustomEvent('opencup:glfx'));
        render();
        toast('Efectos quitados de la línea de tiempo', 'success', 1800);
      }
      return;
    }

    const capcutEffectButton = event.target.closest('[data-capcut-effect]');
    if (capcutEffectButton) {
      const capcutClip = selectedClip();
      if (!capcutClip || !['video', 'image'].includes(capcutClip.clip.kind)) {
        toast('Selecciona un clip de video o imagen para aplicar el efecto', 'warning');
        return;
      }
      const capcutName = capcutEffectButton.dataset.capcutName || '';
      const mapped = capcutToLocal(capcutName);
      const glId = capcutEffectToGL(capcutName);
      state.glEffect = glId;
      state.glAmount = 1;
      if (state.project) {
        const fxClip = {
          id: uid('fx'),
          name: capcutName || 'Efecto',
          glEffect: glId,
          start: Math.max(0, state.playhead),
          duration: 5,
          amount: 1,
        };
        state.project.effectClips = [...(state.project.effectClips ?? []), fxClip];
        saveNow();
        render();
      }
      updateClip(capcutClip.clip.id, { effect: glId ? 'none' : mapped, glEffect: glId || null });
      seek(state.playhead);
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
      if (glId) {
        toast(`“${capcutName}”: shader ${glId} añadido a la línea de tiempo`, 'success', 2200);
      } else if (mapped === 'none') {
        toast(`“${capcutName}”: shader de CapCut pendiente de portar`, 'warning', 2400);
      } else {
        toast(`“${capcutName}” → ${EFFECTS[mapped]?.label || mapped}`, 'success', 2000);
      }
      return;
    }

    const filterCategoryButton = event.target.closest('[data-filter-category]');
    if (filterCategoryButton) {
      filterCategory = filterCategoryButton.dataset.filterCategory;
      filterSearch = '';
      render();
      return;
    }

    const filterSourceButton = event.target.closest('[data-filter-source]');
    if (filterSourceButton) {
      filterSource = filterSourceButton.dataset.filterSource;
      render();
      return;
    }

    const capcutFilterCategoryButton = event.target.closest('[data-capcut-filter-category]');
    if (capcutFilterCategoryButton) {
      capcutFilterCategory = capcutFilterCategoryButton.dataset.capcutFilterCategory;
      capcutFilterSearch = '';
      render();
      return;
    }

    const transitionSourceButton = event.target.closest('[data-transition-source]');
    if (transitionSourceButton) {
      transitionSource = transitionSourceButton.dataset.transitionSource;
      render();
      return;
    }

    const capcutTransitionCategoryButton = event.target.closest('[data-capcut-transition-category]');
    if (capcutTransitionCategoryButton) {
      capcutTransitionCategory = capcutTransitionCategoryButton.dataset.capcutTransitionCategory;
      capcutTransitionSearch = '';
      render();
      return;
    }

    const stickerSourceButton = event.target.closest('[data-sticker-source]');
    if (stickerSourceButton) {
      stickerSource = stickerSourceButton.dataset.stickerSource;
      render();
      return;
    }

    const capcutStickerCategoryButton = event.target.closest('[data-capcut-sticker-category]');
    if (capcutStickerCategoryButton) {
      capcutStickerCategory = capcutStickerCategoryButton.dataset.capcutStickerCategory;
      capcutStickerSearch = '';
      render();
      return;
    }

    const capcutStickerButton = event.target.closest('[data-capcut-sticker]');
    if (capcutStickerButton) {
      const item = CAPCUT_STICKER_ITEMS.find((entry) => entry.id === capcutStickerButton.dataset.capcutSticker);
      if (item) {
        const clip = addStickerClip(item.name, {
          start: state.playhead,
          duration: 4,
          preset: { src: item.img, srcFallback: item.url || null, size: 460 },
        });
        if (clip) {
          toast(`Sticker “${item.name}” añadido`, 'success', 1600);
          prepareStickerSrc(item.img)
            .then((clean) => {
              if (clean && clean !== item.img) updateClip(clip.id, { sticker: { ...clip.sticker, src: clean, srcFallback: null } });
            })
            .catch(() => { /* se queda la imagen original */ });
        }
      }
      return;
    }

    const capcutTextCategoryButton = event.target.closest('[data-capcut-text-category]');
    if (capcutTextCategoryButton) {
      capcutTextCategory = capcutTextCategoryButton.dataset.capcutTextCategory;
      capcutTextSearch = '';
      render();
      return;
    }

    const capcutTextButton = event.target.closest('[data-capcut-text]');
    if (capcutTextButton) {
      const item = CAPCUT_TEXT_ITEMS.find((entry) => entry.id === capcutTextButton.dataset.capcutText);
      if (item) toast(`“${item.name}”: plantilla de texto de CapCut pendiente`, 'warning', 2200);
      return;
    }

    const transitionCategoryButton = event.target.closest('[data-transition-category]');
    if (transitionCategoryButton) {
      transitionCategory = transitionCategoryButton.dataset.transitionCategory;
      render();
      return;
    }

    const adjustCategoryButton = event.target.closest('[data-adjust-category]');
    if (adjustCategoryButton) {
      adjustCategory = adjustCategoryButton.dataset.adjustCategory;
      adjustSearch = '';
      render();
      return;
    }

    const adjustPresetButton = event.target.closest('[data-adjust-preset]');
    if (adjustPresetButton) {
      applyAdjustPreset(adjustPresetButton.dataset.adjustPreset);
      return;
    }

    const adjustSaveButton = event.target.closest('[data-adjust-save]');
    if (adjustSaveButton) {
      void saveCurrentAdjust();
      return;
    }

    const stickerButton = event.target.closest('[data-sticker]');
    if (stickerButton) {
      const sticker = stickerButton.dataset.sticker;
      if (sticker) {
        addStickerClip(sticker, {
          start: state.playhead,
          duration: 4,
          preset: { size: 220 },
        });
        toast('Sticker añadido en el cursor', 'success', 1400);
      }
      return;
    }

    const effectButton = event.target.closest('[data-effect]');
    if (effectButton) {
      const found = selectedClip();
      if (!found || !['video', 'image'].includes(found.clip.kind)) {
        toast('Selecciona un clip de video o imagen para aplicar el efecto', 'warning');
        return;
      }
      updateClip(found.clip.id, { effect: effectButton.dataset.effect });
      toast(effectButton.dataset.effect === 'none' ? 'Efecto eliminado' : 'Efecto aplicado', 'success', 1400);
      return;
    }

    const captionStyleButton = event.target.closest('[data-caption-style]');
    if (captionStyleButton) {
      captionStyle = captionStyleButton.dataset.captionStyle;
      const style = CAPTION_STYLES.find((item) => item.id === captionStyle) || CAPTION_STYLES[0];
      const found = selectedClip();
      if (found?.clip.kind === 'text') {
        updateClip(found.clip.id, {
          text: {
            size: style.size,
            weight: style.weight,
            color: style.color,
            background: style.background,
            shadow: style.shadow,
          },
        });
        toast(`Estilo “${style.label}” aplicado al subtítulo`, 'success', 1600);
      }
      render();
      return;
    }

    const captionSelect = event.target.closest('[data-caption-select]');
    if (captionSelect) {
      selectClip(captionSelect.dataset.captionSelect);
      return;
    }

    const avatarPreset = event.target.closest('[data-avatar]');
    if (avatarPreset) {
      const preset = AVATAR_PRESETS.find((item) => item.id === avatarPreset.dataset.avatar);
      if (!preset) return;
      addStickerClip(preset.emoji, {
        start: state.playhead,
        duration: 5,
        x: -28,
        y: 24,
        scale: 1,
        preset: { size: 190, background: preset.color, shape: 'circle' },
      });
      toast(`Avatar “${preset.name}” añadido`, 'success', 1600);
      return;
    }

    const filterButton = event.target.closest('[data-filter]');
    if (filterButton) { applyFilter(filterButton.dataset.filter); return; }

    const transitionButton = event.target.closest('[data-transition]');
    if (transitionButton) { applyTransition(transitionButton.dataset.transition); return; }

    const capcutFilterButton = event.target.closest('[data-capcut-filter]');
    if (capcutFilterButton) {
      const item = CAPCUT_FILTER_ITEMS.find((entry) => entry.id === capcutFilterButton.dataset.capcutFilter);
      if (item) {
        const mapped = capcutFilterToLocal(item);
        applyFilter(mapped);
        if (mapped === 'none') toast(`“${item.name}”: filtro de CapCut pendiente de LUT`, 'warning', 2200);
      }
      return;
    }

    const capcutTransitionButton = event.target.closest('[data-capcut-transition]');
    if (capcutTransitionButton) {
      const item = CAPCUT_TRANSITION_ITEMS.find((entry) => entry.id === capcutTransitionButton.dataset.capcutTransition);
      if (item) {
        const mapped = capcutTransitionToLocal(item.name);
        applyTransition(mapped);
        toast(`“${item.name}” → ${TRANSITIONS.find((t) => t.id === mapped)?.label || mapped}`, 'success', 1800);
      }
      return;
    }

    const templateButton = event.target.closest('[data-template]');
    if (templateButton) {
      const template = TEMPLATES.find((item) => item.id === templateButton.dataset.template);
      if (template) void applyTemplate(template);
      return;
    }

    const subtitle = event.target.closest('[data-add-subtitle]');
    if (subtitle) {
      const style = CAPTION_STYLES.find((item) => item.id === captionStyle) || CAPTION_STYLES[0];
      addTextClip('Escribe tu subtítulo', {
        start: state.playhead,
        duration: 3,
        preset: {
          size: style.size,
          weight: style.weight,
          color: style.color,
          background: style.background,
          shadow: style.shadow,
        },
      });
      setPanel('text');
      return;
    }

    const srt = event.target.closest('[data-import-srt]');
    if (srt) { pickFiles('srt', '.srt,text/plain', false); return; }

    const genSubs = event.target.closest('[data-gen-subs]');
    if (genSubs) {
      if (!state.project) return;
      openSubtitleModal({ mode: 'generate' });
      return;
    }

    const preDownload = event.target.closest('[data-pre-download-subs]');
    if (preDownload) {
      if (!state.project) return;
      openSubtitleModal({ mode: 'download' });
      return;
    }

    const clearSubs = event.target.closest('[data-clear-subs]');
    if (clearSubs) {
      confirmDialog({
        title: '¿Eliminar todos los subtítulos?',
        message: 'Se borrarán todos los clips de texto de subtítulos de la línea de tiempo. Esta acción se puede deshacer con Ctrl+Z.',
        confirmLabel: 'Eliminar todos',
        danger: true,
      }).then((confirmed) => {
        if (!confirmed) return;
        const count = clearSubtitleClips();
        if (count > 0) {
          toast(`${count} subtítulo${count === 1 ? '' : 's'} eliminado${count === 1 ? '' : 's'}`, 'info');
          render();
        }
      });
      return;
    }

    const avatar = event.target.closest('[data-avatar-upload]');
    if (avatar) { pickFiles('avatar', 'image/*', false); return; }

    const reset = event.target.closest('[data-reset-clip]');
    if (reset) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, {
        filter: 'none',
        effect: 'none',
        transition: 'none',
        transitionDuration: 0.6,
        adjust: { brightness: 100, contrast: 100, saturate: 100 },
        opacity: 1,
        scale: 1,
        rotate: 0,
        x: 0,
        y: 0,
        volume: 1,
        speed: 1,
        fadeIn: 0,
        fadeOut: 0,
      });
      return;
    }

    const card = event.target.closest('[data-media-id]');
    if (card) {
      const media = state.media.get(card.dataset.mediaId);
      if (!media) return;
      const clip = addMediaToTimeline(media.id, { start: state.playhead });
      if (clip) toast(`${media.kind === 'audio' ? 'Audio' : media.kind === 'image' ? 'Imagen' : 'Video'} añadido a la línea de tiempo`, 'success', 1600);
    }
  });

  body.addEventListener('input', (event) => {
    const target = event.target;
    if (target.matches('[data-filter-search]')) {
      filterSearch = target.value;
      debouncedCatalogRender('[data-filter-search]');
      return;
    }
    if (target.matches('[data-capcut-filter-search]')) {
      capcutFilterSearch = target.value;
      debouncedCatalogRender('[data-capcut-filter-search]');
      return;
    }
    if (target.matches('[data-capcut-transition-search]')) {
      capcutTransitionSearch = target.value;
      debouncedCatalogRender('[data-capcut-transition-search]');
      return;
    }
    if (target.matches('[data-capcut-sticker-search]')) {
      capcutStickerSearch = target.value;
      debouncedCatalogRender('[data-capcut-sticker-search]');
      return;
    }
    if (target.matches('[data-capcut-text-search]')) {
      capcutTextSearch = target.value;
      debouncedCatalogRender('[data-capcut-text-search]');
      return;
    }
    if (target.matches('[data-effect-search]')) {
      effectSearch = target.value;
      debouncedCatalogRender('[data-effect-search]');
      return;
    }
    if (target.matches('[data-capcut-search]')) {
      capcutSearch = target.value;
      debouncedCatalogRender('[data-capcut-search]');
      return;
    }
    if (target.matches('[data-sticker-search]')) {
      stickerSearch = target.value;
      debouncedCatalogRender('[data-sticker-search]');
      return;
    }
    if (target.matches('[data-adjust-search]')) {
      adjustSearch = target.value;
      debouncedCatalogRender('[data-adjust-search]');
      return;
    }
    if (target.matches('[data-media-search]')) {
      searchQuery = target.value;
      const query = searchQuery.trim().toLowerCase();
      for (const card of body.querySelectorAll('[data-media-id]')) {
        card.style.display = !query || (card.dataset.name || '').includes(query) ? '' : 'none';
      }
      return;
    }
    if (target.matches('[data-adjust-key]')) {
      const found = selectedClip();
      if (!found) return;
      const key = target.dataset.adjustKey;
      const factor = Number(target.dataset.factor || 1);
      const value = Number(target.value) * factor;
      suppressRender = true;
      if (key.startsWith('adjust.')) {
        updateClip(found.clip.id, { adjust: { [key.slice(7)]: value } }, { history: false });
      } else {
        updateClip(found.clip.id, { [key]: value }, { history: false });
      }
      suppressRender = false;
      for (const readout of body.querySelectorAll('[data-adj-readout]')) {
        if (readout.dataset.adjReadout === key) readout.textContent = adjustReadout(key, value);
      }
      return;
    }
    if (target.matches('[data-tts-rate]')) {
      ttsRate = Number(target.value) || 1;
      const readout = body.querySelector('[data-tts-rate-readout]');
      if (readout) readout.textContent = `${ttsRate.toFixed(2)}×`;
      return;
    }
    if (target.matches('[data-tts-pitch]')) {
      ttsPitch = Number(target.value) || 1;
      const readout = body.querySelector('[data-tts-pitch-readout]');
      if (readout) readout.textContent = ttsPitch.toFixed(2);
    }
  });

  body.addEventListener('change', (event) => {
    if (event.target.matches('[data-tts-voice]')) {
      ttsVoice = event.target.value;
    }
  });

  body.addEventListener('dragstart', (event) => {
    const card = event.target.closest('[data-media-id]');
    if (!card) return;
    event.dataTransfer.effectAllowed = 'copy';
    event.dataTransfer.setData('application/x-openCUP-media', card.dataset.mediaId);
    event.dataTransfer.setData('text/plain', card.dataset.mediaId);
  });

  body.addEventListener('dragover', (event) => {
    if (!['media', 'audio', 'avatar'].includes(state.panel)) return;
    if (!Array.from(event.dataTransfer?.types || []).includes('Files')) return;
    event.preventDefault();
    body.querySelector('.media-drop')?.classList.add('drag');
  });

  body.addEventListener('dragleave', (event) => {
    if (event.relatedTarget && body.contains(event.relatedTarget)) return;
    body.querySelector('.media-drop')?.classList.remove('drag');
  });

  body.addEventListener('drop', async (event) => {
    if (!Array.from(event.dataTransfer?.types || []).includes('Files')) return;
    event.preventDefault();
    body.querySelector('.media-drop')?.classList.remove('drag');
    await importFiles(event.dataTransfer.files);
  });

  fileInput.addEventListener('change', async () => {
    const files = Array.from(fileInput.files || []);
    fileInput.value = '';
    if (!files.length) return;
    if (fileMode === 'srt') await importSrt(files[0]);
    else if (fileMode === 'avatar') {
      await importFiles(files);
      setPanel('media');
    } else {
      await importFiles(files);
    }
  });

  subscriptions.push(on('panel', () => {
    if (railView === 'timeline' || railView === 'secondary') railView = 'media';
    render();
  }));
  subscriptions.push(on('project', render));
  subscriptions.push(on('selection', render));
  let importTimer = null;
  function startImportTimer() {
    if (importTimer) return;
    importTimer = setInterval(() => { if (state.importProgress) render(); else stopImportTimer(); }, 500);
  }
  function stopImportTimer() { if (importTimer) { clearInterval(importTimer); importTimer = null; } }

  subscriptions.push(on('importing', () => {
    if (state.importProgress) startImportTimer(); else stopImportTimer();
    render();
  }));

  render();

  return {
    element: host,
    destroy() {
      stopImportTimer();
      subscriptions.forEach((off) => off());
      speech?.removeEventListener?.('voiceschanged', onVoicesChanged);
      speech?.cancel();
    },
  };
}
function detailRange(label, prop, inputValue, min, max, step, factor, readout, unit = '') {
  return `
    <div class="prop">
      <span class="tx-lbl">${label}</span>
      <input type="range" data-prop="${prop}" data-factor="${factor}" data-unit="${unit}" min="${min}" max="${max}" step="${step}" value="${inputValue}" />
      <b class="tx-val" data-readout="${prop}">${readout}${unit}</b>
    </div>`;
}

function numberField(label, prop, value, min, max, step = 1) {
  return `
    <div>
      <span class="muted" style="font-size:9.5px;display:block;margin-bottom:4px">${label}</span>
      <input class="input" type="number" data-prop-number="${prop}" min="${min}" max="${max}" step="${step}" value="${Math.round(value * 100) / 100}" />
    </div>`;
}

/* ---------------- Controles de Detalles con estética de CapCut ----------------
   Fila: etiqueta a la izquierda, control a la derecha y, al fondo del todo,
   los tres botones de fotograma clave (‹ ◇ ›) de esa propiedad. */

function toRaw(stored, factor) {
  return Math.round((stored / (factor || 1)) * 1000) / 1000;
}

function baseValueFor(clip, prop) {
  if (prop.startsWith('adjust.')) {
    const key = prop.slice(7);
    const fallback = key === 'brightness' || key === 'contrast' || key === 'saturate' ? 100 : 0;
    return clip.adjust?.[key] ?? fallback;
  }
  switch (prop) {
    case 'x': return clip.x || 0;
    case 'y': return clip.y || 0;
    case 'scale': return clip.scale ?? 1;
    case 'scaleX': return clip.scaleX ?? clip.scale ?? 1;
    case 'scaleY': return clip.scaleY ?? clip.scale ?? 1;
    case 'rotate': return clip.rotate || 0;
    case 'opacity': return clip.opacity ?? 1;
    case 'text.opacity': return clip.text?.opacity ?? 1;
    case 'volume': return clip.volume ?? 1;
    case 'speed': return clip.speed ?? 1;
    default: return 0;
  }
}

/* Valor que se muestra en los controles: el efectivo en el cabezal actual. */
function effValue(clip, prop, base) {
  return hasKeyframes(clip, prop) ? keyframeValue(clip, prop, state.playhead, base) : base;
}

function spinBox(prop, { raw, min, max, step = 1, unit = '', factor = 1, prefix = '', cls = '' }) {
  return `<label class="spin ${cls}">
    ${prefix ? `<span class="spin-prefix">${prefix}</span>` : ''}
    <input type="number" data-spin="${prop}" data-spin-factor="${factor}" data-spin-unit="${unit}"
      min="${min}" max="${max}" step="${step}" value="${raw}" />
    ${unit ? `<span class="spin-unit">${unit}</span>` : ''}
  </label>`;
}

function kfControls(clip, prop) {
  const props = prop.split(',');
  const count = props.reduce((sum, item) => sum + keyframeCount(clip, item), 0);
  const active = count > 0 && props.every((item) => keyframeIndexAt(clip, item, state.playhead) >= 0);
  return `<span class="kf-ctl${count ? ' has' : ''}">
    <button class="kf-btn" type="button" data-kf-nav="-1" data-kf-prop="${prop}" title="Fotograma clave anterior" ${count ? '' : 'disabled'}>${icon('chevronLeft', 11)}</button>
    <button class="kf-btn kf-add${active ? ' on' : ''}" type="button" data-kf-toggle="${prop}" title="${active ? 'Quitar fotograma clave' : 'Agregar fotograma clave'}" aria-pressed="${active}"><i></i></button>
    <button class="kf-btn" type="button" data-kf-nav="1" data-kf-prop="${prop}" title="Fotograma clave siguiente" ${count ? '' : 'disabled'}>${icon('chevronRight', 11)}</button>
  </span>`;
}

function dtSliderRow(label, prop, baseStored, { min, max, step = 1, factor = 1, unit = '', clip }) {
  const raw = toRaw(effValue(clip, prop, baseStored), factor);
  return `<div class="dt-row stacked dt-slider">
    <span class="dt-lbl">${label}</span>
    <div class="dt-line">
      <input class="dt-range" type="range" data-prop="${prop}" data-factor="${factor}" data-unit="${unit}" min="${min}" max="${max}" step="${step}" value="${raw}" />
      ${spinBox(prop, { raw, min, max, step, unit, factor, cls: 'spin-val' })}
      ${kfControls(clip, prop)}
    </div>
  </div>`;
}

function canvasSize() {
  const base = RESOLUTIONS[state.project?.resolution] || RESOLUTIONS['1080p'];
  const aspect = ASPECTS[state.project?.ratio] || 16 / 9;
  if (aspect >= base.width / base.height) return { width: base.width, height: base.width / aspect };
  return { width: base.height * aspect, height: base.height };
}

/* Tamaño del clip en el lienzo (%), para alinearlo como en CapCut. */
function layerSizePct(clip) {
  const { width, height } = canvasSize();
  const aspect = width / height;
  const box = clipLayerBox(clip, width, height, state.playhead);
  if (box) {
    return {
      wPct: (box.width * (box.scaleX ?? box.scale) / width) * 100,
      hPct: (box.height * (box.scaleY ?? box.scale) / height) * 100,
      extraX: (clip.text?.x || clip.sticker?.x || 0),
      extraY: (clip.text?.y || clip.sticker?.y || 0),
    };
  }
  const media = clip.mediaId ? state.media?.get(clip.mediaId) : null;
  const mediaAspect = media && media.width && media.height ? media.width / media.height : aspect;
  const tr = transformAt(clip, state.playhead);
  let mediaWidth;
  let mediaHeight;
  if (mediaAspect >= aspect) {
    mediaWidth = width;
    mediaHeight = width / mediaAspect;
  } else {
    mediaHeight = height;
    mediaWidth = height * mediaAspect;
  }
  return {
    wPct: (mediaWidth * tr.scaleX / width) * 100,
    hPct: (mediaHeight * tr.scaleY / height) * 100,
    extraX: 0,
    extraY: 0,
  };
}

function alignPatch(clip, mode) {
  const { wPct, hPct, extraX, extraY } = layerSizePct(clip);
  switch (mode) {
    case 'left': return { x: Math.round((-wPct / 2 - extraX) * 100) / 100 };
    case 'center': return { x: Math.round((-extraX) * 100) / 100 };
    case 'right': return { x: Math.round((wPct / 2 - extraX) * 100) / 100 };
    case 'top': return { y: Math.round((-hPct / 2 - extraY) * 100) / 100 };
    case 'middle': return { y: Math.round((-extraY) * 100) / 100 };
    case 'bottom': return { y: Math.round((hPct / 2 - extraY) * 100) / 100 };
    default: return {};
  }
}

/* Patch para escribir una propiedad respetando los fotogramas clave:
   si la prop ya tiene keyframes, se inserta/actualiza uno en el cabezal. */
function writeClipProp(clip, prop, value) {
  if (hasKeyframes(clip, prop)) return { keyframes: nextKeyframeStore(clip, prop, state.playhead, value) };
  if (prop === 'text.opacity') return { text: { opacity: value } };
  return { [prop]: value };
}

function alignBar() {
  const modes = [
    ['left', 'Alinear a la izquierda', 'alignLeft'],
    ['center', 'Alinear al medio', 'alignCenterH'],
    ['right', 'Alinear a la derecha', 'alignRight'],
    ['top', 'Alinear arriba', 'alignTop'],
    ['middle', 'Alinear al centro', 'alignCenterV'],
    ['bottom', 'Alinear abajo', 'alignBottom'],
  ];
  return `<div class="align-bar" role="toolbar" aria-label="Alinear en el lienzo">
    ${modes.map(([mode, label, name]) => `<button class="align-btn" type="button" data-align="${mode}" title="${label}">${icon(name, 14)}</button>`).join('')}
  </div>`;
}

/* Filas del bloque Transformación (compartidas por imagen/vídeo, sticker y texto). */
function transformInner(clip, { text = false } = {}) {
  const uniform = clip.uniform !== false;
  const scaleRow = uniform
    ? dtSliderRow('Escala', 'scale', clip.scale ?? 1, { min: 10, max: 400, factor: 0.01, unit: '%', clip })
    : [
      dtSliderRow('Escala X', 'scaleX', clip.scaleX ?? clip.scale ?? 1, { min: 10, max: 400, factor: 0.01, unit: '%', clip }),
      dtSliderRow('Escala Y', 'scaleY', clip.scaleY ?? clip.scale ?? 1, { min: 10, max: 400, factor: 0.01, unit: '%', clip }),
    ].join('');
  const rotate = effValue(clip, 'rotate', clip.rotate || 0);
  const dialRot = Math.round(((rotate % 360) + 360) % 360);
  const flipRow = text ? `
    <div class="tx-chip-row">
      <button class="tx-chip${clip.text?.flipX ? ' active' : ''}" data-text-toggle="flipX">⇋ Voltear horizontal</button>
      <button class="tx-chip${clip.text?.flipY ? ' active' : ''}" data-text-toggle="flipY">⇵ Voltear vertical</button>
    </div>` : '';
  return `
    ${scaleRow}
    <div class="dt-row">
      <span class="dt-lbl">Escala uniforme</span>
      <div class="dt-ctl end"><button class="switch ${uniform ? 'on' : ''}" type="button" data-uniform aria-pressed="${uniform}" aria-label="Escala uniforme"></button></div>
    </div>
    <div class="dt-row">
      <span class="dt-lbl">Posición</span>
      <div class="dt-ctl">
        <div class="dt-inline">
          ${spinBox('x', { raw: toRaw(effValue(clip, 'x', clip.x || 0), 1), min: -100, max: 100, step: 1, unit: '%', prefix: 'X' })}
          ${spinBox('y', { raw: toRaw(effValue(clip, 'y', clip.y || 0), 1), min: -100, max: 100, step: 1, unit: '%', prefix: 'Y' })}
        </div>
        ${kfControls(clip, 'x,y')}
      </div>
    </div>
    <div class="dt-row">
      <span class="dt-lbl">Rotación del plano</span>
      <div class="dt-ctl">
        ${spinBox('rotate', { raw: toRaw(rotate, 1), min: -360, max: 360, step: 0.1, unit: '°' })}
        <button class="dial-btn" type="button" data-rotate-dial title="Arrastra para girar el plano"><i style="--dial:${dialRot}deg"></i></button>
        ${kfControls(clip, 'rotate')}
      </div>
    </div>
    ${alignBar()}
    ${flipRow}`;
}

/* Secciones Transformación + Mezcla para los clips que no son texto. */
function transformSection(clip) {
  return `
    <details class="tx-group" open data-section="transform">
      <summary>Transformación<button class="sec-reset" type="button" data-transform-reset title="Restablecer transformación">${icon('rotate', 13)}</button></summary>
      ${transformInner(clip)}
    </details>
    <details class="tx-group" open data-section="mix">
      <summary>Mezcla<button class="sec-reset" type="button" data-mix-reset title="Restablecer mezcla">${icon('rotate', 13)}</button></summary>
      ${dtSliderRow('Opacidad', 'opacity', clip.opacity ?? 1, { min: 0, max: 100, factor: 0.01, unit: '%', clip })}
    </details>`;
}

function colorValue(color) {
  return /^#[0-9a-fA-F]{6}$/.test(color || '') ? color : '#ffffff';
}

function rgbaColor(color, alpha = 1) {
  const match = String(color || '').match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i);
  if (!match) return color || 'transparent';
  return `rgba(${parseInt(match[1], 16)}, ${parseInt(match[2], 16)}, ${parseInt(match[3], 16)}, ${Math.max(0, Math.min(1, alpha))})`;
}

function textRangeField(label, key, value, min, max, step, factor, target = 'text', unit = '') {
  const numeric = Number(value);
  const safe = Number.isFinite(numeric) ? numeric : 0;
  const inputValue = Math.round((safe / factor) * 1000) / 1000;
  const display = factor === 0.01 ? `${Math.round(safe * 100)}${unit || '%'}` : `${Math.round(safe * 100) / 100}${unit}`;
  return `
    <div class="prop tx-prop">
      <span class="tx-lbl">${label}</span>
      <input type="range" data-text-range="${key}" data-text-factor="${factor}" data-text-target="${target}" data-text-unit="${unit}" min="${min}" max="${max}" step="${step}" value="${inputValue}" />
      <b class="tx-val" data-tx-readout="${key}">${display}</b>
    </div>`;
}

function textColorField(label, key, color) {
  return `
    <div class="tx-color-field">
      <span>${label}</span>
      <input class="input" type="color" data-text-paint="${key}" value="${colorValue(color)}" />
    </div>`;
}

export function mountDetailsPanel(host) {
  host.classList.add('panel', 'details-panel');
  host.innerHTML = `
    <div class="details-head">
      <span class="details-title">Detalles</span>
    </div>
    <div class="details-scroll" data-details></div>
    <div class="details-foot" data-details-foot></div>`;
  const details = host.querySelector('[data-details]');
  const foot = host.querySelector('[data-details-foot]');
  const subscriptions = [];
  let suppressRender = false;

  const ratioLabel = (ratio) => (ratio === '16:9' ? 'Original' : ratio);

  function projectMarkup() {
    const project = state.project;
    return `
      <div class="detail-list project-detail-list">
        <div class="detail-item"><span class="d-label">Nombre:</span><span class="d-value strong" title="${escapeHtml(project.name)}">${escapeHtml(project.name)}</span></div>
        <div class="detail-item"><span class="d-label">Ruta:</span><span class="d-value path" title="openCUP / Proyectos / ${escapeHtml(project.name)}">openCUP / Proyectos / ${escapeHtml(project.name)}</span></div>
        <div class="detail-item"><span class="d-label">Archivos multimedia importados:</span><span class="d-value">Mantener en la ubicación original</span></div>
        <div class="detail-item"><span class="d-label">Disposición de capas: <i class="d-info" title="Las capas se apilan en el orden de las pistas">${icon('info', 11)}</i></span><span class="d-value">Activado</span></div>
        <div class="detail-item"><span class="d-label">Proxy: <i class="d-info" title="Los archivos proxy agilizan la reproducción de videos pesados">${icon('info', 11)}</i></span><span class="d-value">Desactivado</span></div>
      </div>
      <div class="details-sep"></div>
      <div class="detail-list">
        <div class="detail-item"><span class="d-label">Relación de aspecto:</span><span class="d-value">${ratioLabel(project.ratio)}</span></div>
        <div class="detail-item"><span class="d-label">Resolución:</span><span class="d-value">${escapeHtml(project.resolution || 'Adaptado')}</span></div>
        <div class="detail-item"><span class="d-label">Cuadros por segundo:</span><span class="d-value">${(project.fps || 30).toFixed(2)} fps</span></div>
      </div>`;
  }

  function openProjectEditor() {
    if (!state.project) return;
    const project = state.project;
    openModal({
      title: 'Modificar proyecto',
      width: 470,
      body: `
        <div class="field"><label>Nombre</label><input class="input" data-edit-name value="${escapeHtml(project.name)}" maxlength="80" /></div>
        <div class="field-row">
          <div class="field"><label>Relación de aspecto</label>
            <select class="input" data-edit-ratio>
              ${['16:9', '9:16', '1:1', '4:3', '21:9', '4:5'].map((ratio) => `<option ${ratio === project.ratio ? 'selected' : ''}>${ratio}</option>`).join('')}
            </select>
          </div>
          <div class="field"><label>Resolución</label>
            <select class="input" data-edit-resolution>
              ${['480p', '720p', '1080p', '4K'].map((key) => `<option ${key === project.resolution ? 'selected' : ''}>${key}</option>`).join('')}
            </select>
          </div>
        </div>
        <div class="field"><label>Cuadros por segundo</label>
          <select class="input" data-edit-fps>
            ${[24, 30, 60].map((fps) => `<option value="${fps}" ${fps === project.fps ? 'selected' : ''}>${fps} fps</option>`).join('')}
          </select>
        </div>`,
      footer: `<button class="btn" data-edit-cancel>Cancelar</button><button class="btn btn-primary" data-edit-save>Guardar</button>`,
      onMount(modal) {
        modal.foot.querySelector('[data-edit-cancel]').addEventListener('click', modal.close);
        modal.foot.querySelector('[data-edit-save]').addEventListener('click', () => {
          const name = modal.body.querySelector('[data-edit-name]').value.trim();
          const ratio = modal.body.querySelector('[data-edit-ratio]').value;
          const resolution = modal.body.querySelector('[data-edit-resolution]').value;
          const fps = Number(modal.body.querySelector('[data-edit-fps]').value);
          updateProject({
            ...(name ? { name } : {}),
            ratio,
            resolution,
            fps,
          });
          modal.close();
        });
      },
    });
  }

  function clipMarkup(found) {
    const { clip } = found;
    const media = clip.mediaId ? state.media.get(clip.mediaId) : null;
    const isText = clip.kind === 'text';
    const isSticker = clip.kind === 'sticker';
    const isVisual = clip.kind === 'video' || clip.kind === 'image';
    const isAudio = clip.kind === 'audio';
    const kindLabel = { video: 'Video', image: 'Imagen', audio: 'Audio', text: 'Texto', sticker: 'Sticker' }[clip.kind] || 'Clip';
    const thumb = media?.thumb
      ? `<div class="details-thumb"><img src="${media.thumb}" alt="" /></div>`
      : isText
        ? `<div class="details-thumb text-thumb">${escapeHtml((clip.text?.content || '').slice(0, 90))}</div>`
        : isSticker
          ? `<div class="details-thumb text-thumb sticker-thumb">${escapeHtml(clip.sticker?.content || clip.name || '✨')}</div>`
          : `<div class="details-thumb icon-thumb">${icon(isAudio ? 'music' : 'film', 26)}</div>`;

    return `
      <div class="clip-details-head"><i class="details-dot"></i><span>${escapeHtml(clip.name || kindLabel)}</span><span class="clip-chip">${kindLabel}</span></div>
      ${thumb}
      <div class="detail-list compact">
        <div class="detail-item"><span class="d-label">Inicio:</span><span class="d-value">${formatTime(clip.start)}</span></div>
        <div class="detail-item"><span class="d-label">Duración:</span><span class="d-value">${formatTime(clip.duration)}</span></div>
        ${media ? `<div class="detail-item"><span class="d-label">Archivo:</span><span class="d-value path" title="${escapeHtml(media.name)}">${escapeHtml(media.name)}</span></div>` : ''}
      </div>
      <div class="details-sep"></div>
      ${isText ? textControls(clip) : ''}
      ${isSticker ? stickerControls(clip) : ''}
      ${isVisual ? `
        <div class="details-section">Color</div>
        <div class="field">
          <label>Filtro</label>
          <select class="input" data-clip-filter>
            ${Object.entries(FILTERS).map(([key, value]) => `<option value="${key}" ${key === clip.filter ? 'selected' : ''}>${value.label}</option>`).join('')}
          </select>
        </div>
        <div class="field">
          <label>Efecto</label>
          <select class="input" data-clip-effect>
            ${Object.entries(EFFECTS).map(([key, value]) => `<option value="${key}" ${key === (clip.effect || 'none') ? 'selected' : ''}>${value.label}</option>`).join('')}
          </select>
        </div>
        ${detailRange('Brillo', 'adjust.brightness', Math.round(clip.adjust?.brightness ?? 100), 0, 200, 1, 1, Math.round(clip.adjust?.brightness ?? 100), '%')}
        ${detailRange('Contraste', 'adjust.contrast', Math.round(clip.adjust?.contrast ?? 100), 0, 200, 1, 1, Math.round(clip.adjust?.contrast ?? 100), '%')}
        ${detailRange('Saturación', 'adjust.saturate', Math.round(clip.adjust?.saturate ?? 100), 0, 300, 1, 1, Math.round(clip.adjust?.saturate ?? 100), '%')}
        ${detailRange('Grises', 'adjust.grayscale', Math.round(clip.adjust?.grayscale ?? 0), 0, 100, 1, 1, Math.round(clip.adjust?.grayscale ?? 0), '%')}
        ${detailRange('Sepia', 'adjust.sepia', Math.round(clip.adjust?.sepia ?? 0), 0, 100, 1, 1, Math.round(clip.adjust?.sepia ?? 0), '%')}
        ${detailRange('Tono', 'adjust.hue', Math.round(clip.adjust?.hue ?? 0), -180, 180, 1, 1, Math.round(clip.adjust?.hue ?? 0), '°')}
      ` : ''}
      ${!isText ? transformSection(clip) : ''}
      ${clip.kind === 'video' || isAudio ? `
        <div class="details-section">Audio</div>
        ${detailRange('Volumen', 'volume', Math.round((clip.volume ?? 1) * 100), 0, 100, 1, 0.01, Math.round((clip.volume ?? 1) * 100), '%')}
        ${detailRange('Velocidad', 'speed', Math.round((clip.speed ?? 1) * 100), 25, 300, 5, 0.01, (clip.speed ?? 1).toFixed(2), '×')}
      ` : ''}
      <div class="details-section">Entrada y transición</div>
      <div class="field">
        <label>Transición al inicio</label>
        <select class="input" data-clip-transition>
          ${TRANSITIONS.map((item) => `<option value="${item.id}" ${item.id === (clip.transition || 'none') ? 'selected' : ''}>${item.label}</option>`).join('')}
        </select>
      </div>
      ${(clip.transition && clip.transition !== 'none') ? numberField('Duración de transición (s)', 'transitionDuration', clip.transitionDuration || 0.6, 0.1, 3, 0.1) : ''}
      <div class="details-section">Fundidos</div>
      <div class="prop">
        <div class="prop-grid">
          ${numberField('Entrada (s)', 'fadeIn', clip.fadeIn || 0, 0, 10, 0.1)}
          ${numberField('Salida (s)', 'fadeOut', clip.fadeOut || 0, 0, 10, 0.1)}
        </div>
      </div>
      <button class="btn btn-ghost btn-block" data-reset-clip>${icon('rotate', 14)} Restablecer ajustes</button>`;
  }

  function textControls(clip) {
    const text = { ...TEXT_DEFAULTS, ...(clip.text || {}) };
    const hasBackground = text.background && text.background !== 'transparent';
    const styleSwatches = `
      <button class="tx-style-swatch tx-style-none" data-text-style-clear title="Sin estilo (restablecer color, trazo y fondo)">⊘</button>
      ${TEXT_STYLES.map((style) => {
        const patch = style.patch || {};
        const background = patch.background && patch.background !== 'transparent'
          ? `background:${patch.background};border-radius:4px;`
          : '';
        const font = patch.font ? `font-family:'${patch.font}',sans-serif;` : '';
        const glow = patch.glow ? `text-shadow:0 0 6px ${patch.glowColor || '#00e5d0'};` : '';
        return `<button class="tx-style-swatch" data-text-style-apply="${style.id}" title="${escapeHtml(style.label)}" style="color:${patch.color || '#ffffff'};font-weight:${patch.weight || 700};${font}${background}${glow}">Aa</button>`;
      }).join('')}`;
    const animationOptions = TEXT_ANIMATION_GROUPS.map((group) => `
      <optgroup label="${group.label}">
        ${group.items.map((item) => `<option value="${item.id}" ${text.animation === item.id ? 'selected' : ''}>${escapeHtml(group.label)} · ${escapeHtml(item.label)}</option>`).join('')}
      </optgroup>`).join('');
    return `
      <div class="details-section">Texto</div>
      <div class="field">
        <label>Contenido</label>
        <textarea class="input" data-text-content rows="3" maxlength="400">${escapeHtml(text.content || '')}</textarea>
      </div>

      <details class="tx-group" open>
        <summary>Fuente</summary>
        <div class="field">
          <label>Fuente</label>
          <select class="input" data-text-select="font">
            ${getFontList().map((font) => `<option value="${escapeHtml(font.value)}" ${text.font === font.value ? 'selected' : ''}>${escapeHtml(font.label)}</option>`).join('')}
          </select>
        </div>
        <button class="btn btn-small btn-ghost" data-load-system-fonts style="margin-top:6px">${icon('refresh', 13)} Detectar fuentes del sistema</button>
        <div class="modal-note" style="margin:4px 0 0;font-size:11px">Detecta las fuentes instaladas en tu equipo (requiere permiso del navegador). Solo Chromium.</div>
        <div class="field-row">
          <div class="field">
            <label>Tamaño de la fuente</label>
            <input class="input" type="number" data-text-number="size" min="8" max="400" step="1" value="${Math.round(text.size || 72)}" />
          </div>
          <div class="field">
            <label>Grosor</label>
            <select class="input" data-text-select="weight">
              ${TEXT_WEIGHTS.map((weight) => `<option value="${weight.value}" ${Number(text.weight) === weight.value ? 'selected' : ''}>${weight.label}</option>`).join('')}
            </select>
          </div>
        </div>
      </details>

      <details class="tx-group" open>
        <summary>Diseño</summary>
        <div class="tx-chip-row">
          <button class="tx-chip tx-solid${text.bold ? ' active' : ''}" data-text-toggle="bold" title="Negrita">B</button>
          <button class="tx-chip tx-italic${text.italic ? ' active' : ''}" data-text-toggle="italic" title="Cursiva">I</button>
          <button class="tx-chip tx-underline${text.underline ? ' active' : ''}" data-text-toggle="underline" title="Subrayado">U</button>
        </div>
      </details>

      <details class="tx-group" open>
        <summary>May./mín.</summary>
        <div class="tx-chip-row">
          ${TEXT_CASES.map((item) => `<button class="tx-chip${text.textCase === item.value ? ' active' : ''}" data-text-case="${item.value}" title="${item.title}">${item.label}</button>`).join('')}
        </div>
      </details>

      <details class="tx-group" open>
        <summary>Color</summary>
        <div class="tx-color-inline">
          <input class="input" type="color" data-text-color value="${colorValue(text.color)}" />
          <span>Color del texto</span>
          <b>${escapeHtml(colorValue(text.color))}</b>
        </div>
      </details>

      <details class="tx-group" open>
        <summary>Carácter y línea</summary>
        ${textRangeField('Carácter', 'letterSpacing', text.letterSpacing, -20, 60, 1, 0.01)}
        ${textRangeField('Línea', 'lineSpacing', text.lineSpacing, 70, 220, 1, 0.01)}
      </details>

      <details class="tx-group" open>
        <summary>Alineación</summary>
        <div class="tx-chip-row">
          ${TEXT_ALIGNS.map((item) => `<button class="tx-chip${text.align === item.value ? ' active' : ''}" data-text-align="${item.value}" title="${item.label}">${item.label}</button>`).join('')}
        </div>
      </details>

      <details class="tx-group" open>
        <summary>Estilo preestablecido</summary>
        <div class="tx-style-grid">${styleSwatches}</div>
      </details>

      <details class="tx-group" open data-section="transform">
        <summary>Transformación<button class="sec-reset" type="button" data-transform-reset title="Restablecer transformación">${icon('rotate', 13)}</button></summary>
        ${transformInner(clip, { text: true })}
      </details>

      <details class="tx-group" open data-section="mix">
        <summary>Mezcla<button class="sec-reset" type="button" data-mix-reset title="Restablecer mezcla">${icon('rotate', 13)}</button></summary>
        ${dtSliderRow('Opacidad', 'text.opacity', text.opacity, { min: 0, max: 100, factor: 0.01, unit: '%', clip })}
      </details>

      <details class="tx-group">
        <summary>Trazo</summary>
        ${textColorField('Color', 'strokeColor', text.strokeColor)}
        ${textRangeField('Grosor', 'strokeWidth', text.strokeWidth, 0, 25, 1, 0.01)}
        <div class="field">
          <label>Posición</label>
          <select class="input" data-text-select="strokeMode">
            <option value="outer" ${text.strokeMode === 'outer' ? 'selected' : ''}>Fuera</option>
            <option value="center" ${text.strokeMode === 'center' ? 'selected' : ''}>Centro</option>
            <option value="inside" ${text.strokeMode === 'inside' ? 'selected' : ''}>Dentro</option>
          </select>
        </div>
      </details>

      <details class="tx-group">
        <summary>Fondo</summary>
        <div class="inline-field">
          <input class="input" type="color" data-text-paint="background" value="${hasBackground ? colorValue(text.background) : '#000000'}" />
          <button class="btn btn-small" data-text-bg-clear>${hasBackground ? 'Quitar fondo' : 'Transparente'}</button>
        </div>
        ${textRangeField('Opacidad', 'backgroundAlpha', text.backgroundAlpha, 0, 100, 1, 0.01)}
        ${textRangeField('Radio', 'backgroundRadius', text.backgroundRadius, 0, 50, 1, 0.01)}
        ${textRangeField('Margen', 'backgroundPadding', text.backgroundPadding, 0, 60, 1, 0.01)}
        <div class="field">
          <label>Estilo</label>
          <select class="input" data-text-select="backgroundStyle">
            <option value="rounded" ${text.backgroundStyle === 'rounded' ? 'selected' : ''}>Redondeado</option>
            <option value="square" ${text.backgroundStyle === 'square' ? 'selected' : ''}>Recto</option>
            <option value="pill" ${text.backgroundStyle === 'pill' ? 'selected' : ''}>Píldora</option>
          </select>
        </div>
        ${textColorField('Borde', 'backgroundBorderColor', text.backgroundBorderColor)}
        ${textRangeField('Grosor del borde', 'backgroundBorderWidth', text.backgroundBorderWidth, 0, 5, 1, 0.01)}
      </details>

      <details class="tx-group">
        <summary>Brillo</summary>
        <div class="tx-switch-row"><span>Resplandor</span><button class="switch ${text.glow ? 'on' : ''}" data-text-toggle="glow" aria-label="Resplandor de texto"></button></div>
        ${textColorField('Color', 'glowColor', text.glowColor)}
        ${textRangeField('Intensidad', 'glowBlur', text.glowBlur, 0, 120, 1, 0.01)}
      </details>

      <details class="tx-group">
        <summary>Sombra</summary>
        <div class="tx-switch-row"><span>Sombra</span><button class="switch ${text.shadow !== false ? 'on' : ''}" data-text-toggle="shadow" aria-label="Sombra de texto"></button></div>
        ${textColorField('Color', 'shadowColor', text.shadowColor)}
        ${textRangeField('Distancia', 'shadowDistance', text.shadowDistance, 0, 30, 1, 0.01)}
        ${textRangeField('Ángulo', 'shadowAngle', text.shadowAngle, -180, 180, 1, 1, 'text', '°')}
        ${textRangeField('Desenfoque', 'shadowBlur', text.shadowBlur, 0, 80, 1, 0.01)}
        ${textRangeField('Opacidad', 'shadowAlpha', text.shadowAlpha, 0, 100, 1, 0.01)}
      </details>

      <details class="tx-group">
        <summary>Curva</summary>
        ${textRangeField('Curva', 'curve', text.curve, -100, 100, 1, 1)}
        <div class="tx-note">Valores positivos curvan el texto hacia arriba; negativos, hacia abajo.</div>
      </details>

      <details class="tx-group" open>
        <summary>Animación</summary>
        <div class="field">
          <label>Animación del texto</label>
          <select class="input" data-text-animation-select>
            <option value="none" ${text.animation === 'none' ? 'selected' : ''}>Sin animación</option>
            ${animationOptions}
          </select>
        </div>
        ${textRangeField('Duración', 'animationDuration', text.animationDuration, 0.1, 3, 0.05, 1, 'text', 's')}
      </details>

      <button class="btn btn-ghost btn-block" data-text-reset>${icon('rotate', 14)} Restablecer estilo de texto</button>
      <div class="details-sep"></div>`;
  }

  function stickerControls(clip) {
    const sticker = clip.sticker || {};
    const hasBackground = sticker.background && sticker.background !== 'transparent';
    return `
      <div class="details-section">Sticker</div>
      <div class="field">
        <label>Contenido</label>
        <input class="input" data-sticker-content maxlength="16" value="${escapeHtml(sticker.content || '')}" />
      </div>
      <div class="field-row">
        <div class="field">
          <label>Tamaño base</label>
          <input class="input" type="number" data-sticker-number="size" min="24" max="700" step="1" value="${Math.round(sticker.size || 220)}" />
        </div>
        <div class="field">
          <label>Forma del fondo</label>
          <select class="input" data-sticker-select="shape">
            <option value="none" ${(sticker.shape || 'none') === 'none' ? 'selected' : ''}>Sin fondo</option>
            <option value="circle" ${sticker.shape === 'circle' ? 'selected' : ''}>Círculo</option>
            <option value="square" ${sticker.shape === 'square' ? 'selected' : ''}>Cuadrado redondeado</option>
          </select>
        </div>
      </div>
      <div class="field">
        <label>Color de fondo</label>
        <div class="inline-field">
          <input class="input" type="color" data-sticker-color value="${hasBackground ? colorValue(sticker.background) : '#1d6d78'}" />
          <button class="btn btn-small" data-sticker-bg-clear>${hasBackground ? 'Quitar fondo' : 'Sin fondo'}</button>
        </div>
      </div>
      <div class="details-sep"></div>`;
  }

  function renderFoot() {
    if (state.selection) {
      foot.innerHTML = `
        <div class="details-foot-row">
          <button class="btn btn-ghost" data-clip-duplicate>${icon('copy', 14)} Duplicar</button>
          <button class="btn btn-ghost danger" data-clip-delete>${icon('trash', 14)} Eliminar</button>
        </div>`;
      return;
    }
    foot.innerHTML = `<button class="modify-btn" data-edit-project>Modificar</button>`;
  }

  /* Estado abierto/cerrado de las secciones con ancla (Transformación, Mezcla…):
     se conserva aunque el panel se vuelva a pintar por un cambio de selección. */
  const sectionState = {};

  function render() {
    if (suppressRender) return;
    const scrollTop = details.scrollTop;
    for (const el of details.querySelectorAll('details[data-section]')) {
      sectionState[el.dataset.section] = el.open;
    }
    const found = selectedClip();
    details.innerHTML = found ? clipMarkup(found) : projectMarkup();
    for (const el of details.querySelectorAll('details[data-section]')) {
      if (sectionState[el.dataset.section] !== undefined) el.open = sectionState[el.dataset.section];
    }
    renderFoot();
    details.scrollTop = scrollTop;
  }

  function applyRange(input) {
    const found = selectedClip();
    if (!found) return;
    const clip = found.clip;
    const key = input.dataset.prop;
    const factor = Number(input.dataset.factor || 1);
    const value = Number(input.value) * factor;
    suppressRender = true;
    if (key.startsWith('adjust.')) {
      updateClip(clip.id, { adjust: { [key.slice(7)]: value } }, { history: false });
    } else {
      // Respetando fotogramas clave: si la prop ya tiene keyframes se
      // inserta/actualiza uno en el cabezal (igual que CapCut).
      updateClip(clip.id, writeClipProp(clip, key, value), { history: false });
    }
    suppressRender = false;
    const unit = input.dataset.unit || (key === 'rotate' ? '°' : key === 'speed' ? '×' : '%');
    const display = key === 'speed'
      ? Number(value).toFixed(2)
      : factor === 1
        ? Math.round(value)
        : Math.round(value * 100);
    const readout = details.querySelector(`[data-readout="${key}"]`);
    if (readout) readout.textContent = `${display}${unit}`;
    syncPeerControls(key, input, toRaw(value, factor));
  }

  /* Mantiene el slider (.dt-range) y el cajón numérico ([data-spin]) sincronizados. */
  function syncPeerControls(prop, origin, raw) {
    if (origin.matches('.dt-range')) {
      const spin = details.querySelector(`[data-spin="${prop}"]`);
      if (spin) spin.value = raw;
      return;
    }
    const range = details.querySelector(`.dt-range[data-prop="${prop}"]`);
    if (range) range.value = raw;
  }

  /* Refresca los controles de transformación sin re-renderizar el panel:
     lo usamos en cada tick para que el cabezal actualice valores efectivos. */
  function syncTransformControls() {
    if (!details.querySelector('.dt-slider')) return;
    const found = selectedClip();
    if (!found) return;
    const clip = found.clip;
    for (const input of details.querySelectorAll('.dt-range[data-prop], [data-spin]')) {
      const prop = input.dataset.prop || input.dataset.spin;
      const factor = Number(input.dataset.factor || input.dataset.spinFactor || 1);
      const raw = toRaw(effValue(clip, prop, baseValueFor(clip, prop)), factor);
      if (Number(input.value) !== raw) input.value = raw;
    }
    const dialNeedle = details.querySelector('.dial-btn i');
    if (dialNeedle) {
      const rot = effValue(clip, 'rotate', clip.rotate || 0);
      dialNeedle.style.setProperty('--dial', `${Math.round((((rot % 360) + 360) % 360) * 10) / 10}deg`);
    }
    for (const button of details.querySelectorAll('[data-kf-toggle]')) {
      const props = button.dataset.kfToggle.split(',');
      const active = props.every((prop) => keyframeIndexAt(clip, prop, state.playhead) >= 0);
      button.classList.toggle('on', active);
      button.setAttribute('aria-pressed', String(active));
      button.title = active ? 'Quitar fotograma clave' : 'Agregar fotograma clave';
    }
    for (const button of details.querySelectorAll('[data-kf-nav]')) {
      button.disabled = button.dataset.kfProp.split(',').every((prop) => keyframeCount(clip, prop) === 0);
    }
  }

  host.addEventListener('click', (event) => {
    if (event.target.closest('[data-edit-project]')) { openProjectEditor(); return; }
    if (event.target.closest('[data-clip-delete]')) { deleteSelected(); return; }
    if (event.target.closest('[data-clip-duplicate]')) { duplicateSelected(); return; }
    if (event.target.closest('[data-export-video]')) { openExportDialog(); return; }
    if (event.target.closest('[data-details-help]')) { openShortcutsDialog(); return; }
    const uniformButton = event.target.closest('[data-uniform]');
    if (uniformButton) {
      const found = selectedClip();
      if (!found) return;
      const clip = found.clip;
      const next = !(clip.uniform !== false);
      const tr = transformAt(clip, state.playhead);
      const patch = { uniform: next };
      if (!next) {
        patch.scaleX = clip.scaleX ?? tr.scaleX;
        patch.scaleY = clip.scaleY ?? tr.scaleY;
      }
      updateClip(clip.id, patch);
      return;
    }
    const transformReset = event.target.closest('[data-transform-reset]');
    if (transformReset) {
      // Está dentro de un <summary>: evitamos que el clic cierre la sección.
      event.preventDefault();
      event.stopPropagation();
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, {
        x: 0,
        y: 0,
        scale: 1,
        scaleX: undefined,
        scaleY: undefined,
        rotate: 0,
        uniform: true,
        keyframes: Object.fromEntries(Object.entries(found.clip.keyframes || {}).filter(([prop]) => prop !== 'x' && prop !== 'y' && prop !== 'scale' && prop !== 'scaleX' && prop !== 'scaleY' && prop !== 'rotate')),
      });
      toast('Transformación restablecida');
      return;
    }
    const mixReset = event.target.closest('[data-mix-reset]');
    if (mixReset) {
      event.preventDefault();
      event.stopPropagation();
      const found = selectedClip();
      if (!found) return;
      const isText = found.clip.kind === 'text';
      const key = isText ? 'text.opacity' : 'opacity';
      updateClip(found.clip.id, isText
        ? { text: { opacity: 1 }, keyframes: withoutKeyframe(found.clip, key, state.playhead) }
        : { opacity: 1, keyframes: withoutKeyframe(found.clip, key, state.playhead) });
      toast('Mezcla restablecida');
      return;
    }
    const alignButton = event.target.closest('[data-align]');
    if (alignButton) {
      const found = selectedClip();
      if (!found) return;
      const patch = alignPatch(found.clip, alignButton.dataset.align);
      updateClip(found.clip.id, patch);
      return;
    }
    const kfToggle = event.target.closest('[data-kf-toggle]');
    if (kfToggle) {
      const found = selectedClip();
      if (!found) return;
      const clip = found.clip;
      const props = kfToggle.dataset.kfToggle.split(',');
      const removing = props.every((prop) => keyframeIndexAt(clip, prop, state.playhead) >= 0);
      let store = clip.keyframes || {};
      for (const prop of props) {
        store = removing
          ? withoutKeyframe({ keyframes: store }, prop, state.playhead)
          : nextKeyframeStore({ keyframes: store }, prop, state.playhead, effValue(clip, prop, baseValueFor(clip, prop)));
      }
      updateClip(clip.id, { keyframes: store });
      return;
    }
    const kfNav = event.target.closest('[data-kf-nav]');
    if (kfNav) {
      const found = selectedClip();
      if (!found) return;
      const clip = found.clip;
      const props = kfNav.dataset.kfProp.split(',');
      const direction = Number(kfNav.dataset.kfNav) || 1;
      const targets = props
        .map((prop) => neighborKeyframe(clip, prop, state.playhead, direction))
        .filter(Boolean);
      if (!targets.length) return;
      const time = direction > 0
        ? Math.min(...targets.map((point) => point.t))
        : Math.max(...targets.map((point) => point.t));
      seek(time);
      return;
    }
    if (event.target.closest('[data-rotate-dial]')) {
      const found = selectedClip();
      if (!found) return;
      const dial = event.target.closest('[data-rotate-dial]');
      const rect = dial.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const angle = Math.round((Math.atan2(event.clientY - cy, event.clientX - cx) * 180) / Math.PI + 90);
      const value = ((angle % 360) + 360) % 360;
      const rotated = value > 180 ? value - 360 : value;
      updateClip(found.clip.id, writeClipProp(found.clip, 'rotate', rotated));
      return;
    }
    const toggle = event.target.closest('[data-text-toggle]');
    if (toggle) {
      const found = selectedClip();
      if (!found) return;
      const field = toggle.dataset.textToggle;
      const current = field === 'shadow' ? found.clip.text?.shadow !== false : !!found.clip.text?.[field];
      updateClip(found.clip.id, { text: { [field]: !current } });
      return;
    }
    const textAlignButton = event.target.closest('[data-text-align]');
    if (textAlignButton) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { text: { align: textAlignButton.dataset.textAlign } });
      return;
    }
    const caseButton = event.target.closest('[data-text-case]');
    if (caseButton) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { text: { textCase: caseButton.dataset.textCase } });
      return;
    }
    const styleApply = event.target.closest('[data-text-style-apply]');
    if (styleApply) {
      const found = selectedClip();
      const style = TEXT_STYLES.find((item) => item.id === styleApply.dataset.textStyleApply);
      if (!found || !style) return;
      updateClip(found.clip.id, { text: style.patch });
      toast(`Estilo “${style.label}” aplicado`, 'success', 1500);
      return;
    }
    if (event.target.closest('[data-text-style-clear]')) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, {
        text: {
          ...TEXT_DEFAULTS,
          content: found.clip.text?.content || '',
        },
      });
      toast('Estilo de texto restablecido', 'info', 1500);
      return;
    }
    if (event.target.closest('[data-text-reset]')) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, {
        text: {
          ...TEXT_DEFAULTS,
          content: found.clip.text?.content || '',
        },
      });
      toast('Estilo de texto restablecido', 'info', 1600);
      return;
    }
    if (event.target.closest('[data-reset-clip]')) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, {
        filter: 'none',
        effect: 'none',
        transition: 'none',
        transitionDuration: 0.6,
        adjust: { brightness: 100, contrast: 100, saturate: 100 },
        opacity: 1,
        scale: 1,
        rotate: 0,
        x: 0,
        y: 0,
        volume: 1,
        speed: 1,
        fadeIn: 0,
        fadeOut: 0,
      });
      return;
    }
    if (event.target.closest('[data-text-bg-clear]')) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { text: { background: 'transparent' } });
      return;
    }
    if (event.target.closest('[data-sticker-bg-clear]')) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { sticker: { background: 'transparent' } });
      return;
    }
  });

  details.addEventListener('input', (event) => {
    const target = event.target;
    if (target.matches('[data-text-content]')) {
      const found = selectedClip();
      if (!found) return;
      suppressRender = true;
      updateClip(found.clip.id, {
        name: target.value.trim().slice(0, 24) || 'Texto',
        text: { content: target.value },
      }, { history: false });
      suppressRender = false;
      return;
    }
    if (target.matches('[data-text-color]')) {
      const found = selectedClip();
      if (!found) return;
      suppressRender = true;
      updateClip(found.clip.id, { text: { color: target.value } }, { history: false });
      suppressRender = false;
      const readout = details.querySelector('.tx-color-inline b');
      if (readout) readout.textContent = target.value;
      return;
    }
    if (target.matches('[data-text-paint]')) {
      const found = selectedClip();
      if (!found) return;
      const key = target.dataset.textPaint;
      suppressRender = true;
      updateClip(found.clip.id, { text: { [key]: target.value } }, { history: false });
      suppressRender = false;
      return;
    }
    if (target.matches('[data-text-range]')) {
      const found = selectedClip();
      if (!found) return;
      const key = target.dataset.textRange;
      const factor = Number(target.dataset.textFactor || 1);
      const value = Number(target.value) * factor;
      suppressRender = true;
      if (target.dataset.textTarget === 'clip') {
        updateClip(found.clip.id, { [key]: value }, { history: false });
      } else {
        updateClip(found.clip.id, { text: { [key]: value } }, { history: false });
      }
      suppressRender = false;
      const unit = target.dataset.textUnit || '';
      const readout = details.querySelector(`[data-tx-readout="${key}"]`);
      if (readout) {
        readout.textContent = factor === 0.01
          ? `${Math.round(value * 100)}${unit || '%'}`
          : `${Math.round(value * 100) / 100}${unit}`;
      }
      return;
    }
    if (target.matches('[data-text-background]')) {
      const found = selectedClip();
      if (!found) return;
      suppressRender = true;
      updateClip(found.clip.id, { text: { background: target.value } }, { history: false });
      suppressRender = false;
      return;
    }
    if (target.matches('[data-sticker-content]')) {
      const found = selectedClip();
      if (!found) return;
      suppressRender = true;
      updateClip(found.clip.id, {
        name: target.value.trim() || 'Sticker',
        sticker: { content: target.value },
      }, { history: false });
      suppressRender = false;
      return;
    }
    if (target.matches('[data-sticker-color]')) {
      const found = selectedClip();
      if (!found) return;
      suppressRender = true;
      updateClip(found.clip.id, { sticker: { background: target.value, shape: 'circle' } }, { history: false });
      suppressRender = false;
      return;
    }
    if (target.matches('[data-prop]')) applyRange(target);
  });

  details.addEventListener('change', (event) => {
    const target = event.target;
    const found = selectedClip();
    if (!found) return;
    if (target.matches('[data-clip-filter]')) {
      updateClip(found.clip.id, { filter: target.value });
      return;
    }
    if (target.matches('[data-clip-effect]')) {
      updateClip(found.clip.id, { effect: target.value });
      return;
    }
    if (target.matches('[data-clip-transition]')) {
      const type = target.value;
      const fades = TRANSITION_FADES[type] || { fadeIn: 0.2, fadeOut: 0.2 };
      updateClip(found.clip.id, {
        transition: type,
        transitionDuration: found.clip.transitionDuration || 0.6,
        ...(type === 'none' ? { fadeIn: 0, fadeOut: 0 } : fades),
      });
      return;
    }
    if (target.matches('[data-sticker-number]')) {
      updateClip(found.clip.id, { sticker: { [target.dataset.stickerNumber]: Number(target.value) } });
      return;
    }
    if (target.matches('[data-sticker-select]')) {
      updateClip(found.clip.id, { sticker: { [target.dataset.stickerSelect]: target.value } });
      return;
    }
    if (target.matches('[data-text-number]')) {
      const key = target.dataset.textNumber;
      updateClip(found.clip.id, { text: { [key]: Number(target.value) } });
      return;
    }
    if (target.matches('[data-text-animation-select]')) {
      const value = target.value;
      const item = TEXT_ANIMATIONS.find((animation) => animation.id === value);
      updateClip(found.clip.id, {
        text: {
          animation: value,
          animationPhase: item?.phase || 'in',
          animationDuration: item?.duration || found.clip.text?.animationDuration || 0.6,
        },
      });
      return;
    }
    if (target.matches('[data-text-select]')) {
      const key = target.dataset.textSelect;
      updateClip(found.clip.id, { text: { [key]: target.value } });
      return;
    }
    if (target.matches('[data-prop-number]')) {
      const key = target.dataset.propNumber;
      const value = Number(target.value);
      updateClip(found.clip.id, { [key]: value });
      return;
    }
    if (target.matches('[data-spin]')) {
      const clip = found.clip;
      const key = target.dataset.spin;
      const factor = Number(target.dataset.spinFactor) || 1;
      const min = target.min !== '' ? Number(target.min) : -Infinity;
      const max = target.max !== '' ? Number(target.max) : Infinity;
      const clamped = Math.min(max, Math.max(min, Number(target.value)));
      const stored = Math.round(clamped * factor * 1000) / 1000;
      // La escala uniforme escribe la misma escala en ambos ejes.
      if (key === 'scale' && clip.uniform !== false) {
        updateClip(clip.id, writeClipProp(clip, 'scale', stored));
      } else if (key === 'opacity' && clip.kind === 'text') {
        updateClip(clip.id, writeClipProp(clip, 'text.opacity', stored));
      } else {
        updateClip(clip.id, writeClipProp(clip, key, stored));
      }
      syncPeerControls(key, target, stored);
    }
  });

  /* `toggle` no burbujea: lo capturamos en fase de captura desde el contenedor. */
  details.addEventListener('toggle', (event) => {
    const el = event.target;
    if (el instanceof HTMLDetailsElement && el.dataset.section) {
      sectionState[el.dataset.section] = el.open;
    }
  }, true);

  subscriptions.push(on('project', render));
  subscriptions.push(on('selection', render));
  // Los fotogramas clave hacen variar los valores con el cabezal: sincronizamos
  // sliders, cajones y el dial sin re-pintar el panel entero.
  subscriptions.push(on('tick', syncTransformControls));

  render();

  return {
    element: host,
    destroy() {
      subscriptions.forEach((off) => off());
    },
  };
}
