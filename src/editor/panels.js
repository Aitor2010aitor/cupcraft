import {
  state,
  on,
  setPanel,
  selectedClip,
  addTextClip,
  addSubtitleClips,
  clearSubtitleClips,
  addStickerClip,
  addMediaToTimeline,
  updateClip,
  updateClipsMany,
  duplicateSelected,
  deleteSelected,
  selectClip,
  updateProject,
  newProject,
  saveNow,
  importFiles,
  importExternalMedia,
  openProject,
  goHome,
  projectDuration,
  seek,
  FILTERS,
  FILTER_LINE_CATEGORIES,
  TEXT_DEFAULTS,
  RESOLUTIONS,
  ASPECTS,
} from '../store.js';
import {
  transformAt,
  hasKeyframes,
  keyframeCount,
  keyframeIndexAt,
  keyframeValue,
  nextKeyframeStore,
  withoutKeyframe,
  neighborKeyframe,
} from '../keyframes.js';
import { clipLayerBox } from '../renderer.js';
import { icon } from '../icons.js';
import { openModal, confirmDialog, promptDialog } from '../modal.js';
import { openExportDialog } from '../export.js';
import { openShortcutsDialog } from '../dialogs.js';
import { TEMPLATES } from '../templates.js';
import {
  generateTtsAbiertoAudio,
} from '../tts-abierto.js';
import {
  STICKERS,
  STICKER_CATEGORIES,
  EFFECTS,
  EFFECT_CATEGORIES,
  TRANSITIONS,
  TRANSITION_FADES,
  CAPTION_STYLES,
  captionStyleTextPatch,
  AVATAR_PRESETS,
  ADJUST_PRESETS,
  ADJUST_CATEGORIES,
} from '../library.js';
import {
  CAPCUT_EFFECT_ITEMS,
  CAPCUT_EFFECT_CATEGORIES,
  CAPCUT_TRANSITION_ITEMS,
  CAPCUT_TRANSITION_CATEGORIES as CAPCUT_TRANSITION_PANEL_CATEGORIES,
  CAPCUT_STICKER_ITEMS,
  CAPCUT_STICKER_CATEGORIES,
  CAPCUT_TEXT_ITEMS,
  CAPCUT_TEXT_CATEGORIES,
} from '../capcut-panels.js';
import { capcutEffectToGL } from '../gl-post.js';
import { prepareStickerSrc } from '../sticker-image.js';

