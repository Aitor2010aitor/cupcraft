import {
  state,
  on,
  initStore,
  newProject,
  importFiles,
  saveNow,
  togglePlay,
  stepFrame,
  skipToEdge,
  splitAtPlayhead,
  duplicateSelected,
  deleteSelected,
  undo,
  redo,
  selectClip,
  selectedClip,
  updateClip,
} from './store.js';
import { mountHome } from './home.js';
import { mountEditor } from './editor.js';
import { icon } from './icons.js';
import { isEditableTarget, toast } from './utils.js';
import { installAiBridgeApi, startAiBridge } from './ai-bridge.js';
import { installLogCapture } from './log.js';

const root = document.getElementById('app');
let active = null;

function render() {
  try {
    active?.destroy?.();
  } catch (error) {
    console.error(error);
  }
  active = null;
  root.innerHTML = '';
  if (state.view === 'editor' && state.project) {
    try {
      active = mountEditor(root);
    } catch (error) {
      console.error(error);
      root.innerHTML = `
        <div style="height:100vh;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:12px;color:#9a9aa3">
          ${icon('alert', 34)}
          <strong style="color:#f2f2f4">No se pudo abrir el editor</strong>
          <span style="font-size:12px">${error?.message || 'Error inesperado'}</span>
          <button class="btn btn-primary" onclick="location.reload()">Recargar cupcraft</button>
        </div>`;
    }
  } else {
    active = mountHome(root);
  }
}

on('view', render);

function pickMedia() {
  const input = document.createElement('input');
  input.type = 'file';
  input.multiple = true;
  input.accept = 'video/*,image/*,audio/*,.mkv,.mov,.m4v,.webm,.mp3,.wav,.m4a,.aac,.ogg';
  input.addEventListener('change', async () => {
    const files = Array.from(input.files || []);
    input.remove();
    if (files.length) await importFiles(files);
  });
  input.click();
}

window.addEventListener('keydown', (event) => {
  if (document.querySelector('.modal-backdrop')) return;
  if (isEditableTarget(event.target)) return;
  const mod = event.ctrlKey || event.metaKey;
  const key = event.key;
  const lower = key.toLowerCase();

  if (mod && lower === 's') {
    event.preventDefault();
    saveNow().then(() => toast('Proyecto guardado', 'success', 1600));
    return;
  }

  if (state.view !== 'editor' || !state.project) {
    if (mod && lower === 'i') {
      event.preventDefault();
      pickMedia();
    }
    return;
  }

  if (mod && lower === 'z') {
    event.preventDefault();
    if (event.shiftKey) redo(); else undo();
  } else if (mod && lower === 'y') {
    event.preventDefault();
    redo();
  } else if (mod && lower === 'd') {
    event.preventDefault();
    duplicateSelected();
  } else if (mod && lower === 'i') {
    event.preventDefault();
    pickMedia();
  } else if (key === ' ') {
    event.preventDefault();
    togglePlay();
  } else if (key === 'Delete' || key === 'Backspace') {
    event.preventDefault();
    deleteSelected();
  } else if (lower === 'b') {
    event.preventDefault();
    splitAtPlayhead();
  } else if (event.altKey && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(key)) {
    const found = selectedClip();
    if (found && ['text', 'sticker'].includes(found.clip.kind)) {
      event.preventDefault();
      const step = event.shiftKey ? 5 : 1;
      const patch = {};
      if (key === 'ArrowLeft') patch.x = (found.clip.x || 0) - step;
      if (key === 'ArrowRight') patch.x = (found.clip.x || 0) + step;
      if (key === 'ArrowUp') patch.y = (found.clip.y || 0) - step;
      if (key === 'ArrowDown') patch.y = (found.clip.y || 0) + step;
      updateClip(found.clip.id, patch);
    }
  } else if (key === 'ArrowLeft') {
    event.preventDefault();
    stepFrame(-1);
  } else if (key === 'ArrowRight') {
    event.preventDefault();
    stepFrame(1);
  } else if (key === 'Home') {
    event.preventDefault();
    skipToEdge('start');
  } else if (key === 'End') {
    event.preventDefault();
    skipToEdge('end');
  } else if (key === 'Escape') {
    selectClip(null);
  }
});

const overlay = document.createElement('div');
overlay.className = 'drop-overlay hidden';
overlay.innerHTML = `
  <div class="drop-card">
    ${icon('import', 30)}
    <strong>Suelta para importar</strong>
    <span>Videos, imágenes y audio se añadirán a tu proyecto</span>
  </div>`;
document.body.append(overlay);

let dragDepth = 0;
const hasFiles = (event) => Array.from(event.dataTransfer?.types || []).includes('Files');
const isDropZone = (target) => target instanceof Element && !!target.closest('.left-panel, .timeline, .modal-backdrop, .drop-card');

function showOverlay(show) {
  overlay.classList.toggle('hidden', !show);
}

window.addEventListener('dragenter', (event) => {
  if (!hasFiles(event) || isDropZone(event.target)) return;
  dragDepth += 1;
  showOverlay(true);
});

window.addEventListener('dragover', (event) => {
  if (!hasFiles(event)) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
  if (!isDropZone(event.target)) showOverlay(true);
  else showOverlay(false);
});

window.addEventListener('dragleave', (event) => {
  if (!hasFiles(event)) return;
  dragDepth = Math.max(0, dragDepth - 1);
  if (!dragDepth) showOverlay(false);
});

window.addEventListener('drop', async (event) => {
  if (!hasFiles(event)) return;
  dragDepth = 0;
  showOverlay(false);
  if (event.defaultPrevented) return;
  event.preventDefault();
  const files = Array.from(event.dataTransfer.files || []);
  if (!files.length) return;
  if (state.view === 'editor' && state.project) {
    await importFiles(files);
    return;
  }
  const base = (files[0].name || 'Proyecto importado').replace(/\.[^.]+$/, '').slice(0, 60);
  await newProject(base || 'Proyecto importado');
  await importFiles(files);
});

window.addEventListener('beforeunload', (event) => {
  if (state.project && state.saving) {
    event.preventDefault();
    event.returnValue = '';
  }
});

window.addEventListener('pagehide', () => {
  if (state.project) saveNow();
});

installLogCapture();
installAiBridgeApi();

async function boot() {
  try {
    await initStore();
  } catch (error) {
    console.error(error);
    state.booted = true;
    toast('No se pudo acceder al almacenamiento local. Los proyectos no se guardarán.', 'error', 7000);
  }
  if (!root.childElementCount) render();
  startAiBridge();
}

boot();