const CAPCUT_TRANSITION_MAP = [
  [/golpe|slam|choque/i, 'slam-hit'],
  [/sacud|shake|punch|vibrac/i, 'shake-punch'],
  [/terremot|earthquake/i, 'earthquake'],
  [/fuego|fire|flama|llama|ardient/i, 'fire-burst'],
  [/rayo|relampag|lightning|trueno/i, 'lightning-flash'],
  [/humo|smoke|niebla/i, 'smoke-cloud'],
  [/glitch|rgb|cromat/i, 'glitch-rgb'],
  [/latigazo|whip/i, 'whip-left'],
  [/rebote|bounce|snap/i, 'zoom-snap'],
  [/coraz|heart/i, 'heart-iris'],
  [/estrella|star/i, 'star-iris'],
  [/destello|flare|optico/i, 'lens-flare'],
  [/estrob|strobe/i, 'strobe'],
  [/pixel|mosaico/i, 'pixelate'],
  [/remolino|vortice|swirl/i, 'swirl-warp'],
  [/caleido|kaleido/i, 'kaleido'],
  [/pliegue|pagina|page/i, 'page-curl'],
  [/voronoi|shatter|fractura/i, 'drift-voronoi'],
  [/plasma/i, 'drift-plasma'],
  [/liquido|liquid|smudge|derret/i, 'drift-liquid'],
  [/matrix/i, 'drift-matrix'],
  [/tinta|ink|bleed/i, 'drift-ink'],
  [/interferenc|analog/i, 'drift-interference'],
  [/panal|honeycomb|hexagon/i, 'drift-honeycomb'],
  [/burbuja|bubble/i, 'drift-bubbles'],
  [/vhs|scanline/i, 'drift-vhs'],
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

/* Portada de un elemento CapCut: usamos siempre una miniatura local propia.
   Evitamos pedir miles de portadas ausentes (404) ni URLs firmadas caducadas. */
const IMG_BLANK = 'data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==';
function capcutCard(item, attr) {
  return `<button class="effect-card" ${attr}="${item.id}" data-capcut-name="${escapeHtml(item.name)}" title="${escapeHtml(item.name)}">
              <img class="effect-thumb capcut" src="${item.img}" onerror="this.onerror=null;this.src='${IMG_BLANK}'" alt="${escapeHtml(item.name)}" draggable="false" loading="lazy" decoding="async" />
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
import { adjustCss } from '../renderer.js';
import { capcutTextStyle } from '../capcut-text-style.js';
import {
  TEXT_STYLES,
  TEXT_BUBBLES,
  TEXT_EFFECTS,
  TEXT_ANIMATION_GROUPS,
  TEXT_ANIMATIONS,
  TEXT_FONTS,
  getFontList,
  loadSystemFonts,
  ensureFontReady,
  importCustomFont,
  isFavoriteFont,
  toggleFavoriteFont,
  TEXT_CASES,
  TEXT_ALIGNS,
  TEXT_WEIGHTS,
} from '../text-catalog.js';
import { openCapCutImportDialog } from '../capcut.js';
import { autoEditPanelHtml, wireAutoEditPanel, isWhisperModelReady, ensureWhisperPipeline, generateSubtitlesForVideo } from '../auto-edit.js';
import { fixWebmDuration } from '../webm.js';
import { escapeHtml, formatTime, formatBytes, toast, clamp, uid } from '../utils.js';

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
let transitionSource = 'local';
let capcutTransitionCategory = 'Todas';
let capcutTransitionSearch = '';
let stickerSource = 'capcut';
let capcutStickerCategory = 'Todos';
let capcutStickerSearch = '';
let capcutTextCategory = 'Todos';
let capcutTextSearch = '';
let fontPickerOpen = false;
let fontSearchQuery = '';
let fontActiveTab = 'all';
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

/* Galería del panel Filtros: solo la línea propia de cupcraft (src/filter-line.js).
   No hay pestaña de origen ni catálogo externo: todo se resuelve localmente. */
function filterGallery(activeFilter, includeNone, category = 'Todos', hasTarget = false) {
  const categories = ['Todos', ...FILTER_LINE_CATEGORIES];
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
      <label class="search-box filter-search">${icon('search', 13)}<input data-filter-search placeholder="Buscar filtro" value="${escapeHtml(filterSearch)}" /><span class="filter-count">${entries.length}</span></label>
      <div class="filter-grid">
        ${entries.map(([key, value]) => `
          <button class="filter-card${activeFilter === key ? ' active' : ''}" data-filter="${key}" data-filter-label="${escapeHtml(value.label.toLowerCase())}"${value.gl ? ' data-filter-gl="1"' : ''} title="${escapeHtml(value.label)}${value.gl ? ' · shader GL' : ''}">
            <div class="filter-preview" style="filter:${value.css}"></div>
            <div class="filter-name"><span>${value.label}</span>${activeFilter === key ? icon('check', 11) : ''}</div>
          </button>`).join('')}
      </div>
      <div class="panel-empty" style="padding-top:2px">
        ${!hasTarget ? 'Selecciona un clip de video o imagen para aplicarle un filtro.'
          : activeFilter ? 'Filtro aplicado al clip seleccionado.'
            : 'Elige un filtro de la línea cupcraft para aplicarlo al clip seleccionado.'}
      </div>
    </div>`;
}

/* Clave de la línea que corresponde al clip: los filtros GL se guardan en
   clip.glFilter y los heredados de proyectos antiguos (LUT) no tienen
   equivalente propio, así que no se marcan ninguna tarjeta. */
function lineKeyForClip(clip) {
  if (!clip) return null;
  if (clip.glFilter) {
    const hit = Object.entries(FILTERS).find(([, value]) => value.gl === clip.glFilter);
    return hit ? hit[0] : null;
  }
  if (clip.lut) return null;
  return clip.filter && clip.filter !== 'none' ? clip.filter : null;
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
  let filterCategory = 'Todos';
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
  /* El panel Detalles → Texto a voz abre esta misma pestaña con el texto del clip. */
  window.addEventListener('opencup:open-tts', (event) => {
    textTab = 'tts';
    state.panel = 'text';
    render();
    const area = host.querySelector('[data-tts-text]');
    if (area && event.detail?.text) {
      area.value = event.detail.text;
      area.dispatchEvent(new Event('input', { bubbles: true }));
    }
  });

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
    const def = FILTERS[filter];
    if (!def) return;
    // Al cambiar de filtro se limpian los otros motores (GL y LUT) para que no
    // se quede encima el anterior: el clip termina con exactamente un filtro.
    if (def.gl) {
      updateClip(found.clip.id, {
        filter: 'none',
        lut: null,
        lutAmount: 1,
        glFilter: def.gl,
        glFilterName: def.label,
        glFilterAmount: 1,
      });
    } else {
      updateClip(found.clip.id, {
        filter,
        lut: null,
        lutAmount: 1,
        glFilter: null,
        glFilterName: null,
        glFilterAmount: 1,
      });
    }
    seek(state.playhead);
    window.dispatchEvent(new CustomEvent('opencup:glfx'));
    toast(`Filtro “${def.label}” aplicado`, 'success', 1800);
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
    const fades = TRANSITION_FADES[kind] || { fadeIn: 0, fadeOut: 0 };
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
      const added = addSubtitleClips(
        cues.slice(0, 200).map((cue) => ({ text: cue.content, start: cue.start, end: cue.end })),
        { captionStyleId: captionStyle, captionAnimation, replace: false },
      );
      toast(`${added.length} subtítulo${added.length === 1 ? '' : 's'} importado${added.length === 1 ? '' : 's'}`, 'success');
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

  /* ------------------------- TTS-Abierto (Piper local) ------------------------- */
  let ttsBusy = false;

  function setTtsProgress(label, value) {
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

  function ttsButtonState(busy) {
    const button = body.querySelector('[data-tts-generate]');
    if (!button) return;
    button.disabled = busy;
    button.innerHTML = busy
      ? `${icon('rotate', 13)} Generando con TTS Abierto…`
      : `${icon('sparkles', 13)} Generar voz con TTS Abierto`;
  }

  async function generateTts() {
    if (ttsBusy) return;
    const area = body.querySelector('[data-tts-text]');
    const text = (area?.value || activeTextClip()?.text?.content || '').trim();
    if (!text) {
      toast('Escribe el texto que quieres convertir en voz', 'warning');
      return;
    }
    const voice = body.querySelector('[data-tts-voice-model]')?.value || 'es_ES-davefx-medium';
    ttsBusy = true;
    ttsButtonState(true);
    try {
      const result = await generateTtsAbiertoAudio({
        text,
        voice,
        speed: ttsRate,
        onProgress: (label, value) => setTtsProgress(label, value),
      });
      setTtsProgress('Importando el audio al proyecto…', 100);
      const stamp = new Date().toISOString().slice(11, 19).replace(/:/g, '-');
      const file = new File([result.blob], `TTS ${voice} ${stamp}.wav`, { type: 'audio/wav' });
      await importFiles([file], { silent: true });
      toast(`Voz generada (${result.seconds.toFixed(1)} s) e importada a la línea de tiempo`, 'success', 4200);
    } catch (error) {
      console.error('TTS Abierto', error);
      const message = error?.message || 'no se pudo generar la voz';
      toast(`TTS Abierto: ${message}`, 'error', 5400);
    } finally {
      ttsBusy = false;
      setTtsProgress(null);
      ttsButtonState(false);
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
    const featured = CAPCUT_TEXT_ITEMS.filter((item) => (item.cats || []).includes('Básico') || (item.cats || []).includes('Tendencias')).slice(0, 12);
    return `
      <div class="text-panel-hint">${clip ? 'Aplica un efecto al texto seleccionado.' : 'Elige un efecto: se creará un texto nuevo con ese acabado.'}</div>
      <div class="text-panel-section-head"><span>Estilos de CapCut</span><b>${CAPCUT_TEXT_ITEMS.length}</b></div>
      <div class="effect-grid capcut-text-featured">
        ${featured.map((item) => capcutCard(item, 'data-capcut-text')).join('')}
      </div>
      <button class="btn btn-small btn-block" data-text-tab="capcut" style="margin:8px 12px 4px;width:calc(100% - 24px)">Ver los ${CAPCUT_TEXT_ITEMS.length} estilos de CapCut</button>
      <div class="text-panel-section-head"><span>Efectos básicos</span><b>${TEXT_EFFECTS.length}</b></div>
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
        <p>El seguimiento de CapCut analiza el video en la nube para pegar el texto a una persona u objeto. cupcraft es un editor local y no incluye ese análisis.</p>
      </div>
      <div class="text-info-card">
        <strong>Alternativa sin conexión</strong>
        <p>Puedes mover el texto manualmente en el preview y ajustar la posición con fotogramas clave en la línea de tiempo. El resultado se mantiene editable y se exporta igual.</p>
        <button class="btn btn-block" data-add-text>${icon('plus', 14)} Añadir texto y colocarlo a mano</button>
      </div>
      <div class="text-info-card muted-card">La detección automática de caras, objetos y puntos de seguimiento necesita un servicio externo. No se envía ningún archivo desde cupcraft.</div>`;
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
    const sample = clip?.text?.content || 'Hola, esta es una vista previa de la voz de cupcraft.';
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
        <div class="text-panel-section-head"><span>Voz neuronal TTS Abierto (Piper)</span><b>100% Local</b></div>
        <div class="field">
          <label>Voz neuronal</label>
          <select class="input" data-tts-voice-model>
            <option value="es_ES-davefx-medium" selected>Davefx · Español (masculina, alta calidad)</option>
            <option value="es_ES-sharvard-medium">Sharvard · Español (neutral, alta calidad)</option>
            <option value="es_ES-carlfm-x_low">Carlfm · Español (femenina, ultrarrápida)</option>
          </select>
        </div>
        <div class="tts-progress hidden" data-tts-progress>
          <div class="tts-progress-track"><div class="tts-progress-bar" data-tts-progress-bar></div></div>
          <span class="tts-progress-label" data-tts-progress-label>Preparando…</span>
        </div>
        <button class="btn btn-primary btn-block" data-tts-generate>${icon('sparkles', 13)} Generar voz con TTS Abierto</button>
        <div class="text-panel-hint" style="margin-top:6px">Motor Piper TTS con modelos ONNX en español (TTS-Abierto). 100% privado, local e instantáneo.</div>
      </div>
      <div class="text-info-card">${icon('mic', 14)} <strong>Voces locales</strong><p>Se usan las voces de Windows mediante <code>speechSynthesis</code>. Si no aparece ninguna voz en la lista, instala un paquete de voz en Configuración → Hora e idioma → Voz.</p></div>`;
  }

  function textPanel() {
    const tabs = [
      ['text', 'Texto'],
      ['animation', 'Animación'],
      ['tracking', 'Seguimiento'],
      ['tts', 'Texto a voz'],
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
          ${list.map((item) => capcutCard(item, 'data-capcut-effect')).join('')}
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
              <img class="effect-thumb" src="assets/previews/effects/${key}.jpg" onerror="this.onerror=null;this.src='${IMG_BLANK}'" alt="${escapeHtml(value.label)}" loading="lazy" decoding="async" />
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
                <span class="caption-style-preview" style="font-family:'${escapeHtml(style.font || 'Inter')}';font-weight:${style.weight};font-style:${style.italic ? 'italic' : 'normal'};color:${style.color};background:${style.background === 'transparent' ? 'linear-gradient(135deg,#171d20,#222b2e 52%,#191b23)' : style.background};border-radius:${Math.round((style.backgroundRadius ?? 0.16) * 18)}px;letter-spacing:${style.letterSpacing || 0}em;-webkit-text-stroke:${style.strokeWidth || 0}em ${style.strokeColor || '#000000'};text-shadow:${style.glow ? `0 0 ${Math.max(3, (style.glowBlur || 0.35) * 12)}px ${style.glowColor || '#00e5d0'}` : style.shadow ? '0 2px 5px #000' : 'none'}">${escapeHtml(style.sample || style.label)}</span>
                <span class="caption-style-label">${escapeHtml(style.label)}</span>
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
              captionAnimation,
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

  let captionAnimation = 'none';
  let captionAnimationGroup = 'in';
  const captionAnimationConfig = (id, text = {}) => {
    if (id === 'caption-karaoke' || id === 'karaoke') {
      return { captionAnimation: 'karaoke', animation: 'none', animationPhase: 'in', animationDuration: 0.6, highlight: text.highlight || '#ffeb3b' };
    }
    if (id === 'caption-pop' || id === 'pop') {
      return { captionAnimation: 'pop', animation: 'in-pop', animationPhase: 'in', animationDuration: 0.25, highlight: text.highlight || '' };
    }
    const spec = TEXT_ANIMATIONS.find((item) => item.id === id && item.phase !== 'caption');
    return {
      captionAnimation: 'none',
      animation: spec?.id || 'none',
      animationPhase: spec?.phase || 'in',
      animationDuration: spec?.duration || 0.6,
      highlight: text.highlight || '',
    };
  };

  function captionsPanel() {
    const textClips = state.project?.tracks
      .filter((track) => track.type === 'text')
      .flatMap((track) => track.clips.filter((clip) => clip.kind === 'text'))
      .sort((a, b) => a.start - b.start) || [];

    const animationGroups = [
      { id: 'in', label: 'Entrada', items: [{ id: 'none', label: 'Sin animación' }, ...TEXT_ANIMATIONS.filter((item) => item.phase === 'in')] },
      { id: 'loop', label: 'Bucle', items: [{ id: 'none', label: 'Sin animación' }, ...TEXT_ANIMATIONS.filter((item) => item.phase === 'loop')] },
      { id: 'out', label: 'Salida', items: [{ id: 'none', label: 'Sin animación' }, ...TEXT_ANIMATIONS.filter((item) => item.phase === 'out')] },
      { id: 'caption', label: 'Por palabra', items: [
        { id: 'caption-karaoke', label: 'Palabra amarilla' },
        { id: 'caption-pop', label: 'Salto' },
      ] },
    ];
    const activeAnimationGroup = animationGroups.find((group) => group.id === captionAnimationGroup) || animationGroups[0];

    return `
      <div class="details-section" style="margin:2px 12px 6px">Estilo de subtítulo</div>
      <div class="caption-styles">
        ${CAPTION_STYLES.map((style) => `
          <button class="caption-style${captionStyle === style.id ? ' active' : ''}" data-caption-style="${style.id}">
            <span class="caption-style-preview" style="font-family:'${escapeHtml(style.font || 'Inter')}';font-weight:${style.weight};font-style:${style.italic ? 'italic' : 'normal'};color:${style.color};background:${style.background === 'transparent' ? 'linear-gradient(135deg,#171d20,#222b2e 52%,#191b23)' : style.background};border-radius:${Math.round((style.backgroundRadius ?? 0.16) * 18)}px;letter-spacing:${style.letterSpacing || 0}em;-webkit-text-stroke:${style.strokeWidth || 0}em ${style.strokeColor || '#000000'};text-shadow:${style.glow ? `0 0 ${Math.max(3, (style.glowBlur || 0.35) * 12)}px ${style.glowColor || '#00e5d0'}` : style.shadow ? '0 2px 5px #000' : 'none'}">${escapeHtml(style.sample || style.label)}</span>
            <span class="caption-style-label">${escapeHtml(style.label)}${captionStyle === style.id ? icon('check', 11) : ''}</span>
          </button>`).join('')}
      </div>

      <div class="details-section" style="margin:8px 12px 6px">Animación de subtítulo</div>
      <div class="caption-anim-groups">
        ${animationGroups.map((group) => `
          <button class="caption-anim-group${captionAnimationGroup === group.id ? ' active' : ''}" data-caption-anim-group="${group.id}">${group.label}</button>`).join('')}
      </div>
      <div class="caption-anims">
        ${activeAnimationGroup.items.map((anim) => `
          <button class="caption-anim-card${captionAnimation === anim.id ? ' active' : ''}" data-caption-anim="${anim.id}" title="${escapeHtml(anim.label)}">
            <span class="caption-anim-preview anim-${anim.id.replace(/[^a-z0-9-]/gi, '')}" aria-hidden="true">
              <span class="caption-preview-copy">
                <span class="caption-preview-word word-1">MIRA</span>
                <span class="caption-preview-word word-2">TU</span>
                <span class="caption-preview-word word-3">IDEA</span>
              </span>
              ${captionAnimation === anim.id ? `<span class="caption-preview-check">${icon('check', 10)}</span>` : ''}
            </span>
            <span class="caption-anim-label">${escapeHtml(anim.label)}</span>
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
      const hasTarget = !!found && ['video', 'image'].includes(found.clip.kind);
      return filterGallery(hasTarget ? lineKeyForClip(found.clip) : null, true, filterCategory, hasTarget);
    }
    if (panel === 'transitions') return transitionPanel();
    if (panel === 'captions') {
      if (!whisperCheckedOnce && !whisperChecking) checkWhisperStatus();
      return captionsPanel();
    }
    if (panel === 'adjust') return adjustPanel();
    if (panel === 'mask') return maskPanel();
    if (panel === 'retouch') return retouchPanel();
    if (panel === 'aiStylize') return aiStylizePanel();
    if (panel === 'speed') return speedPanel();
    if (panel === 'animation') return animationPanel();
    if (panel === 'hsl') return hslPanel();
    if (panel === 'curves') return curvesPanel();
    if (panel === 'colorCircle') return colorCirclePanel();
    if (panel === 'audioFilter') return audioFilterPanel();
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
          body: `<div class="modal-note" style="margin:0">cupcraft funciona sin cuenta. Los proyectos y medios se guardan en el almacenamiento local de este navegador.</div>`,
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
        'La generación con IA necesita un servicio en la nube. cupcraft es un editor local y no incluye estas herramientas; puedes importar tus propios medios o usar las plantillas y los stickers.',
      );
      return;
    }

    if (event.target.closest('[data-space-create]')) {
      unavailable('Crear espacio de trabajo', 'Los espacios compartidos requieren una cuenta en la nube. En esta versión local, cada proyecto se guarda únicamente en tu navegador.');
      return;
    }
    if (event.target.closest('[data-space-info]')) {
      unavailable('Espacios', 'Un espacio es una carpeta compartida para trabajar con otras personas. cupcraft no tiene cuentas ni colaboración: todo se queda en este equipo.');
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
      loadSystemFonts({ refresh: true }).then((fonts) => {
        toast(fonts.length ? `${fonts.length} fuentes instaladas detectadas` : 'No se detectaron fuentes adicionales', fonts.length ? 'success' : 'warn');
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
      void generateTts();
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
        // `img` es ahora una miniatura local; el contenido real está en `asset`.
        const stickerSrc = item.asset || item.url || item.img;
        const clip = addStickerClip(item.name, {
          start: state.playhead,
          duration: 4,
          preset: { src: stickerSrc, srcFallback: item.img, size: 460 },
        });
        if (clip) {
          toast(`Sticker "${item.name}" añadido`, 'success', 1600);
          prepareStickerSrc(stickerSrc, item.img)
            .then((clean) => {
              if (clean && clean !== stickerSrc) updateClip(clip.id, { sticker: { ...clip.sticker, src: clean, srcFallback: null } });
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
      if (!item) return;
      toast(`Aplicando “${item.name}”…`, 'info', 1200);
      void capcutTextStyle(item).then((style) => {
        if (!style) return;
        const found = selectedClip();
        const hasTextClip = found?.clip?.kind === 'text';
        if (!hasTextClip) {
          const raw = String(item.name || '')
            .replace(/[_-]+/g, ' ')
            .replace(/\s+/g, ' ')
            .trim();
          // Usamos el nombre real de la plantilla como contenido: en CapCut las
          // portadas muestran ese texto y el usuario lo espera ver en el canvas.
          const sample = raw.slice(0, 28) || 'Texto';
          const preset = { text: { ...style.patch, size: 86, weight: 900, textCase: 'upper' } };
          addTextClip(sample || 'Tu texto aquí', { start: state.playhead, duration: 4, preset });
          window.dispatchEvent(new CustomEvent('opencup:glfx'));
          toast(`Estilo “${item.name}” añadido en el cursor`, 'success', 1800);
          return;
        }
        applyTextPatch(style.patch, `Estilo “${item.name}” aplicado`);
        window.dispatchEvent(new CustomEvent('opencup:glfx'));
      }).catch(() => toast('No se pudo leer el estilo de esa plantilla', 'warning'));
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
        if (style.id === 'karaoke') captionAnimation = 'caption-karaoke';
        else if (captionAnimation === 'caption-karaoke') captionAnimation = 'none';
        updateClip(found.clip.id, {
          text: {
            ...captionStyleTextPatch(style),
            ...captionAnimationConfig(captionAnimation, { ...(found.clip.text || {}), ...captionStyleTextPatch(style) }),
          },
        });
        toast(`Estilo “${style.label}” aplicado al subtítulo`, 'success', 1600);
      }
      render();
      return;
    }

    const captionAnimGroupButton = event.target.closest('[data-caption-anim-group]');
    if (captionAnimGroupButton) {
      captionAnimationGroup = captionAnimGroupButton.dataset.captionAnimGroup;
      render();
      return;
    }

    const captionAnimButton = event.target.closest('[data-caption-anim]');
    if (captionAnimButton) {
      captionAnimation = captionAnimButton.dataset.captionAnim;
      const targets = state.project?.tracks
        .flatMap((track) => track.clips)
        .filter((clip) => clip.kind === 'text');
      const selected = selectedClip()?.clip;
      const applyTo = selected?.kind === 'text' ? [selected] : (targets || []);
      for (const clip of applyTo) {
        updateClip(clip.id, {
          text: {
            ...captionAnimationConfig(captionAnimation, clip.text || {}),
          },
        });
      }
      const label = [...TEXT_ANIMATIONS, { id: 'none', label: 'Sin animación' }, { id: 'caption-karaoke', label: 'Palabra amarilla' }, { id: 'caption-pop', label: 'Salto' }]
        .find((item) => item.id === captionAnimation)?.label || 'Animación aplicada';
      toast(captionAnimation === 'none' ? 'Animación de subtítulo eliminada' : `“${label}” aplicado`, 'success', 1600);
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
      addSubtitleClips(
        [{ text: 'Escribe tu subtítulo', start: state.playhead, duration: 3 }],
        { captionStyleId: captionStyle, captionAnimation, replace: false },
      );
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
  /* Arrastre de puntos de la curva de color (Ajustar → Curvas). */
  let curveDrag = null;
  let curveDraggedAt = 0;

  const ratioLabel = (ratio) => (ratio === '16:9' ? 'Original' : ratio);

  /* Pestañas de Detalles con la misma organización que CapCut. */
  const DETAIL_TABS = [
    { id: 'video', label: 'Video', subs: [['basic', 'Básico'], ['cutout', 'Eliminar fondo'], ['mask', 'Máscara'], ['retouch', 'Retoque']] },
    { id: 'audio', label: 'Audio', subs: [['basic', 'Básico'], ['voice', 'Modificador de voz']] },
    { id: 'speed', label: 'Velocidad', subs: [['standard', 'Estándar'], ['curve', 'Curva'], ['fx', 'Efectos de velocidad']] },
    { id: 'animation', label: 'Animación', subs: [['in', 'Entrada'], ['out', 'Salida'], ['combo', 'Combinado']] },
    { id: 'adjust', label: 'Ajustar', subs: [['basic', 'Básico'], ['hsl', 'HSL'], ['curves', 'Curvas'], ['wheel', 'Círculo cromático'], ['mask', 'Máscara']] },
    { id: 'ai', label: 'Estilizado de IA', subs: [['face', 'Rostro'], ['body', 'Cuerpo'], ['hair', 'Cabello'], ['paragraph', 'Párrafos'], ['bodyParagraph', 'Párrafos corporales']] },
  ];
  /* Un clip de texto solo tiene detalles de texto: nada de Video, Audio,
     Velocidad, Ajustar ni Estilizado de IA. Coincide con el panel de CapCut. */
  const TEXT_DETAIL_TABS = [
    { id: 'text', label: 'Texto', subs: [['basic', 'Básico'], ['bubble', 'Burbuja'], ['effects', 'Efectos']] },
    { id: 'textanim', label: 'Animación', subs: [['in', 'Entrada'], ['loop', 'Bucle'], ['out', 'Salida'], ['caption', 'Subtítulos']] },
    { id: 'tracking', label: 'Seguimiento', subs: [['tracking', 'Seguimiento']] },
    { id: 'tts', label: 'Texto a voz', subs: [['basic', 'Básico']] },
    { id: 'avatar', label: 'Avatares de IA', subs: [['basic', 'Básico']] },
  ];
  let detailTab = 'video';
  const detailSub = {
    video: 'basic', audio: 'basic', speed: 'standard', animation: 'in', adjust: 'basic', ai: 'face',
    text: 'basic', textanim: 'in', tracking: 'tracking', tts: 'basic', avatar: 'basic',
  };

  const VOICE_FILTERS = [
    'Original', 'Aumento de graves', 'Supergrave', 'Perdiendo la cordura', 'Eco con eco', 'Locutora de megáfono',
    'Problema doble', 'Cinta de casete', 'TV de tubo', 'Micrófono de bajos', 'Ecos infinitos', 'Voz llena',
    'Micrófono roto', 'Bot cósmico', 'Voces claras', 'Eco de adulto', 'Adolescente', 'Dinámico', 'Micrófono de estudio',
    'Punto de eternidad', 'Supertermal', 'Cristalina', 'Casa grande', 'Bot amable', 'Eco místico', 'Ahogado',
    'Sala de conciertos', 'Eco II', 'Acordes mágicos', 'Droide retro', 'Retumbante', 'Magnético', 'Frecuencia cantante',
    'Infierno metálico', 'Profesión de DJ', 'Robótica', 'Superestrés', 'Música alienígena', 'Radio FM', 'Rebumbante',
    'Recuerdos', 'Fonógrafo', 'Susurros', 'Alto', 'Distorsionado', 'Megáfono', 'Infierno', 'Micrófono rústico',
    'Micrófono antiguo', 'Reproductor de juguete', 'Voz clara VHS', 'Alma perdida', 'Micrófono basura', '8 bits',
    'Rey de los conciertos', 'Teléfono', 'Radio de la RCA', 'Espacio', 'Voz doble', 'Teléfono analógico',
    'Hablando a la mano', 'Cámara antigua', 'Edición dividida', 'Ventilador', 'Tuberías metálicas',
    'Auriculares de consola', 'Micrófono viejo', 'Bucle de error', 'Transmisión en vivo', 'Melodía de mercado',
    'Broadway 1', 'En el vientre', 'Panorámica', 'Dron con altavoz', 'Estudio vacío', 'Archibald', 'Explosión envolvente',
    'Distorsión lenta', 'Sala de ensayo', 'Club nocturno', 'Paisaje sonoro 9D', 'Eco de cabina', 'Enmarcado',
    'Voz carcajada', 'Dimensión exterior', 'Tictac de reloj', 'Android 0G', 'Simulador de aleatorio', 'Celular barato',
    'Máscara de guardia', 'Cacao de motocicleta', 'Cabeza de cubo',
  ];

  function projectMarkup() {
    const project = state.project;
    return `
      <div class="detail-list project-detail-list">
        <div class="detail-item"><span class="d-label">Nombre:</span><span class="d-value strong" title="${escapeHtml(project.name)}">${escapeHtml(project.name)}</span></div>
        <div class="detail-item"><span class="d-label">Ruta:</span><span class="d-value path" title="cupcraft / Proyectos / ${escapeHtml(project.name)}">cupcraft / Proyectos / ${escapeHtml(project.name)}</span></div>
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
              ${['480p', '720p', '1080p', '1440p', '4K'].map((key) => `<option ${key === project.resolution ? 'selected' : ''}>${key}</option>`).join('')}
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

  const MASK_SHAPES = [
    ['none', 'Ninguna'],
    ['linear', 'Lineal'],
    ['mirror', 'Espejo'],
    ['circle', 'Círculo'],
    ['rect', 'Rectángulo'],
    ['rounded', 'Redondeado'],
    ['heart', 'Corazón'],
    ['star', 'Estrella'],
    ['triangle', 'Triángulo'],
  ];

  function switchRow(label, key, value, { badge = '', note = '', collapsed = false } = {}) {
    const on = !!value;
    const inner = `<div class="detail-row${note ? ' has-note' : ''}">
      <div class="detail-row-main">
        <span class="detail-row-label">${label}${badge ? `<i class="detail-badge">${badge}</i>` : ''}</span>
        <button class="switch${on ? ' on' : ''}" type="button" data-clip-switch="${key}" aria-pressed="${on}" aria-label="${label}"></button>
      </div>
      ${note ? `<div class="detail-row-note">${note}</div>` : ''}
    </div>`;
    if (collapsed) {
      return `<details class="tx-group" data-section="sw-${key}">
        <summary>${label}${badge ? ` <i class="detail-badge">${badge}</i>` : ''}</summary>
        <div class="switch-collapsed-body">${inner}</div>
      </details>`;
    }
    return inner;
  }

  function lockedRow(label, badge = 'IA', note = '') {
    return `<button type="button" class="detail-row detail-row-btn" data-ai-locked="${escapeHtml(label)}">
      <div class="detail-row-main">
        <span class="detail-row-label">${label}${badge ? `<i class="detail-badge">${badge}</i>` : ''}</span>
        ${icon('chevronRight', 12)}
      </div>
      ${note ? `<div class="detail-row-note">${note}</div>` : ''}
    </button>`;
  }

  function maskGrid(clip) {
    const active = clip.mask?.shape || 'none';
    return `<div class="mask-grid">
      ${MASK_SHAPES.map(([id, label]) => `
        <button type="button" class="mask-tile${active === id ? ' active' : ''}" data-mask-shape="${id}" title="${label}">
          <span class="mask-shape mask-shape-${id}"></span>
          <span>${label}</span>
        </button>`).join('')}
    </div>`;
  }

  function maskControls(clip) {
    const mask = clip.mask || {};
    const shape = mask.shape || 'none';
    return `
      <div class="detail-subhead">Máscara<button class="btn btn-small btn-ghost" data-mask-remove>${icon('trash', 12)} Quitar</button></div>
      ${maskGrid(clip)}
      ${shape === 'none'
        ? `<div class="detail-hint">Elige una forma para recortar el clip. Después ajusta su posición, escala y rotación.</div>`
        : `
        ${detailRange('Pluma', 'mask.feather', Math.round(mask.feather ?? 0), 0, 100, 1, 1, Math.round(mask.feather ?? 0), '%')}
        ${detailRange('Escala', 'mask.scale', Math.round((mask.scale ?? 1) * 100), 10, 200, 1, 0.01, Math.round((mask.scale ?? 1) * 100), '%')}
        ${detailRange('Rotación', 'mask.rotate', Math.round(mask.rotate ?? 0), -180, 180, 1, 1, Math.round(mask.rotate ?? 0), '°')}
        ${detailRange('Posición X', 'mask.x', Math.round(mask.x ?? 0), -100, 100, 1, 1, Math.round(mask.x ?? 0), '%')}
        ${detailRange('Posición Y', 'mask.y', Math.round(mask.y ?? 0), -100, 100, 1, 1, Math.round(mask.y ?? 0), '%')}
        ${switchRow('Invertir', 'mask.invert', !!mask.invert)}`}`;
  }

  function voiceGrid(clip) {
    const active = clip.voice || 'Original';
    return `
      <div class="voice-search-row">
        <span class="detail-badge">Efecto de voz</span>
        <button class="btn btn-small btn-ghost" data-voice="Original">Restablecer</button>
      </div>
      <div class="voice-grid">
        ${VOICE_FILTERS.map((name) => `<button type="button" class="voice-chip${active === name ? ' active' : ''}" data-voice="${escapeHtml(name)}">${escapeHtml(name)}</button>`).join('')}
      </div>
      <div class="detail-hint">El filtro de voz se aplica al audio del clip en la vista previa y en la exportación.</div>`;
  }

  function dbRow(clip) {
    const vol = clip.volume ?? 1;
    const db = vol > 0 ? Math.max(-30, Math.min(6, 20 * Math.log10(vol))) : -30;
    const shown = Math.round(db * 2) / 2;
    return `<div class="prop">
      <span class="tx-lbl">Volumen</span>
      <input type="range" data-clip-db min="-30" max="6" step="0.5" value="${shown}" />
      <b class="tx-val" data-db-readout>${shown === 0 ? '0.0' : shown.toFixed(1)}dB</b>
    </div>`;
  }

  function audioBasicTab(clip) {
    return `
      <div class="detail-subhead">Básico</div>
      ${dbRow(clip)}
      ${detailRange('Aparición progresiva', 'fadeIn', Math.round((clip.fadeIn ?? 0) * 10) / 10, 0, 10, 0.1, 1, (clip.fadeIn ?? 0).toFixed(1), 's')}
      ${detailRange('Desaparición progresiva', 'fadeOut', Math.round((clip.fadeOut ?? 0) * 10) / 10, 0, 10, 0.1, 1, (clip.fadeOut ?? 0).toFixed(1), 's')}
      ${switchRow('Normalizar la intensidad', 'normalize', !!clip.normalize, { badge: 'PRO', note: 'Normaliza el clip al valor objetivo.' })}
      ${lockedRow('Mejorar voz', 'IA')}
      ${lockedRow('Traductor de videos', 'IA', 'Traduce la voz del clip a otro idioma.')}
      ${lockedRow('Reducir el ruido', 'IA', 'Elimina el ruido de fondo del audio.')}
      ${lockedRow('Aislar voz', 'PRO', 'Separa la voz de la música y los efectos.')}
      ${lockedRow('Completar canal', '', 'Convierte el audio mono en estéreo.')}`;
  }

  function speedTiles(clip) {
    const active = clip.speedCurve || 'none';
    const curves = [
      ['none', 'Ninguno'],
      ['custom', 'Personalizado'],
      ['montage', 'Montaje'],
      ['hero', 'Héroe'],
      ['bullet', 'Bala'],
      ['cut', 'Corte en el ...'],
      ['flashIn', 'Flash de entra...'],
      ['flashOut', 'Flash de salida'],
    ];
    return `<div class="curve-grid">
      ${curves.map(([id, label]) => `
        <button type="button" class="curve-tile${active === id ? ' active' : ''}" data-speed-curve="${id}" title="${label}">
          <span class="curve-art curve-${id}"></span>
          <span>${label}</span>
        </button>`).join('')}
    </div>
    <div class="detail-hint">Las curvas de velocidad marcan aceleraciones progresivas dentro del clip.</div>`;
  }

  function speedFxTiles(clip) {
    const active = clip.speedFx || 'none';
    const items = [
      ['none', 'Ninguno'],
      ['flash', 'Flash'],
      ['motion', 'Movimiento'],
      ['fade', 'Desvanecer'],
      ['zoom', 'Zoom retro'],
      ['alcohol', 'Alcohol'],
    ];
    return `<div class="fx-tile-grid">
      ${items.map(([id, label]) => `
        <button type="button" class="fx-tile${active === id ? ' active' : ''}" data-speed-fx="${id}" title="${label}">
          <span class="fx-thumb speed-thumb-${id}"></span>
          <span>${label}</span>
        </button>`).join('')}
    </div>`;
  }

  function speedTab(clip, sub) {
    if (sub === 'curve') return speedTiles(clip);
    if (sub === 'fx') return speedFxTiles(clip);
    const speed = clip.speed ?? 1;
    const sourceDuration = clip.mediaId ? state.media.get(clip.mediaId)?.duration : null;
    const outDuration = sourceDuration ? (clip.duration || 0) : clip.duration;
    return `
      <div class="detail-subhead">Estándar</div>
      ${detailRange('Velocidad', 'speed', Math.round(speed * 100), 10, 1000, 5, 0.01, speed.toFixed(2), '×')}
      <div class="detail-row">
        <div class="detail-row-main"><span class="detail-row-label">Duración</span><b class="detail-value">${formatTime(outDuration || 0)}</b></div>
      </div>
      ${switchRow('Cambiar tono del audio', 'preservePitch', clip.preservePitch !== false, { note: 'Si se desactiva, el audio mantiene su tono al cambiar la velocidad.' })}
      <div class="detail-hint">Cámara lenta fluida: solo se admiten videos en cámara lenta.</div>`;
  }

  function animationTab(clip, sub) {
    const cat = sub === 'in' ? 'Entrada' : sub === 'out' ? 'Salida' : 'Combinado';
    const list = CAPCUT_EFFECT_ITEMS
      .filter((item) => item.cats.includes(cat))
      .slice(0, 80);
    const activeName = sub === 'in' ? clip.animIn : sub === 'out' ? clip.animOut : clip.animCombo;
    if (!list.length) {
      return `<div class="detail-hint">Añade un clip de video o imagen para aplicar animaciones de ${cat.toLowerCase()}.</div>`;
    }
    return `
      <div class="fx-tile-grid">
        <button type="button" class="fx-tile${!activeName ? ' active' : ''}" data-anim-kind="${sub}" data-anim-effect="">
          <span class="fx-thumb anim-none">⊘</span><span>Ninguno</span>
        </button>
        ${list.map((item) => `
          <button type="button" class="fx-tile${activeName === item.name ? ' active' : ''}" data-anim-kind="${sub}" data-anim-effect="${escapeHtml(item.name)}" data-anim-id="${item.id}" title="${escapeHtml(item.name)}">
            <img class="fx-thumb" src="${item.img}" alt="" loading="lazy" decoding="async" onerror="this.onerror=null;this.replaceWith(Object.assign(document.createElement('span'),{className:'fx-thumb anim-none',textContent:'✦'}))" />
            <span>${escapeHtml(item.name)}</span>
          </button>`).join('')}
      </div>`;
  }

  function hslTab(clip) {
    const hsl = clip.hsl || {};
    const colors = [
      ['red', 'Rojos', '#ff3b3b'],
      ['orange', 'Naranjas', '#ff9f1a'],
      ['yellow', 'Amarillos', '#ffe14a'],
      ['green', 'Verdes', '#4ade80'],
      ['cyan', 'Cianes', '#22d3ee'],
      ['blue', 'Azules', '#3b82f6'],
      ['purple', 'Morados', '#a855f7'],
      ['magenta', 'Magentas', '#ec4899'],
    ];
    const active = hsl.color || 'red';
    return `
      <div class="hsl-swatches">
        ${colors.map(([id, , color]) => `<button type="button" class="hsl-swatch${active === id ? ' active' : ''}" data-hsl-color="${id}" style="--swatch:${color}" title="${id}"></button>`).join('')}
      </div>
      ${detailRange('Matiz', 'hsl.hue', Math.round(hsl.hue ?? 0), -180, 180, 1, 1, Math.round(hsl.hue ?? 0), '°')}
      ${detailRange('Saturación', 'hsl.sat', Math.round(hsl.sat ?? 0), -100, 100, 1, 1, Math.round(hsl.sat ?? 0), '')}
      ${detailRange('Brillo', 'hsl.light', Math.round(hsl.light ?? 0), -100, 100, 1, 1, Math.round(hsl.light ?? 0), '')}
      <div class="detail-hint">Ajusta cada familia de color por separado (como el HSL de CapCut).</div>`;
  }

  /* Genera una curva suave (Catmull-Rom aproximada) a partir de los puntos. */
  function curvePathFromPoints(points) {
    const sorted = [...(points || [])].sort((a, b) => a[0] - b[0]);
    if (!sorted.length) return 'M0 100 L100 0';
    let d = `M${sorted[0][0] * 100} ${100 - sorted[0][1] * 100}`;
    for (let index = 0; index < sorted.length - 1; index += 1) {
      const from = sorted[index];
      const to = sorted[index + 1];
      const cx = ((from[0] + to[0]) / 2) * 100;
      d += ` C${cx} ${100 - from[1] * 100}, ${cx} ${100 - to[1] * 100}, ${to[0] * 100} ${100 - to[1] * 100}`;
    }
    return d;
  }

  function curvesTab(clip) {
    const curves = clip.curves || {};
    const channel = clip.curveChannel || 'luma';
    const channels = [['luma', 'Luma'], ['red', 'Rojo'], ['green', 'Verde'], ['blue', 'Azul']];
    const points = curves[channel] || [];
    const dots = points.map(([x, y], index) => `<i class="curve-dot" data-curve-point="${index}" style="left:${x * 100}%;bottom:${y * 100}%"></i>`).join('');
    return `
      <div class="curve-channels">
        ${channels.map(([id, label]) => `<button type="button" class="curve-channel${channel === id ? ' active' : ''}" data-curve-channel="${id}">${label}</button>`).join('')}
      </div>
      <div class="curve-box curve-box-${channel}" data-curve-box data-curve-box-channel="${channel}">
        <div class="curve-grid-lines"></div>
        <svg class="curve-line" viewBox="0 0 100 100" preserveAspectRatio="none">
          <path d="${curvePathFromPoints(points)}" />
        </svg>
        ${dots}
      </div>
      <div class="detail-hint">Haz clic en la curva para añadir un punto. Arrastra los círculos para ajustar la curva y haz doble clic en un círculo para quitarlo.</div>`;
  }

  function wheelTab(clip) {
    const wheels = clip.wheels || {};
    const items = [['shadow', 'Sombras'], ['midtone', 'Gris intermedio'], ['tint', 'Tinte'], ['offset', 'Desplazamiento']];
    return `
      <div class="wheel-row">
        ${items.map(([id, label]) => {
          const value = wheels[id] || {};
          const x = Number(value.x ?? 0);
          const y = Number(value.y ?? 0);
          return `<div class="wheel">
            <span class="wheel-label">${label}</span>
            <button type="button" class="wheel-disc" data-wheel="${id}"><i style="left:${50 + x * 40}%;top:${50 + y * 40}%"></i></button>
            <div class="wheel-vals">
              <span>${x.toFixed(2)}</span><span>${y.toFixed(2)}</span><span>${Number(value.z ?? 0).toFixed(2)}</span>
            </div>
          </div>`;
        }).join('')}
      </div>
      <div class="detail-hint">Arrastra el punto del círculo para teñir sombras, medios y luces.</div>`;
  }

  function adjustBasicTab(clip) {
    const a = clip.adjust || {};
    const val = (key, fallback) => Math.round(a[key] ?? fallback);
    const luts = Object.keys(FILTERS);
    return `
      <div class="detail-subhead">Básico</div>
      ${detailRange('Intensidad', 'adjust.intensity', val('intensity', 80), 0, 100, 1, 1, val('intensity', 80), '')}
      ${switchRow('Combinación de colores', 'colorMix', !!clip.colorMix, { badge: 'PRO' })}
      ${detailRange('Corrección del color', 'adjust.correction', val('correction', 80), 0, 100, 1, 1, val('correction', 80), '')}
      <div class="field">
        <label>LUT</label>
        <select class="input" data-clip-filter>
          ${luts.map((key) => `<option value="${key}" ${key === (clip.filter || 'none') ? 'selected' : ''}>${FILTERS[key].label}</option>`).join('')}
        </select>
      </div>
      <div class="details-section">Ajustar</div>
      ${detailRange('Temp.', 'adjust.temperature', val('temperature', 0), -100, 100, 1, 1, val('temperature', 0), '')}
      ${detailRange('Tinte', 'adjust.tint', val('tint', 0), -100, 100, 1, 1, val('tint', 0), '')}
      ${detailRange('Saturación', 'adjust.saturate', val('saturate', 100), 0, 300, 1, 1, val('saturate', 100), '')}
      ${detailRange('Claridad', 'adjust.clarity', val('clarity', 0), -100, 100, 1, 1, val('clarity', 0), '')}
      ${detailRange('Exposición', 'adjust.exposure', val('exposure', 0), -100, 100, 1, 1, val('exposure', 0), '')}
      ${detailRange('Contraste', 'adjust.contrast', val('contrast', 100), 0, 200, 1, 1, val('contrast', 100), '')}
      ${detailRange('Luminar', 'adjust.highlight', val('highlight', 0), -100, 100, 1, 1, val('highlight', 0), '')}
      ${detailRange('Sombra', 'adjust.shadow', val('shadow', 0), -100, 100, 1, 1, val('shadow', 0), '')}
      ${detailRange('Blancos', 'adjust.whites', val('whites', 0), -100, 100, 1, 1, val('whites', 0), '')}
      ${detailRange('Negros', 'adjust.blacks', val('blacks', 0), -100, 100, 1, 1, val('blacks', 0), '')}
      ${detailRange('Luminosidad', 'adjust.brightness', val('brightness', 100), 0, 200, 1, 1, val('brightness', 100), '%')}
      <div class="details-section">Efectos</div>
      ${detailRange('Enfoque', 'adjust.sharpen', val('sharpen', 0), 0, 100, 1, 1, val('sharpen', 0), '')}
      ${detailRange('Partículas', 'adjust.particle', val('particle', 0), 0, 100, 1, 1, val('particle', 0), '')}
      ${detailRange('Desvanecer', 'adjust.fade', val('fade', 0), 0, 100, 1, 1, val('fade', 0), '')}
      ${detailRange('Viñeta', 'adjust.vignette', val('vignette', 0), 0, 100, 1, 1, val('vignette', 0), '')}`;
  }

  function retouchTab(clip) {
    const beauty = clip.beauty || {};
    const v = (key) => Math.round(beauty[key] ?? 0);
    return `
      <div class="detail-subhead">Retoque</div>
      ${detailRange('Suavizar piel', 'beauty.smooth', v('smooth'), 0, 100, 1, 1, v('smooth'), '')}
      ${detailRange('Blanquear', 'beauty.whiten', v('whiten'), 0, 100, 1, 1, v('whiten'), '')}
      ${detailRange('Sonrojo', 'beauty.rosy', v('rosy'), 0, 100, 1, 1, v('rosy'), '')}
      ${lockedRow('Adelgazar', 'IA')}
      ${lockedRow('Ojos', 'IA')}
      ${lockedRow('Dientes', 'IA')}
      <div class="detail-hint">El suavizado y el blanqueo se aplican al clip en la vista previa y en la exportación.</div>`;
  }

  function aiTab(clip, sub) {
    const groups = {
      face: [['skinSmooth', 'Suavizado de piel'], ['skinWhiten', 'Blanqueo de piel'], ['skinRosy', 'Sonrojo'], ['faceSlim', 'Adelgazar rostro'], ['eyes', 'Aumentar ojos'], ['teeth', 'Blanquear dientes']],
      body: [['bodySlim', 'Adelgazar cuerpo'], ['legs', 'Alargar piernas'], ['waist', 'Cintura'], ['arms', 'Brazos'], ['shoulders', 'Hombros']],
      hair: [['volume', 'Volumen'], ['shine', 'Brillo'], ['color', 'Color'], ['edges', 'Contorno']],
      paragraph: [['spacing', 'Espaciado'], ['size', 'Tamaño'], ['weight', 'Grosor'], ['color', 'Color']],
      bodyParagraph: [['spacing', 'Espaciado'], ['size', 'Tamaño'], ['weight', 'Grosor'], ['align', 'Alineación']],
    };
    const labels = { face: 'Rostro', body: 'Cuerpo', hair: 'Cabello', paragraph: 'Párrafos', bodyParagraph: 'Párrafos corporales' };
    const entries = groups[sub] || [];
    const store = clip.aiStyle?.[sub] || {};
    return `
      <div class="detail-subhead">${labels[sub] || 'Estilizado'}</div>
      ${entries.map(([key, label]) => detailRange(
        label,
        `aiStyle.${sub}.${key}`,
        Math.round(store[key] ?? 0),
        0, 100, 1, 1,
        Math.round(store[key] ?? 0),
        '',
      )).join('')}
      <div class="detail-hint">El estilizado de IA avanzado (retoque facial real, cabello y cuerpo) requiere el motor de IA en la nube. Los deslizadores guardan el valor para cuando esté disponible.</div>`;
  }

  function cutoutTab(clip) {
    const cutoutStroke = clip.cutoutStroke || {};
    const customCutout = clip.customCutout || {};
    const chromaKey = clip.chromaKey || {};
    const strokeStyles = [
      { id: 'none', label: 'Ninguno', icon: 'circle-slash' },
      { id: 'paper', label: 'Canto papel', icon: 'edit' },
      { id: 'glow', label: 'Luminiscencia', icon: 'sparkles' },
      { id: 'hand', label: 'Dibujo mano', icon: 'pencil' },
      { id: 'cutout', label: 'Contorno cortado', icon: 'scissors' },
      { id: 'solid', label: 'Contorno sólido', icon: 'square' },
      { id: 'wind', label: 'Contorno viento', icon: 'wave' },
      { id: 'dotted', label: 'Contorneado', icon: 'more' },
    ];
    const activeStroke = cutoutStroke.type || 'none';

    return `
      ${switchRow('Eliminación automática', 'autoCutout', !!clip.autoCutout, { badge: 'IA' })}
      ${dtSliderRow('Intensidad', 'autoCutoutIntensity', clip.autoCutoutIntensity ?? 1, { min: 0, max: 100, step: 1, factor: 0.01, unit: '%', clip })}
      ${dtSliderRow('Estabilidad temporal', 'autoCutoutTemporal', clip.autoCutoutTemporal ?? 0.3, { min: 0, max: 100, step: 1, factor: 0.01, unit: '%', clip })}
      ${dtSliderRow('Limpieza de bordes', 'autoCutoutEdge', clip.autoCutoutEdge ?? 0.35, { min: 0, max: 100, step: 1, factor: 0.01, unit: '%', clip })}
      <div class="detail-hint" style="margin: 6px 0;">
        BackgroundMattingV2 (local, MIT): usa una <b>foto del fondo</b> o un <b>color sólido</b> para recortar.
        La estabilidad temporal y la limpieza de bordes son postproceso propio: reducen el parpadeo y el rastro
        sin usar modelos ni código de CapCut.
      </div>
      <div class="detail-row" style="margin: 6px 0 4px;">
        <div class="detail-row-main">
          <span class="detail-row-label">Color del fondo</span>
          <div class="inline-field">
            <input type="color" class="input" data-bg-color value="${colorValue(clip.autoCutoutBgColor || '#00b140')}" />
            <b class="dt-val" data-bg-color-readout>${colorValue(clip.autoCutoutBgColor || '#00b140')}</b>
            <button class="btn btn-small btn-ghost" type="button" data-bg-color-use>Elegir color</button>
          </div>
        </div>
      </div>
      <div class="detail-row" style="margin: 6px 0;">
        <div class="detail-row-main">
          <span class="detail-row-label">Foto del fondo</span>
          <button class="btn btn-small btn-ghost" type="button" data-bg-plate-pick>Elegir…</button>
        </div>
      </div>
      <input type="file" accept="image/*" data-bg-plate-input class="hidden" />
      <div class="detail-hint" style="margin-top: 4px;">${clip.autoCutoutBg
        ? `Fondo: <b>${escapeHtml(state.media.get(clip.autoCutoutBg)?.name || 'seleccionado')}</b>`
        : '<b>Sin foto de fondo.</b> Puedes usar el color de arriba.'}</div>
      ${switchRow('Trazo', 'cutoutStroke.active', !!cutoutStroke.active, { collapsed: true })}
      <div class="stroke-grid" style="display: flex; flex-wrap: wrap; gap: 6px; margin: 8px 0;">
        ${strokeStyles.map(s => `
          <div class="stroke-tile ${activeStroke === s.id ? 'active' : ''}" data-cutout-stroke="${s.id}" style="flex: 1 1 calc(25% - 6px); min-width: 60px; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 8px 4px; font-size: 9px; text-align: center; background: ${activeStroke === s.id ? '#1a3a3a' : '#333'}; border: 1px solid ${activeStroke === s.id ? '#00e5d0' : '#555'}; border-radius: 4px; cursor: pointer;">
            <div style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; margin-bottom: 4px;">
              ${icon(s.icon, 20)}
            </div>
            <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; width: 100%;">${s.label}</span>
          </div>
        `).join('')}
      </div>

      <div class="field">
        <label>Color</label>
        <div class="inline-field">
          <input type="color" class="input" data-nested-prop="cutoutStroke.color" value="${cutoutStroke.color || '#ffffff'}" />
          <b>${cutoutStroke.color || '#ffffff'}</b>
        </div>
      </div>
      ${detailRange('Tamaño', 'cutoutStroke.size', Math.round(cutoutStroke.size ?? 10), 0, 100, 1, 1, Math.round(cutoutStroke.size ?? 10))}
      ${detailRange('Espaciado', 'cutoutStroke.spacing', Math.round(cutoutStroke.spacing ?? 10), 0, 100, 1, 1, Math.round(cutoutStroke.spacing ?? 10))}

      <div class="details-sep"></div>

      ${switchRow('Eliminación personalizada', 'customCutout.active', !!customCutout.active, { badge: 'IA', collapsed: true })}
      <div class="detail-hint" style="margin: 6px 0;">Pinta sobre el <b>video del reproductor</b> para marcar lo que quieres borrar. El clip seleccionado es <b>${escapeHtml(clip?.name || 'ninguno')}</b>.</div>
      <div class="brush-grid" style="display: flex; gap: 6px; margin: 8px 0;">
        <button class="btn btn-ghost btn-small ${customCutout.tool === 'brush' ? 'active' : ''}" style="flex: 1;" data-cutout-tool="brush" title="Pincel">${icon('edit', 14)} Pincel</button>
        <button class="btn btn-ghost btn-small ${customCutout.tool === 'quick' ? 'active' : ''}" style="flex: 1;" data-cutout-tool="quick" title="Pincel rápido">${icon('wand', 14)} Rápido</button>
        <button class="btn btn-ghost btn-small ${customCutout.tool === 'lasso' ? 'active' : ''}" style="flex: 1;" data-cutout-tool="lasso" title="Lazo">${icon('mask', 14)} Lazo</button>
        <button class="btn btn-ghost btn-small ${customCutout.tool === 'eraser' ? 'active' : ''}" style="flex: 1;" data-cutout-tool="eraser" title="Borrador">${icon('trash', 14)} Borrador</button>
      </div>
      ${detailRange('Tamaño', 'customCutout.size', Math.round(customCutout.size ?? 20), 1, 100, 1, 1, Math.round(customCutout.size ?? 20))}
      <div style="display:flex;gap:6px;margin-bottom:8px;">
        <button class="btn btn-small btn-ghost" style="flex:1" data-draw-clear>Limpiar todo</button>
        <button class="btn btn-small ${(customCutout.strokes || []).length && !customCutout.applied ? 'btn-primary' : 'btn-ghost'}" style="flex:1" data-cutout-apply ${customCutout.applied && (customCutout.strokes || []).length ? 'disabled' : ''}>${customCutout.applied && (customCutout.strokes || []).length ? 'Recorte aplicado ✓' : 'Aplicar borrado'}</button>
      </div>
      <div class="detail-hint" style="margin-top:4px">${customCutout.applied && (customCutout.strokes || []).length
        ? 'Borrado fijado. Elige una herramienta para volver a editar las zonas.'
        : 'Selecciona una herramienta y <b>pinta directamente sobre el video central</b>. Después pulsa <b>Aplicar borrado</b>.'}</div>

      <div class="details-sep"></div>

      ${switchRow('Chroma key', 'chromaKey.active', !!chromaKey.active, { collapsed: true })}
      <div class="detail-row">
        <span class="detail-row-label">Selector de colores</span>
        <div class="inline-field" style="gap: 8px">
          <button class="btn btn-small btn-ghost" type="button" style="padding: 4px" data-chroma-pick title="Tomar el color del video">${icon('eyedropper', 14)}</button>
          <input type="color" class="input" data-chroma-color value="${colorValue(chromaKey.color || '#00ff00')}" style="width: 34px; height: 26px; padding: 0;" />
          <b class="dt-val" data-chroma-readout>${colorValue(chromaKey.color || '#00ff00')}</b>
        </div>
        <span class="muted" style="font-size: 9px; margin-left: 8px">Elige el color y, luego, ajusta la intensidad.</span>
      </div>
      ${detailRange('Intensidad', 'chromaKey.strength', Math.round(chromaKey.strength ?? 20), 0, 100, 1, 1, Math.round(chromaKey.strength ?? 20))}
      ${detailRange('Sombra', 'chromaKey.shadow', Math.round(chromaKey.shadow ?? 0), 0, 100, 1, 1, Math.round(chromaKey.shadow ?? 0))}
      ${detailRange('Borde de plumas', 'chromaKey.feather', Math.round(chromaKey.feather ?? 0), 0, 100, 1, 1, Math.round(chromaKey.feather ?? 0), '', 'Free')}
      ${detailRange('Limpiar borde', 'chromaKey.cleanup', Math.round(chromaKey.cleanup ?? 0), 0, 100, 1, 1, Math.round(chromaKey.cleanup ?? 0), '', 'IA')}

      <div class="details-sep"></div>
      <button type="button" class="btn btn-block" data-detail-sub-jump="mask">${icon('mask', 14)} Recortar con una máscara</button>
    `;
  }

  function detailTextSample(patch) {
    const p = patch || {};
    const background = p.background && p.background !== 'transparent'
      ? `background:${p.background};padding:2px 7px;border-radius:6px;` : '';
    const border = p.backgroundBorderWidth > 0
      ? `border:1px solid ${p.backgroundBorderColor || '#000'};` : '';
    const stroke = p.strokeWidth > 0
      ? `-webkit-text-stroke:${Math.max(1, Math.round(p.strokeWidth * 14))}px ${p.strokeColor || '#000'};` : '';
    const glow = p.glow ? `text-shadow:0 0 8px ${p.glowColor || '#00e5d0'};` : '';
    const shadow = p.shadow ? `text-shadow:2px 2px 4px ${p.shadowColor || '#000'};` : '';
    return `color:${p.color || '#ffffff'};font-weight:${p.weight || 800};${background}${border}${stroke}${glow}${shadow}`;
  }

  /* Cuerpo de los Detalles de un clip de texto. Solo controles de texto. */
  function textDetailTabBody(clip, tab, sub) {
    const text = { ...TEXT_DEFAULTS, ...(clip.text || {}) };
    if (tab === 'text') {
      if (sub === 'bubble') {
        return `
          <div class="detail-hint">Elige una burbuja: se aplica al texto seleccionado.</div>
          <div class="tx-style-grid">
            ${TEXT_BUBBLES.map((bubble) => `
              <button type="button" class="tx-style-swatch" data-detail-bubble="${escapeHtml(bubble.id)}"
                title="${escapeHtml(bubble.label)}" style="${detailTextSample(bubble.patch)}">Aa</button>`).join('')}
          </div>
          <div class="detail-hint">También puedes ajustar fondo, borde y radio en Básico → Fondo.</div>`;
      }
      if (sub === 'effects') {
        return `
          <div class="detail-hint">Efectos que se aplican al texto seleccionado.</div>
          <div class="tx-style-grid">
            ${TEXT_EFFECTS.map((effect) => `
              <button type="button" class="tx-style-swatch" data-detail-text-effect="${escapeHtml(effect.id)}"
                title="${escapeHtml(effect.label)}" style="${detailTextSample(effect.patch)}">Aa</button>`).join('')}
          </div>
          <div class="detail-subhead">Estilo preestablecido</div>
          <div class="tx-style-grid">
            ${TEXT_STYLES.map((style) => `
              <button type="button" class="tx-style-swatch" data-detail-text-style="${escapeHtml(style.id)}"
                title="${escapeHtml(style.label)}" style="${detailTextSample(style.patch || {})}">Aa</button>`).join('')}
          </div>`;
      }
      return textControls(clip);
    }
    if (tab === 'textanim') {
      const group = TEXT_ANIMATION_GROUPS.find((item) => item.id === sub) || TEXT_ANIMATION_GROUPS[0];
      const active = text.animation || 'none';
      return `
        <button type="button" class="btn btn-small btn-block" data-detail-text-anim="none">${icon('rotate', 13)} Sin animación</button>
        <div class="detail-subhead">${escapeHtml(group.label)}</div>
        <div class="fx-tile-grid">
          ${group.items.map((item) => `
            <button type="button" class="fx-tile${active === item.id ? ' active' : ''}" data-detail-text-anim="${escapeHtml(item.id)}" title="${escapeHtml(item.label)}">
              <span class="fx-thumb anim-none">Aa</span><span>${escapeHtml(item.label)}</span>
            </button>`).join('')}
        </div>
        ${textRangeField('Duración', 'animationDuration', text.animationDuration, 0.1, 3, 0.05, 1, 'text', 's')}`;
    }
    if (tab === 'tracking') {
      const tracking = clip.textTracking || 'none';
      return `
        <div class="field">
          <label>Tipo de seguimiento</label>
          <select class="input" data-detail-tracking>
            <option value="none" ${tracking === 'none' ? 'selected' : ''}>Ninguno</option>
            <option value="direction" ${tracking === 'direction' ? 'selected' : ''}>Dirección</option>
            <option value="object" ${tracking === 'object' ? 'selected' : ''}>Seguir objeto</option>
          </select>
        </div>
        ${tracking !== 'none' ? `
          ${detailRange('Dirección', 'textTrackAngle', Math.round(clip.textTrackAngle || 0), -180, 180, 1, 1, Math.round(clip.textTrackAngle || 0), '°')}
          ${detailRange('Escala', 'textTrackScale', Math.round((clip.textTrackScale ?? 1) * 100), 10, 400, 1, 0.01, Math.round((clip.textTrackScale ?? 1) * 100), '%')}
          ${detailRange('Distancia', 'textTrackDistance', Math.round(clip.textTrackDistance || 0), -100, 100, 1, 1, Math.round(clip.textTrackDistance || 0), '%')}` : ''}
        <div class="detail-hint">El seguimiento automático de CapCut usa la nube. Aquí el texto se coloca y se anima en local con fotogramas clave.</div>`;
    }
    if (tab === 'tts') {
      return `
        <div class="field">
          <label>Texto del clip</label>
          <textarea class="input" rows="4" readonly>${escapeHtml(text.content || '')}</textarea>
        </div>
        <button type="button" class="btn btn-primary btn-block" data-detail-tts-speak>${icon('play', 13)} Reproducir con voz del sistema</button>
        <button type="button" class="btn btn-block" data-detail-tts-open>${icon('mic', 13)} Abrir Texto a voz completo</button>
        <div class="detail-hint">El panel izquierdo «Texto → Texto a voz» genera audio con Piper en local y con las voces instaladas en Windows.</div>`;
    }
    return `
      <div class="detail-hint">Los avatares de IA requieren procesamiento en la nube y no están disponibles en esta versión local. El resto de opciones de texto sí funcionan sin conexión.</div>`;
  }

  function detailTabBody(clip, tab, sub) {
    const isText = clip.kind === 'text';
    if (isText) return textDetailTabBody(clip, tab, sub);
    const isSticker = clip.kind === 'sticker';
    const isVisual = clip.kind === 'video' || clip.kind === 'image';
    const isAudio = clip.kind === 'audio';
    const prefix = isText ? textControls(clip) : isSticker ? stickerControls(clip) : '';
    if (tab === 'video') {
      if (sub === 'mask') return prefix + maskControls(clip);
      if (sub === 'retouch') return prefix + retouchTab(clip);
      if (sub === 'cutout') return prefix + cutoutTab(clip);
      const area = clip.aiRemoveArea || {};
      return `${prefix}
        ${!isText ? transformSection(clip) : ''}
        ${switchRow('Desenfoque de zona', 'aiRemove', !!clip.aiRemove, { note: 'Difumina/borra una zona concreta del clip (funciona en local).' })}
        ${clip.aiRemove ? `
          <div class="detail-subhead">Zona a eliminar</div>
          ${detailRange('Centro X', 'aiRemoveArea.x', Math.round(area.x ?? 0), -50, 50, 1, 1, Math.round(area.x ?? 0), '%')}
          ${detailRange('Centro Y', 'aiRemoveArea.y', Math.round(area.y ?? 0), -50, 50, 1, 1, Math.round(area.y ?? 0), '%')}
          ${detailRange('Ancho', 'aiRemoveArea.w', Math.round(area.w ?? 30), 4, 100, 1, 1, Math.round(area.w ?? 30), '%')}
          ${detailRange('Alto', 'aiRemoveArea.h', Math.round(area.h ?? 30), 4, 100, 1, 1, Math.round(area.h ?? 30), '%')}
          ${detailRange('Difuminado', 'aiRemoveStrength', Math.round(clip.aiRemoveStrength ?? 28), 4, 60, 1, 1, Math.round(clip.aiRemoveStrength ?? 28), 'px')}
          <div class="detail-hint">Coloca la zona sobre el objeto que quieras quitar: se rellena con una versión difuminada del propio fotograma, sin subir nada a la nube.</div>` : ''}
        ${switchRow('Estabilización', 'stabilize', !!clip.stabilize, { note: 'Reduce el temblor de la cámara con análisis de movimiento.' })}
        ${switchRow('Mejorar la calidad', 'enhanceQuality', !!clip.enhanceQuality, { note: 'Escala y mejora la nitidez localmente.' })}
        ${switchRow('Reducir el ruido de la imagen', 'denoise', !!clip.denoise)}
        ${switchRow('Flujo óptico', 'opticalFlow', !!clip.opticalFlow)}
        ${switchRow('Ampliación de IA', 'aiExpand', !!clip.aiExpand)}
        ${switchRow('Remix de IA', 'aiRemix', !!clip.aiRemix)}
        ${switchRow('Contacto visual', 'eyeContact', !!clip.eyeContact)}
        ${switchRow('Sincronización labial', 'lipSync', !!clip.lipSync)}
        ${switchRow('Adición de iluminación', 'lighting', !!clip.lighting)}
        ${switchRow('Automarcado', 'autoFrame', !!clip.autoFrame)}
        ${switchRow('Eliminar los parásitos', 'flickerRemove', !!clip.flickerRemove)}
        ${switchRow('Movimiento de IA', 'aiMotion', !!clip.aiMotion)}
        ${switchRow('Seguimiento de cámara', 'cameraTracking', !!clip.cameraTracking)}
        ${switchRow('Movimiento difuminado', 'blurMotion', !!clip.blurMotion)}
        <div class="detail-subhead">Lienzo<button class="btn btn-small btn-ghost" data-apply-all>Aplicar a todo</button></div>
        <div class="detail-row"><div class="detail-row-main"><span class="detail-row-label">Relación de aspecto</span><b class="detail-value">${state.project?.ratio || '16:9'}</b></div></div>
        <div class="detail-row"><div class="detail-row-main"><span class="detail-row-label">Resolución</span><b class="detail-value">${state.project?.resolution || '1080p'}</b></div></div>`;
    }
    if (tab === 'audio') {
      if (sub === 'voice') return voiceGrid(clip);
      if (!(clip.kind === 'video' || isAudio)) {
        return `<div class="detail-hint">Selecciona un clip con audio para ajustar su volumen.</div>`;
      }
      return audioBasicTab(clip);
    }
    if (tab === 'speed') return speedTab(clip, sub);
    if (tab === 'animation') return animationTab(clip, sub);
    if (tab === 'adjust') {
      if (sub === 'hsl') return hslTab(clip);
      if (sub === 'curves') return curvesTab(clip);
      if (sub === 'wheel') return wheelTab(clip);
      if (sub === 'mask') return maskControls(clip);
      if (!isVisual) return `<div class="detail-hint">Los ajustes de color se aplican a clips de video o imagen.</div>`;
      return adjustBasicTab(clip);
    }
    if (tab === 'ai') return aiTab(clip, sub);
    return '';
  }

  function clipMarkup(found) {
    const { clip } = found;
    const kindLabel = { video: 'Video', image: 'Imagen', audio: 'Audio', text: 'Texto', sticker: 'Sticker' }[clip.kind] || 'Clip';
    const isText = clip.kind === 'text';
    const tabs = isText ? TEXT_DETAIL_TABS : DETAIL_TABS;
    const activeTab = tabs.find((item) => item.id === detailTab) || tabs[0];
    const sub = detailSub[activeTab.id] || activeTab.subs[0][0];
    return `
      <div class="clip-details-head"><i class="details-dot"></i><span>${escapeHtml(clip.name || kindLabel)}</span><span class="clip-chip">${kindLabel}</span></div>
      <div class="detail-tabs">
        ${tabs.map((item) => `<button type="button" class="detail-tab${item.id === activeTab.id ? ' active' : ''}" data-detail-tab="${item.id}">${item.label}</button>`).join('')}
      </div>
      <div class="detail-subtabs">
        ${activeTab.subs.map(([id, label, badge]) => `<button type="button" class="detail-subtab${sub === id ? ' active' : ''}" data-detail-sub="${id}">${label}${badge ? `<i class="detail-badge pro">${badge}</i>` : ''}</button>`).join('')}
      </div>
      <div class="detail-tab-body">${detailTabBody(clip, activeTab.id, sub)}</div>
      ${isText ? '' : `
      <div class="details-sep"></div>
      <div class="detail-subhead">Entrada y transición</div>
      <div class="field">
        <label>Transición al inicio</label>
        <select class="input" data-clip-transition>
          ${TRANSITIONS.map((item) => `<option value="${item.id}" ${item.id === (clip.transition || 'none') ? 'selected' : ''}>${item.label}</option>`).join('')}
        </select>
      </div>
      ${(clip.transition && clip.transition !== 'none') ? numberField('Duración de transición (s)', 'transitionDuration', clip.transitionDuration || 0.6, 0.1, 3, 0.1) : ''}`}
      <div class="detail-meta">Inicio ${formatTime(clip.start)} · Duración ${formatTime(clip.duration)}</div>
      ${isText ? '' : `<button class="btn btn-ghost btn-block" data-reset-clip>${icon('rotate', 14)} Restablecer ajustes</button>`}`;
  }

  function renderCapCutFontPicker(currentFont) {
    const fonts = getFontList(fontActiveTab, fontSearchQuery);
    return `
      <div class="capcut-font-picker-dropdown" id="capcut-font-dropdown">
        <div class="capcut-font-header">
          <div class="capcut-font-search-box">
            <span class="capcut-font-search-icon">🔍</span>
            <input type="text" class="capcut-font-search-input" placeholder="Buscar texto" data-font-search-input value="${escapeHtml(fontSearchQuery)}" autofocus />
            ${fontSearchQuery ? `<button type="button" class="capcut-font-search-clear" data-font-clear-search>✕</button>` : ''}
          </div>
          <div class="capcut-font-nav">
            <button type="button" class="capcut-font-tab ${fontActiveTab === 'all' ? 'active' : ''}" data-font-tab="all">Todos</button>
            <button type="button" class="capcut-font-tab ${fontActiveTab === 'popular' ? 'active' : ''}" data-font-tab="popular">Comercial</button>
            <button type="button" class="capcut-font-tab ${fontActiveTab === 'system' ? 'active' : ''}" data-font-tab="system">Sistema</button>
            <button type="button" class="capcut-font-tab ${fontActiveTab === 'imported' ? 'active' : ''}" data-font-tab="imported">Importadas</button>
            <button type="button" class="capcut-font-tab ${fontActiveTab === 'favorites' ? 'active' : ''}" data-font-tab="favorites">★ Favoritos</button>
            <button type="button" class="capcut-font-tab capcut-font-import-btn" data-trigger-font-import title="Importar archivo .ttf / .otf descargado">+ Importar</button>
          </div>
        </div>

        <div class="capcut-font-list">
          ${fonts.length === 0 ? `
            <div class="capcut-font-empty">No se encontraron fuentes</div>
          ` : fonts.map((font) => {
            const isSelected = (currentFont || 'Inter').toLowerCase() === font.value.toLowerCase();
            const isFav = isFavoriteFont(font.value);
            return `
              <div class="capcut-font-row ${isSelected ? 'selected' : ''}" data-pick-font="${escapeHtml(font.value)}">
                <div class="capcut-font-row-main">
                  <span class="capcut-font-check">${isSelected ? '✓' : ''}</span>
                  <span class="capcut-font-preview" style="font-family: '${escapeHtml(font.value)}', sans-serif;">
                    ${escapeHtml(font.label || font.value)}
                  </span>
                </div>
                <div class="capcut-font-row-actions">
                  <span class="capcut-font-status-icon" title="Lista para usar">📥</span>
                  <button type="button" class="capcut-font-fav-btn ${isFav ? 'active' : ''}" data-fav-font="${escapeHtml(font.value)}" title="${isFav ? 'Quitar de favoritos' : 'Añadir a favoritos'}">
                    ${isFav ? '★' : '☆'}
                  </button>
                </div>
              </div>
            `;
          }).join('')}
        </div>

        <div class="capcut-font-footer">
          <button type="button" class="btn btn-small btn-ghost" data-scan-system-fonts style="font-size:11px;">
            ${icon('refresh', 12)} Detectar fuentes locales de Windows
          </button>
          <button type="button" class="btn btn-small btn-ghost" data-trigger-font-import style="font-size:11px;color:#00e5d0;">
            📁 Cargar .ttf / .otf
          </button>
          <input type="file" id="cupcraft-font-file-input" accept=".ttf,.otf,.woff,.woff2" style="display:none;" />
        </div>
      </div>
    `;
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
          <div class="tx-font-selector-wrap">
            <button type="button" class="tx-font-trigger" data-toggle-font-picker title="Seleccionar fuente">
              <span class="tx-font-trigger-name" style="font-family: '${escapeHtml(text.font || 'Inter')}', sans-serif;">
                ${escapeHtml(text.font || 'Inter')}
              </span>
              <span class="tx-font-trigger-caret">${fontPickerOpen ? '▲' : '▾'}</span>
            </button>
            ${fontPickerOpen ? renderCapCutFontPicker(text.font || 'Inter') : ''}
          </div>
        </div>
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

  /* Lee y escribe propiedades anidadas (mask.feather, beauty.smooth…). */
  function readNested(source, key) {
    return key.split('.').reduce((acc, part) => (acc == null ? acc : acc[part]), source);
  }

  function setNestedPatch(clip, key, value) {
    if (!key.includes('.')) return { [key]: value };
    const parts = key.split('.');
    const root = JSON.parse(JSON.stringify(clip[parts[0]] || {}));
    let node = root;
    for (let index = 1; index < parts.length - 1; index += 1) {
      node[parts[index]] = { ...(node[parts[index]] || {}) };
      node = node[parts[index]];
    }
    node[parts[parts.length - 1]] = value;
    return { [parts[0]]: root };
  }

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
    } else if (key.includes('.')) {
      updateClip(clip.id, setNestedPatch(clip, key, value), { history: false });
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

  /* El pincel se usa sobre el reproductor (preview.js). Desde el panel solo
     se limpian los trazos guardados en el clip seleccionado. */
  details.addEventListener('click', (event) => {
    if (!event.target.closest('[data-draw-clear]')) return;
    const found = selectedClip();
    if (!found) { toast('Selecciona un clip de vídeo', 'info', 1200); return; }
    updateClip(found.clip.id, {
      customCutout: { ...(found.clip.customCutout || {}), strokes: [], applied: false },
    }, { history: true });
    window.dispatchEvent(new CustomEvent('opencup:glfx'));
    toast('Trazos eliminados', 'info', 1200);
    render();
  });

  /* Chroma key: selector de color real y cuentagotas que toma el color
     directamente del fotograma mostrado en el reproductor. */
  function setChromaColor(hex, activate = false) {
    const found = selectedClip();
    if (!found) return;
    const chromaKey = { ...(found.clip.chromaKey || {}), color: hex };
    if (activate) chromaKey.active = true;
    updateClip(found.clip.id, { chromaKey }, { history: true });
    window.dispatchEvent(new CustomEvent('opencup:glfx'));
    render();
  }

  let cancelChromaPick = null;
  details.addEventListener('click', (event) => {
    if (!event.target.closest('[data-chroma-pick]')) return;
    cancelChromaPick?.();
    toast('Haz clic sobre el color del video que quieres quitar', 'info', 3200);
    const onKey = (keyEvent) => {
      if (keyEvent.key === 'Escape') cancel();
    };
    const onPick = (clickEvent) => {
      // El clic sobre el reproductor también deselecciona el clip; en modo
      // cuentagotas debe consumirse para no perder la selección.
      clickEvent.stopPropagation();
      clickEvent.preventDefault();
      const stage = document.querySelector('.stage-wrap');
      const canvas = document.querySelector('.stage-canvas');
      cancel();
      if (!stage || !canvas || !stage.contains(clickEvent.target)) {
        toast('Selección de color cancelada', 'info', 1200);
        return;
      }
      const rect = canvas.getBoundingClientRect();
      if (rect.width < 1 || rect.height < 1) {
        toast('Espera a que el video se muestre en el reproductor', 'info', 1800);
        return;
      }
      const px = Math.max(0, Math.min(canvas.width - 1, Math.round((clickEvent.clientX - rect.left) / rect.width * canvas.width)));
      const py = Math.max(0, Math.min(canvas.height - 1, Math.round((clickEvent.clientY - rect.top) / rect.height * canvas.height)));
      let pixel;
      try { pixel = canvas.getContext('2d').getImageData(px, py, 1, 1).data; }
      catch {
        toast('No se pudo leer el color del video', 'error', 1800);
        return;
      }
      const hex = '#' + [pixel[0], pixel[1], pixel[2]]
        .map((v) => v.toString(16).padStart(2, '0')).join('');
      setChromaColor(hex, true);
      toast(`Color ${hex} aplicado al chroma key`, 'success', 1800);
    };
    const cancel = () => {
      window.removeEventListener('click', onPick, true);
      window.removeEventListener('keydown', onKey);
      document.body.style.cursor = '';
      cancelChromaPick = null;
    };
    cancelChromaPick = cancel;
    document.body.style.cursor = 'crosshair';
    window.addEventListener('keydown', onKey);
    // Se añade en el siguiente tick para que este mismo clic no cuente como pick.
    setTimeout(() => window.addEventListener('click', onPick, true), 0);
  });

  /* Foto del fondo para BackgroundMattingV2. Se guarda como medio local (no
     se añade a la línea de tiempo) y queda referenciada por el clip. */
  details.addEventListener('click', (event) => {
    if (event.target.closest('[data-bg-plate-pick]')) {
      details.querySelector('[data-bg-plate-input]')?.click();
      return;
    }
    if (event.target.closest('[data-bg-color-use]')) {
      const found = selectedClip();
      if (!found) { toast('Selecciona un clip primero', 'info', 1200); return; }
      const color = details.querySelector('[data-bg-color]')?.value || '#00b140';
      updateClip(found.clip.id, {
        autoCutout: true,
        autoCutoutBg: null,
        autoCutoutBgColor: color,
      }, { history: true });
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
      toast(`Recorte IA activado con fondo ${color}`, 'success', 1800);
      render();
    }
  });

  /* Selector de fuentes interactivo estilo CapCut:
     Previsualización con la tipografía real, búsqueda en vivo, favoritos,
     detección de fuentes del sistema e importación de fuentes locales (.ttf / .otf). */
  details.addEventListener('click', (event) => {
    if (event.target.closest('[data-toggle-font-picker]')) {
      fontPickerOpen = !fontPickerOpen;
      render();
      if (fontPickerOpen) {
        // Se vuelve a escanear al abrir: así una fuente instalada mientras la
        // app está abierta aparece sin reiniciar (el escaneo tarda ~150 ms).
        loadSystemFonts({ refresh: true }).then(() => {
          if (fontPickerOpen) render();
        });
      }
      return;
    }
    if (event.target.closest('[data-font-tab]')) {
      fontActiveTab = event.target.closest('[data-font-tab]').dataset.fontTab;
      render();
      return;
    }
    if (event.target.closest('[data-font-clear-search]')) {
      fontSearchQuery = '';
      render();
      return;
    }
    if (event.target.closest('[data-fav-font]')) {
      event.stopPropagation();
      const fontVal = event.target.closest('[data-fav-font]').dataset.favFont;
      toggleFavoriteFont(fontVal);
      render();
      return;
    }
    if (event.target.closest('[data-pick-font]')) {
      const fontVal = event.target.closest('[data-pick-font]').dataset.pickFont;
      fontPickerOpen = false;
      render();
      // Cargar la fuente antes de redibujar garantiza que el lienzo no use
      // la tipografía de reserva en el primer fotograma.
      ensureFontReady(fontVal).then(() => {
        const current = selectedClip();
        if (current) updateClip(current.clip.id, { text: { font: fontVal } });
        render();
      });
      return;
    }
    if (event.target.closest('[data-trigger-font-import]')) {
      details.querySelector('#cupcraft-font-file-input')?.click();
      return;
    }
    if (event.target.closest('[data-scan-system-fonts]')) {
      toast('Detectando fuentes instaladas en Windows...', 'info', 1500);
      loadSystemFonts({ refresh: true }).then((fonts) => {
        toast(fonts.length ? `${fonts.length} fuentes locales detectadas` : 'Fuentes locales cargadas', 'success');
        fontActiveTab = 'system';
        render();
      });
      return;
    }
  });

  details.addEventListener('input', (event) => {
    if (event.target.matches('[data-font-search-input]')) {
      fontSearchQuery = event.target.value;
      const listEl = details.querySelector('.capcut-font-list');
      if (listEl) {
        const found = selectedClip();
        const currentFont = found?.clip?.text?.font || 'Inter';
        const fonts = getFontList(fontActiveTab, fontSearchQuery);
        listEl.innerHTML = fonts.length === 0
          ? `<div class="capcut-font-empty">No se encontraron fuentes</div>`
          : fonts.map((font) => {
            const isSelected = currentFont.toLowerCase() === font.value.toLowerCase();
            const isFav = isFavoriteFont(font.value);
            return `
              <div class="capcut-font-row ${isSelected ? 'selected' : ''}" data-pick-font="${escapeHtml(font.value)}">
                <div class="capcut-font-row-main">
                  <span class="capcut-font-check">${isSelected ? '✓' : ''}</span>
                  <span class="capcut-font-preview" style="font-family: '${escapeHtml(font.value)}', sans-serif;">
                    ${escapeHtml(font.label || font.value)}
                  </span>
                </div>
                <div class="capcut-font-row-actions">
                  <span class="capcut-font-status-icon" title="Lista para usar">📥</span>
                  <button type="button" class="capcut-font-fav-btn ${isFav ? 'active' : ''}" data-fav-font="${escapeHtml(font.value)}" title="${isFav ? 'Quitar de favoritos' : 'Añadir a favoritos'}">
                    ${isFav ? '★' : '☆'}
                  </button>
                </div>
              </div>
            `;
          }).join('');
      }
    }
  });

  details.addEventListener('change', async (event) => {
    if (event.target.id === 'cupcraft-font-file-input') {
      const file = event.target.files?.[0];
      if (file) {
        toast(`Importando fuente "${file.name}"...`, 'info', 1500);
        importCustomFont(file).then((entry) => {
          toast(`¡Fuente "${entry.label}" cargada y lista para usar!`, 'success', 2000);
          fontActiveTab = 'imported';
          const found = selectedClip();
          if (found) {
            updateClip(found.clip.id, { text: { font: entry.value } });
          }
          render();
        }).catch((err) => {
          toast(`Error al importar la fuente: ${err.message}`, 'error');
        });
      }
      return;
    }
    if (event.target.matches('[data-chroma-color]')) {
      setChromaColor(event.target.value);
      return;
    }
    if (event.target.matches('[data-bg-color]')) {
      const found = selectedClip();
      if (!found) return;
      const readout = details.querySelector('[data-bg-color-readout]');
      if (readout) readout.textContent = event.target.value;
      updateClip(found.clip.id, { autoCutoutBgColor: event.target.value }, { history: true });
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
      return;
    }
    if (!event.target.matches('[data-bg-plate-input]')) return;
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) return;
    const found = selectedClip();
    if (!found) { toast('Selecciona un clip primero', 'info', 1200); return; }
    try {
      const meta = await importExternalMedia(file, { silent: true });
      updateClip(found.clip.id, { autoCutout: true, autoCutoutBg: meta.id, autoCutoutBgColor: null }, { history: true });
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
      toast('Fondo cargado: recorte IA activado', 'success', 1800);
    } catch (error) {
      toast(error?.message || 'No se pudo cargar la foto del fondo', 'error', 3200);
    }
    render();
  });

  host.addEventListener('click', (event) => {
    if (event.target.closest('[data-edit-project]')) { openProjectEditor(); return; }

    const detailTabButton = event.target.closest('[data-detail-tab]');
    if (detailTabButton) {
      detailTab = detailTabButton.dataset.detailTab;
      render();
      return;
    }
    const detailSubButton = event.target.closest('[data-detail-sub]');
    if (detailSubButton) {
      detailSub[detailTab] = detailSubButton.dataset.detailSub;
      render();
      return;
    }
    const subJump = event.target.closest('[data-detail-sub-jump]');
    if (subJump) {
      detailSub[detailTab] = subJump.dataset.detailSubJump;
      render();
      return;
    }

    /* Detalles de texto: burbujas, efectos, estilos y animaciones. */
    const detailBubble = event.target.closest('[data-detail-bubble]');
    if (detailBubble) {
      const found = selectedClip();
      const bubble = TEXT_BUBBLES.find((item) => item.id === detailBubble.dataset.detailBubble);
      if (found && bubble) {
        updateClip(found.clip.id, { text: { ...bubble.patch } });
        toast(`Burbuja “${bubble.label}” aplicada`, 'success', 1400);
        render();
      }
      return;
    }
    const detailEffect = event.target.closest('[data-detail-text-effect]');
    if (detailEffect) {
      const found = selectedClip();
      const effect = TEXT_EFFECTS.find((item) => item.id === detailEffect.dataset.detailTextEffect);
      if (found && effect) {
        updateClip(found.clip.id, { text: { ...effect.patch } });
        toast(`Efecto “${effect.label}” aplicado`, 'success', 1400);
        render();
      }
      return;
    }
    const detailStyle = event.target.closest('[data-detail-text-style]');
    if (detailStyle) {
      const found = selectedClip();
      const style = TEXT_STYLES.find((item) => item.id === detailStyle.dataset.detailTextStyle);
      if (found && style) {
        updateClip(found.clip.id, { text: { ...(style.patch || {}) } });
        toast(`Estilo “${style.label}” aplicado`, 'success', 1400);
        render();
      }
      return;
    }
    const detailAnim = event.target.closest('[data-detail-text-anim]');
    if (detailAnim) {
      const found = selectedClip();
      if (found) {
        const id = detailAnim.dataset.detailTextAnim;
        const item = TEXT_ANIMATIONS.find((animation) => animation.id === id);
        updateClip(found.clip.id, {
          text: {
            animation: id,
            animationPhase: item?.phase || 'in',
            animationDuration: item?.duration || found.clip.text?.animationDuration || 0.6,
          },
        });
        toast(id === 'none' ? 'Animación eliminada' : 'Animación de texto aplicada', 'success', 1400);
        render();
      }
      return;
    }
    if (event.target.closest('[data-detail-tts-speak]')) {
      const found = selectedClip();
      const content = found?.clip.text?.content || '';
      if (!content) { toast('El clip no tiene texto', 'info', 1400); return; }
      if (!('speechSynthesis' in window)) { toast('Tu navegador no tiene voz del sistema', 'error', 1800); return; }
      window.speechSynthesis.cancel();
      const utterance = new SpeechSynthesisUtterance(content);
      utterance.lang = 'es-ES';
      window.speechSynthesis.speak(utterance);
      return;
    }
    if (event.target.closest('[data-detail-tts-open]')) {
      const found = selectedClip();
      state.panel = 'text';
      window.dispatchEvent(new CustomEvent('opencup:open-tts', {
        detail: { text: found?.clip.text?.content || '' },
      }));
      return;
    }
    const maskShape = event.target.closest('[data-mask-shape]');
    if (maskShape) {
      const found = selectedClip();
      if (!found) return;
      const shape = maskShape.dataset.maskShape;
      updateClip(found.clip.id, { mask: { ...(found.clip.mask || {}), shape } });
      render();
      return;
    }
    if (event.target.closest('[data-mask-remove]')) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { mask: { ...(found.clip.mask || {}), shape: 'none' } });
      render();
      return;
    }
    const clipSwitch = event.target.closest('[data-clip-switch]');
    if (clipSwitch) {
      const found = selectedClip();
      if (!found) return;
      const key = clipSwitch.dataset.clipSwitch;
      const current = !!readNested(found.clip, key);
      // La IA no puede recortar sin la foto del fondo (background plate).
      // Si no hay una, el switch abre el selector en vez de quedarse encendido
      // sin efecto, que es lo que hacía creer que la herramienta no funcionaba.
      if (key === 'autoCutout' && !current && !found.clip.autoCutoutBg && !found.clip.autoCutoutBgColor) {
        details.querySelector('[data-bg-plate-input]')?.click();
        toast('Elige la foto del fondo (sin la persona) para activar el recorte IA', 'info', 4200);
        return;
      }
      updateClip(found.clip.id, setNestedPatch(found.clip, key, !current));
      render();
      return;
    }
    const aiLocked = event.target.closest('[data-ai-locked]');
    if (aiLocked) {
      unavailable('Función en la nube', `“${aiLocked.dataset.aiLocked}” requiere procesamiento en la nube y no está disponible sin conexión. El resto de ajustes del clip siguen funcionando en local.`);
      return;
    }
    const voiceButton = event.target.closest('[data-voice]');
    if (voiceButton) {
      const found = selectedClip();
      if (!found) return;
      const voice = voiceButton.dataset.voice;
      updateClip(found.clip.id, { voice: voice === 'Original' ? null : voice });
      render();
      return;
    }
    const speedCurveButton = event.target.closest('[data-speed-curve]');
    if (speedCurveButton) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { speedCurve: speedCurveButton.dataset.speedCurve });
      render();
      return;
    }
    const speedFxButton = event.target.closest('[data-speed-fx]');
    if (speedFxButton) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { speedFx: speedFxButton.dataset.speedFx });
      render();
      return;
    }
    const animButton = event.target.closest('[data-anim-kind]');
    if (animButton) {
      const found = selectedClip();
      if (!found) return;
      const kind = animButton.dataset.animKind;
      const prop = kind === 'in' ? 'animIn' : kind === 'out' ? 'animOut' : 'animCombo';
      const name = animButton.dataset.animEffect || '';
      if (!name) {
        updateClip(found.clip.id, { [prop]: null });
        render();
        return;
      }
      const mapped = capcutToLocal(name);
      const glId = capcutEffectToGL(name);
      state.glEffect = glId;
      state.glAmount = 1;
      updateClip(found.clip.id, {
        effect: glId ? 'none' : mapped,
        glEffect: glId || null,
        [prop]: name,
      });
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
      seek(state.playhead);
      render();
      return;
    }
    const hslButton = event.target.closest('[data-hsl-color]');
    if (hslButton) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { hsl: { ...(found.clip.hsl || {}), color: hslButton.dataset.hslColor } });
      render();
      return;
    }
    const curveChannelButton = event.target.closest('[data-curve-channel]');
    if (curveChannelButton) {
      const found = selectedClip();
      if (!found) return;
      updateClip(found.clip.id, { curveChannel: curveChannelButton.dataset.curveChannel });
      render();
      return;
    }
    const wheelButton = event.target.closest('[data-wheel]');
    if (wheelButton) {
      const found = selectedClip();
      if (!found) return;
      const rect = wheelButton.getBoundingClientRect();
      const x = Math.max(-1, Math.min(1, ((event.clientX - rect.left) / rect.width) * 2 - 1));
      const y = Math.max(-1, Math.min(1, ((event.clientY - rect.top) / rect.height) * 2 - 1));
      const id = wheelButton.dataset.wheel;
      const wheels = { ...(found.clip.wheels || {}) };
      wheels[id] = { ...(wheels[id] || {}), x: Math.round(x * 100) / 100, y: Math.round(y * 100) / 100 };
      updateClip(found.clip.id, { wheels });
      render();
      return;
    }
    const curvePoint = event.target.closest('[data-curve-point]');
    if (curvePoint) {
      // El clic sobre un punto no crea otro punto (el arrastre y el doble
      // clic se gestionan con los eventos pointer/dblclick).
      curveDraggedAt = 0;
      return;
    }
    const curveBox = event.target.closest('[data-curve-box]');
    if (curveBox) {
      // Si venimos de arrastrar un punto, el clic de finalización no debe
      // añadir un punto nuevo en la curva.
      if (curveDraggedAt && Date.now() - curveDraggedAt < 500) {
        curveDraggedAt = 0;
        return;
      }
      const found = selectedClip();
      if (!found) return;
      const rect = curveBox.getBoundingClientRect();
      const x = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
      const y = Math.max(0, Math.min(1, 1 - (event.clientY - rect.top) / rect.height));
      const channel = curveBox.dataset.curveBoxChannel || 'luma';
      const curves = JSON.parse(JSON.stringify(found.clip.curves || {}));
      const points = curves[channel] || [[0, 0], [1, 1]];
      points.push([Math.round(x * 100) / 100, Math.round(y * 100) / 100]);
      points.sort((a, b) => a[0] - b[0]);
      curves[channel] = points;
      updateClip(found.clip.id, { curves });
      render();
      return;
    }
    if (event.target.closest('[data-apply-all]')) {
      const found = selectedClip();
      if (!found) return;
      const { adjust, mask, beauty, aiStyle, filter, effect, glFilter, glEffect, hsl, curves } = found.clip;
      const patch = {
        ...(adjust ? { adjust: { ...adjust } } : {}),
        ...(mask ? { mask: { ...mask } } : {}),
        ...(beauty ? { beauty: { ...beauty } } : {}),
        ...(aiStyle ? { aiStyle: JSON.parse(JSON.stringify(aiStyle)) } : {}),
        ...(hsl ? { hsl: { ...hsl } } : {}),
        ...(curves ? { curves: JSON.parse(JSON.stringify(curves)) } : {}),
        ...(filter ? { filter } : {}),
        ...(effect ? { effect } : {}),
        ...(glFilter ? { glFilter } : {}),
        ...(glEffect ? { glEffect } : {}),
      };
      const entries = [];
      for (const track of state.project?.tracks || []) {
        for (const clip of track.clips) {
          if (clip.kind !== 'video' && clip.kind !== 'image') continue;
          if (clip.id === found.clip.id) continue;
          entries.push({ clipId: clip.id, patch });
        }
      }
      if (!entries.length) {
        toast('No hay otros clips visuales en el proyecto', 'info', 1800);
        return;
      }
      updateClipsMany(entries);
      toast('Ajustes aplicados a todos los clips visuales', 'success', 1900);
      render();
      return;
    }
    if (event.target.closest('[data-cutout-apply]')) {
      const found = selectedClip();
      if (!found) return;
      const strokes = found.clip.customCutout?.strokes || [];
      if (!strokes.length) {
        toast('Pinta primero sobre el vídeo del reproductor', 'info', 1800);
        return;
      }
      updateClip(found.clip.id, {
        customCutout: { ...found.clip.customCutout, active: true, applied: true },
      }, { history: true });
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
      toast(`${strokes.length} zona(s) borrada(s) y fijada(s)`, 'success', 1800);
      render();
      return;
    }
    if (event.target.closest('[data-clip-duplicate]')) { duplicateSelected(); return; }
    if (event.target.closest('[data-export-video]')) { openExportDialog(); return; }
    if (event.target.closest('[data-details-help]')) { openShortcutsDialog(); return; }
    const cutoutStroke = event.target.closest('[data-cutout-stroke]');
    if (cutoutStroke) {
      const found = selectedClip();
      if (!found) return;
      const type = cutoutStroke.dataset.cutoutStroke;
      updateClip(found.clip.id, { cutoutStroke: { ...(found.clip.cutoutStroke || {}), type } });
      render();
      return;
    }
    const cutoutTool = event.target.closest('[data-cutout-tool]');
    if (cutoutTool) {
      const found = selectedClip();
      if (!found) return;
      const tool = cutoutTool.dataset.cutoutTool;
      updateClip(found.clip.id, { customCutout: { ...(found.clip.customCutout || {}), tool, active: true, applied: false } });
      toast(`Herramienta: ${tool} — pinta sobre el vídeo`, 'info', 1400);
      render();
      return;
    }
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

  /* Arrastre de los puntos de control de la curva de color. */
  host.addEventListener('pointerdown', (event) => {
    const dot = event.target.closest('[data-curve-point]');
    if (!dot) return;
    const box = dot.closest('[data-curve-box]');
    if (!box) return;
    curveDrag = {
      index: Number(dot.dataset.curvePoint),
      channel: box.dataset.curveBoxChannel || 'luma',
      box,
      dot,
    };
    curveDraggedAt = 0;
    try { dot.setPointerCapture(event.pointerId); } catch { /* ignorado */ }
    event.preventDefault();
  });

  host.addEventListener('pointermove', (event) => {
    if (!curveDrag) return;
    const found = selectedClip();
    if (!found) return;
    const rect = curveDrag.box.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    const y = Math.max(0, Math.min(1, 1 - (event.clientY - rect.top) / rect.height));
    const curves = JSON.parse(JSON.stringify(found.clip.curves || {}));
    const points = curves[curveDrag.channel] || [];
    if (!points[curveDrag.index]) return;
    points[curveDrag.index] = [Math.round(x * 100) / 100, Math.round(y * 100) / 100];
    curves[curveDrag.channel] = points;
    curveDraggedAt = Date.now();
    suppressRender = true;
    updateClip(found.clip.id, { curves }, { history: false });
    suppressRender = false;
    // Actualización directa del DOM para que el arrastre sea fluido.
    curveDrag.dot.style.left = `${x * 100}%`;
    curveDrag.dot.style.bottom = `${y * 100}%`;
    const path = curveDrag.box.querySelector('.curve-line path');
    if (path) path.setAttribute('d', curvePathFromPoints(points));
  });

  host.addEventListener('pointerup', () => {
    if (!curveDrag) return;
    const { channel } = curveDrag;
    const moved = curveDraggedAt;
    curveDrag = null;
    if (!moved) return;
    const found = selectedClip();
    if (!found) return;
    // Al soltar, ordenamos por X para que la curva no se cruce.
    const curves = JSON.parse(JSON.stringify(found.clip.curves || {}));
    const points = curves[channel];
    if (!points) return;
    points.sort((a, b) => a[0] - b[0]);
    curves[channel] = points;
    updateClip(found.clip.id, { curves }, { history: false });
    render();
  });

  /* Doble clic sobre un punto de la curva: quitar el punto. */
  host.addEventListener('dblclick', (event) => {
    const dot = event.target.closest('[data-curve-point]');
    if (!dot) return;
    const found = selectedClip();
    if (!found) return;
    const box = dot.closest('[data-curve-box]');
    const channel = box?.dataset.curveBoxChannel || found.clip.curveChannel || 'luma';
    const curves = JSON.parse(JSON.stringify(found.clip.curves || {}));
    const points = curves[channel] || [];
    const index = Number(dot.dataset.curvePoint);
    if (points.length <= 2 || !Number.isFinite(index) || index < 0 || index >= points.length) {
      toast('La curva necesita al menos dos puntos', 'info', 1600);
      return;
    }
    points.splice(index, 1);
    curves[channel] = points;
    updateClip(found.clip.id, { curves });
    curveDraggedAt = 0;
    render();
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
    if (target.matches('[data-clip-db]')) {
      const found = selectedClip();
      if (!found) return;
      const db = Number(target.value);
      const volume = Math.max(0, Math.min(2, Math.pow(10, db / 20)));
      suppressRender = true;
      updateClip(found.clip.id, { volume }, { history: false });
      suppressRender = false;
      const readout = details.querySelector('[data-db-readout]');
      if (readout) readout.textContent = `${db === 0 ? '0.0' : db.toFixed(1)}dB`;
      return;
    }
    if (target.matches('[data-prop]')) applyRange(target);
  });

  details.addEventListener('change', (event) => {
    const target = event.target;
    const found = selectedClip();
    if (!found) return;
    if (target.matches('[data-nested-prop]')) {
      updateClip(found.clip.id, setNestedPatch(found.clip, target.dataset.nestedProp, target.value));
      render();
      return;
    }
    if (target.matches('[data-detail-tracking]')) {
      updateClip(found.clip.id, { textTracking: target.value });
      render();
      return;
    }
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
      const fades = TRANSITION_FADES[type] || { fadeIn: 0, fadeOut: 0 };
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
