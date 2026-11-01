import {
  state,
  on,
  timelineLength,
  projectDuration,
  trackById,
  findClip,
  selectClip,
  addMediaToTimeline,
  addTextClip,
  addTrack,
  moveClip,
  trimClip,
  splitAtPlayhead,
  deleteSelected,
  duplicateSelected,
  undo,
  redo,
  seek,
  pause,
  setZoom,
  toggleSnap,
  importFiles,
  updateProject,
  formatTime,
  setPanel,
} from './store.js';
import { icon } from './icons.js';
import { escapeHtml, clamp, toast } from './utils.js';

const TRACK_META = {
  text: { hint: 'Añade texto desde la pestaña Texto', icon: 'text' },
  video: { hint: 'Arrastra tus videos e imágenes aquí', icon: 'film' },
  audio: { hint: 'Arrastra música o audio aquí', icon: 'music' },
  overlay: { hint: 'Capa extra: stickers, texto o efectos', icon: 'layers' },
};

const escapeSelector = (value) => String(value).replace(/["\\]/g, '\\$&');

export function mountTimeline(host) {
  host.classList.add('timeline');
  host.innerHTML = `
    <div class="timeline-toolbar">
      <div class="tl-tools">
        <button class="tl-btn tl-add" data-act="import" title="Añadir medios">${icon('plus', 17)}</button>
        <span class="tl-sep"></span>
        <button class="tl-btn tl-select" data-act="select" title="Herramienta de selección">${icon('cursor', 15)}${icon('chevronDown', 10)}</button>
        <span class="tl-sep"></span>
        <button class="tl-btn" data-act="undo" title="Deshacer (Ctrl+Z)" disabled>${icon('undo', 16)}</button>
        <button class="tl-btn" data-act="redo" title="Rehacer (Ctrl+Y)" disabled>${icon('redo', 16)}</button>
        <button class="tl-btn" data-act="split" title="Dividir en el cursor (B)">${icon('split', 16)}</button>
        <button class="tl-btn" data-act="duplicate" title="Duplicar (Ctrl+D)">${icon('copy', 15)}</button>
        <button class="tl-btn" data-act="delete" title="Eliminar (Supr)">${icon('trash', 15)}</button>
        <button class="tl-btn" data-act="shield" title="Guardado automático activado">${icon('shield', 15)}</button>
      </div>
      <div class="tl-spacer"></div>
      <div class="tl-tools tl-right">
        <button class="tl-btn" data-act="mic" title="Grabar voz">${icon('mic', 15)}</button>
        <span class="tl-sep"></span>
        <button class="tl-btn tl-cyan ${state.snap ? 'active' : ''}" data-act="snap" title="Imán de ajuste">${icon('magnet', 15)}</button>
        <button class="tl-btn tl-cyan" data-act="link" title="Sincronizar pistas">${icon('link', 15)}</button>
        <button class="tl-btn tl-cyan" data-act="auto" title="Ajuste automático">${icon('sparkles', 15)}</button>
        <span class="tl-sep"></span>
        <button class="tl-btn" data-act="fit" title="Ajustar a la ventana">${icon('fit', 15)}</button>
        <button class="tl-btn" data-act="zoomOut" title="Alejar">${icon('zoomOut', 16)}</button>
        <input class="tl-zoom" type="range" min="20" max="320" step="1" data-zoom-range aria-label="Zoom de la línea de tiempo" />
        <button class="tl-btn" data-act="zoomIn" title="Acercar">${icon('zoomIn', 16)}</button>
      </div>
    </div>
    <div class="timeline-main">
      <div class="track-labels" data-labels></div>
      <div class="timeline-scroll" data-scroll>
        <div class="timeline-content" data-content>
          <div class="ruler" data-ruler></div>
          <div class="tracks" data-tracks></div>
          <div class="snap-guide hidden" data-guide></div>
          <div class="scrub-tooltip hidden" data-tooltip></div>
          <div class="playhead" data-playhead><div class="playhead-head"></div></div>
        </div>
      </div>
      <div class="timeline-empty" data-empty>
        <span class="tl-empty-icon">${icon('film', 13)}</span>
        <span>Arrastra el material aquí y empieza a crear</span>
      </div>
    </div>
    <input type="file" class="hidden" multiple data-file-input />`;

  const scroll = host.querySelector('[data-scroll]');
  const content = host.querySelector('[data-content]');
  const ruler = host.querySelector('[data-ruler]');
  const labels = host.querySelector('[data-labels]');
  const tracksEl = host.querySelector('[data-tracks]');
  const playhead = host.querySelector('[data-playhead]');
  const guide = host.querySelector('[data-guide]');
  const tooltip = host.querySelector('[data-tooltip]');
  const emptyHint = host.querySelector('[data-empty]');
  const fileInput = host.querySelector('[data-file-input]');
  const toolbar = host.querySelector('.timeline-toolbar');
  const undoBtn = host.querySelector('[data-act="undo"]');
  const redoBtn = host.querySelector('[data-act="redo"]');
  const splitBtn = host.querySelector('[data-act="split"]');
  const duplicateBtn = host.querySelector('[data-act="duplicate"]');
  const deleteBtn = host.querySelector('[data-act="delete"]');
  const snapBtn = host.querySelector('[data-act="snap"]');
  const zoomRange = host.querySelector('[data-zoom-range]');
  const subscriptions = [];
  let destroyed = false;
  let lastPlayheadTime = -1;

  const pxPerSecond = () => state.zoom;
  const fps = () => state.project?.fps || 30;

  function timeFromX(clientX) {
    const rect = content.getBoundingClientRect();
    return Math.max(0, (clientX - rect.left) / pxPerSecond());
  }

  function rowAt(clientY) {
    for (const row of tracksEl.querySelectorAll('.track-row')) {
      const rect = row.getBoundingClientRect();
      if (clientY >= rect.top && clientY <= rect.bottom) return row;
    }
    return null;
  }

  function timecode(seconds) {
    return formatTime(seconds, { fps: fps(), frames: true });
  }

  function renderRuler(length, px) {
    const width = Math.max(length * px, scroll.clientWidth);
    const steps = [0.2, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600];
    const target = 84 / px;
    const major = steps.find((step) => step >= target) || 600;
    const minor = major / 2;
    const marks = [];
    for (let time = 0; time <= length + 0.001; time += minor) {
      const isMajor = Math.abs(time / major - Math.round(time / major)) < 0.0001;
      const left = time * px;
      if (isMajor) {
        const label = major < 1
          ? `${Math.floor(time / 60)}:${String(Math.floor(time % 60)).padStart(2, '0')}.${Math.round((time % 1) * 10)}`
          : formatTime(time);
        marks.push(`<div class="ruler-tick" style="left:${left}px"><span>${label}</span></div>`);
      } else {
        marks.push(`<div class="ruler-tick minor" style="left:${left}px"></div>`);
      }
    }
    ruler.innerHTML = marks.join('');
    return width;
  }

  function clipMarkup(clip, px) {
    const selected = state.selection === clip.id;
    const width = Math.max(8, clip.duration * px);
    const media = clip.mediaId ? state.media.get(clip.mediaId) : null;
    const thumb = media?.thumb;
    const thumbStyle = thumb ? ` style="background-image:url('${thumb}')"` : '';
    const wave = clip.kind === 'audio' ? '<canvas class="clip-wave"></canvas>' : '';
    // Rombos de fotograma clave (unión de instantes de todas las propiedades animadas).
    const kfTimes = [...new Set(
      Object.values(clip.keyframes || {}).flatMap((list) => (Array.isArray(list) ? list.map((point) => point.t) : [])),
    )].sort((a, b) => a - b);
    const diamonds = kfTimes.map((time) => `<i class="kf-diamond" style="left:${(time - clip.start) * px}px" title="Fotograma clave ${formatTime(time)}"></i>`).join('');
    const textPreview = clip.kind === 'text'
      ? `<div class="clip-text-preview">${escapeHtml(clip.text?.content || '')}</div>`
      : clip.kind === 'sticker'
        ? `<div class="clip-text-preview sticker-preview">${escapeHtml(clip.sticker?.content || clip.name || '')}</div>`
        : '';
    return `
      <div class="clip ${clip.kind}${selected ? ' selected' : ''}" data-clip-id="${clip.id}" style="left:${clip.start * px}px;width:${width}px">
        ${thumb ? `<div class="clip-thumb"${thumbStyle}></div>` : ''}
        <div class="clip-shade"></div>
        ${diamonds}
        ${wave}
        ${textPreview}
        ${clip.kind !== 'text' && clip.kind !== 'sticker' ? `<div class="clip-label">${escapeHtml(clip.name || 'Clip')}</div>` : ''}
        ${clip.kind !== 'text' && clip.kind !== 'sticker' ? `<div class="clip-meta">${formatTime(clip.duration)}</div>` : ''}
        ${clip.effect && clip.effect !== 'none' ? `<div class="clip-fx" title="Efecto: ${escapeHtml(clip.effect)}">fx</div>` : (clip.glEffect ? `<div class="clip-fx" title="Shader: ${escapeHtml(clip.glEffect)}">fx</div>` : '')}
        <div class="clip-handle left" data-edge="left"></div>
        <div class="clip-handle right" data-edge="right"></div>
      </div>`;
  }

  function renderLabels() {
    if (!state.project) return;
    labels.innerHTML = `
      <div class="track-label-spacer"></div>
      ${state.project.tracks.map((track) => {
        const meta = TRACK_META[track.type] || TRACK_META.video;
        const showMute = track.type === 'audio';
        return `
          <div class="track-label" data-track-id="${track.id}">
            <span class="t-icon">${icon(meta.icon, 13)}</span>
            <span class="t-name">${escapeHtml(track.name)}</span>
            <span class="t-actions">
              <button class="icon-btn sm ${(showMute ? track.muted : track.hidden) ? 'active' : ''}" data-track-toggle="${showMute ? 'muted' : 'hidden'}" title="${showMute ? 'Silenciar pista' : 'Ocultar pista'}">
                ${icon(showMute ? (track.muted ? 'mute' : 'volume') : (track.hidden ? 'eye' : 'eye'), 13)}
              </button>
              <button class="icon-btn sm ${track.locked ? 'active' : ''}" data-track-toggle="locked" title="Bloquear pista">${icon('lock', 12)}</button>
            </span>
          </div>`;
      }).join('')}
      <div class="track-add-row" title="Añadir pista/capa">
        <span class="t-add-icon">${icon('plus', 13)}</span>
        <button class="t-add-type" data-add-track-type="video" title="Añadir pista de vídeo">${icon('film', 12)}</button>
        <button class="t-add-type" data-add-track-type="text" title="Añadir pista de texto">${icon('text', 12)}</button>
        <button class="t-add-type" data-add-track-type="audio" title="Añadir pista de audio">${icon('music', 12)}</button>
        <button class="t-add-type" data-add-track-type="overlay" title="Añadir pista de superposición">${icon('layers', 12)}</button>
      </div>`;
  }

  function renderTracks(px) {
    if (!state.project) return;
    tracksEl.innerHTML = state.project.tracks.map((track) => {
      const meta = TRACK_META[track.type] || TRACK_META.video;
      const clips = [...track.clips].sort((a, b) => a.start - b.start);
      return `
        <div class="track-row${clips.length ? '' : ' is-empty'}${track.hidden ? ' hidden-track' : ''}${track.locked ? ' locked-track' : ''}" data-track-id="${track.id}" data-hint="${meta.hint}">
          ${clips.map((clip) => clipMarkup(clip, px)).join('')}
        </div>`;
    }).join('');
    requestAnimationFrame(drawWaveforms);
  }

  function drawWaveforms() {
    if (destroyed || !state.project) return;
    for (const canvas of tracksEl.querySelectorAll('.clip-wave')) {
      const clipEl = canvas.closest('.clip');
      const found = findClip(clipEl?.dataset.clipId);
      if (!found?.clip?.mediaId) continue;
      const media = state.media.get(found.clip.mediaId);
      const peaks = media?.peaks;
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      if (!width || !height) continue;
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      canvas.width = Math.max(1, Math.floor(width * dpr));
      canvas.height = Math.max(1, Math.floor(height * dpr));
      const ctx = canvas.getContext('2d');
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, width, height);
      if (!peaks?.length) continue;
      ctx.fillStyle = 'rgba(196, 202, 255, 0.62)';
      const bars = Math.min(peaks.length, Math.max(8, Math.floor(width / 2.5)));
      for (let i = 0; i < bars; i += 1) {
        const peak = peaks[Math.floor((i / bars) * peaks.length)] || 0;
        const amplitude = Math.max(1, peak * height * 0.46);
        const x = (i / bars) * width;
        ctx.fillRect(x, height / 2 - amplitude, Math.max(1, width / bars - 1), amplitude * 2);
      }
    }
  }

  function updateSelection() {
    for (const element of tracksEl.querySelectorAll('.clip')) {
      element.classList.toggle('selected', element.dataset.clipId === state.selection);
    }
    updateToolbar();
  }

  function updateToolbar() {
    undoBtn.disabled = !state.history.length;
    redoBtn.disabled = !state.future.length;
    splitBtn.disabled = !state.project?.tracks.some((track) => track.clips.length);
    duplicateBtn.disabled = !state.selection;
    deleteBtn.disabled = !state.selection;
    snapBtn.classList.toggle('active', state.snap);
  }

  function updatePlayhead() {
    const px = pxPerSecond();
    const x = state.playhead * px;
    playhead.style.left = `${x}px`;
    if (state.playing && Math.abs(state.playhead - lastPlayheadTime) > 0.001) {
      const viewLeft = scroll.scrollLeft;
      const viewRight = viewLeft + scroll.clientWidth;
      if (x < viewLeft + 24) scroll.scrollLeft = Math.max(0, x - 24);
      else if (x > viewRight - 80) scroll.scrollLeft = Math.max(0, x - scroll.clientWidth * 0.38);
      lastPlayheadTime = state.playhead;
    }
  }

  function render() {
    if (!state.project || destroyed) return;
    const px = pxPerSecond();
    const length = timelineLength();
    const duration = projectDuration();
    const width = renderRuler(length, px);
    content.style.width = `${width}px`;
    content.style.height = '100%';
    tracksEl.style.setProperty('--grid-step', `${px}px`);
    renderLabels();
    renderTracks(px);
    playhead.style.height = '100%';
    playhead.style.bottom = 'auto';
    guide.style.height = '100%';
    const isEmpty = state.project.tracks.every((track) => !track.clips.length);
    host.classList.toggle('tl-empty', isEmpty);
    emptyHint.classList.toggle('hidden', !isEmpty);
    if (zoomRange) {
      zoomRange.value = String(Math.round(clamp(px, 20, 320)));
      zoomRange.style.setProperty('--fill', `${((clamp(px, 20, 320) - 20) / 300) * 100}%`);
    }
    updatePlayhead();
    updateSelection();
    if (duration <= 0) lastPlayheadTime = -1;
  }

  function showGuide(time) {
    const x = time * pxPerSecond();
    guide.style.left = `${x}px`;
    guide.classList.remove('hidden');
  }

  function hideGuide() {
    guide.classList.add('hidden');
  }

  function snapPoints(excludeId) {
    const points = [state.playhead, 0];
    for (const track of state.project.tracks) {
      for (const clip of track.clips) {
        if (clip.id === excludeId) continue;
        points.push(clip.start, clip.start + clip.duration);
      }
    }
    return points;
  }

  function snapStart(desired, duration, excludeId) {
    if (!state.snap) return { value: Math.max(0, desired), point: null };
    const threshold = 8 / pxPerSecond();
    let best = null;
    for (const point of snapPoints(excludeId)) {
      for (const offset of [0, duration]) {
        const candidate = point - offset;
        const distance = Math.abs(candidate - desired);
        if (candidate >= -0.001 && distance < threshold && (!best || distance < best.distance)) {
          best = { value: Math.max(0, candidate), point, distance };
        }
      }
    }
    return best ? { value: best.value, point: best.point } : { value: Math.max(0, desired), point: null };
  }

  function sourceLimit(clip) {
    if (clip.kind === 'image' || clip.kind === 'text') return Number.POSITIVE_INFINITY;
    const media = clip.mediaId ? state.media.get(clip.mediaId) : null;
    return Math.max(0.1, (media?.duration || clip.in + clip.duration) - (clip.in || 0));
  }

  function beginDrag(event, clipEl, found, mode, edge) {
    if (event.button !== 0) return;
    const { clip, track } = found;
    if (track.locked) {
      toast('La pista está bloqueada', 'warning');
      return;
    }
    event.preventDefault();
    selectClip(clip.id);
    const px = pxPerSecond();
    const startX = event.clientX;
    const startY = event.clientY;
    const originalStart = clip.start;
    const originalDuration = clip.duration;
    let finalStart = originalStart;
    let finalDelta = 0;
    let targetTrackId = track.id;
    let moved = false;
    clipEl.classList.add('dragging');

    const rows = Array.from(tracksEl.querySelectorAll('.track-row'));

    const move = (moveEvent) => {
      if (Math.abs(moveEvent.clientX - startX) > 2 || Math.abs(moveEvent.clientY - startY) > 2) moved = true;
      if (!moved) return;
      if (mode === 'move') {
        const raw = originalStart + (moveEvent.clientX - startX) / px;
        const snapped = moveEvent.altKey
          ? { value: Math.max(0, raw), point: null }
          : snapStart(raw, originalDuration, clip.id);
        finalStart = snapped.value;
        clipEl.style.left = `${finalStart * px}px`;
        if (snapped.point !== null) showGuide(snapped.point); else hideGuide();
        for (const row of rows) row.classList.remove('drag-over');
        const row = rowAt(moveEvent.clientY);
        if (row) {
          const candidate = trackById(row.dataset.trackId);
          if (candidate && candidate.type === track.type && !candidate.locked) {
            targetTrackId = candidate.id;
            row.classList.add('drag-over');
          }
        }
      } else {
        let delta = (moveEvent.clientX - startX) / px;
        if (edge === 'left') {
          delta = clamp(delta, -originalStart, originalDuration - 0.1);
          clipEl.style.left = `${(originalStart + delta) * px}px`;
          clipEl.style.width = `${(originalDuration - delta) * px}px`;
        } else {
          const maxDelta = sourceLimit(clip) - originalDuration;
          delta = clamp(delta, -originalDuration + 0.1, Math.max(-originalDuration + 0.1, maxDelta));
          clipEl.style.width = `${(originalDuration + delta) * px}px`;
        }
        finalDelta = delta;
      }
    };

    const up = () => {
      document.removeEventListener('pointermove', move);
      document.removeEventListener('pointerup', up);
      clipEl.classList.remove('dragging');
      hideGuide();
      for (const row of rows) row.classList.remove('drag-over');
      if (!moved) return;
      if (mode === 'move') moveClip(clip.id, { start: finalStart, trackId: targetTrackId });
      else trimClip(clip.id, edge, finalDelta);
    };

    document.addEventListener('pointermove', move);
    document.addEventListener('pointerup', up, { once: true });
  }

  tracksEl.addEventListener('pointerdown', (event) => {
    const handle = event.target.closest('.clip-handle');
    const clipEl = event.target.closest('.clip');
    if (!clipEl) return;
    const found = findClip(clipEl.dataset.clipId);
    if (!found) return;
    beginDrag(event, clipEl, found, handle ? 'trim' : 'move', handle?.dataset.edge);
  });

  tracksEl.addEventListener('click', (event) => {
    if (event.target.closest('.clip')) return;
    if (!state.project) return;
    selectClip(null);
    if (state.playing) pause();
    seek(timeFromX(event.clientX));
  });

  content.addEventListener('pointerdown', (event) => {
    if (!event.target.closest('[data-ruler]')) return;
    if (event.button !== 0) return;
    event.preventDefault();
    if (state.playing) pause();
    const scrub = (clientX) => {
      const time = timeFromX(clientX);
      seek(time);
      tooltip.textContent = timecode(time);
      tooltip.style.left = `${time * pxPerSecond()}px`;
      tooltip.classList.remove('hidden');
    };
    scrub(event.clientX);
    const move = (moveEvent) => scrub(moveEvent.clientX);
    const up = () => {
      tooltip.classList.add('hidden');
      document.removeEventListener('pointermove', move);
      document.removeEventListener('pointerup', up);
    };
    document.addEventListener('pointermove', move);
    document.addEventListener('pointerup', up, { once: true });
  });

  const hasPayload = (event) => {
    const types = Array.from(event.dataTransfer?.types || []);
    return types.includes('Files') || types.includes('application/x-openCUP-media');
  };

  tracksEl.addEventListener('dragover', (event) => {
    if (!hasPayload(event)) return;
    event.preventDefault();
    const row = rowAt(event.clientY);
    for (const other of tracksEl.querySelectorAll('.track-row')) {
      other.classList.toggle('drag-over', other === row);
    }
  });

  tracksEl.addEventListener('dragleave', (event) => {
    if (event.relatedTarget && tracksEl.contains(event.relatedTarget)) return;
    for (const row of tracksEl.querySelectorAll('.track-row')) row.classList.remove('drag-over');
  });

  tracksEl.addEventListener('drop', async (event) => {
    if (!hasPayload(event)) return;
    event.preventDefault();
    for (const row of tracksEl.querySelectorAll('.track-row')) row.classList.remove('drag-over');
    const row = rowAt(event.clientY);
    const track = row ? trackById(row.dataset.trackId) : null;
    if (track?.locked) {
      toast('La pista está bloqueada', 'warning');
      return;
    }
    const mediaId = event.dataTransfer.getData('application/x-openCUP-media');
    if (mediaId) {
      const start = timeFromX(event.clientX);
      const clip = addMediaToTimeline(mediaId, { trackId: track?.id, start });
      if (!clip) toast('Ese archivo no se puede colocar en esta pista', 'warning');
      return;
    }
    if (event.dataTransfer.files?.length) {
      await importFiles(event.dataTransfer.files);
    }
  });

  labels.addEventListener('click', (event) => {
    const addType = event.target.closest('[data-add-track-type]');
    if (addType) {
      addTrack(addType.dataset.addTrackType);
      return;
    }
    const button = event.target.closest('[data-track-toggle]');
    if (!button) return;
    const label = button.closest('.track-label');
    const track = trackById(label?.dataset.trackId);
    if (!track || !state.project) return;
    const field = button.dataset.trackToggle;
    const tracks = state.project.tracks.map((item) => (
      item.id === track.id ? { ...item, [field]: !item[field] } : item
    ));
    updateProject({ tracks });
  });

  toolbar.addEventListener('click', async (event) => {
    const button = event.target.closest('[data-act]');
    if (!button) return;
    const action = button.dataset.act;
    if (action === 'undo') undo();
    else if (action === 'redo') redo();
    else if (action === 'split') splitAtPlayhead();
    else if (action === 'duplicate') duplicateSelected();
    else if (action === 'delete') deleteSelected();
    else if (action === 'text') addTextClip('Tu texto aquí');
    else if (action === 'snap') toggleSnap();
    else if (action === 'import') fileInput.click();
    else if (action === 'select') toast('Herramienta de selección activa', 'info', 1300);
    else if (action === 'shield') toast('El proyecto se guarda automáticamente en este navegador', 'success', 2400);
    else if (action === 'mic') setPanel('audio');
    else if (action === 'link') toast('Sincronización de pistas disponible al mover clips', 'info', 1800);
    else if (action === 'auto') toast('La edición automática con IA no está incluida en esta versión local', 'info', 2400);
    else if (action === 'zoomIn') setZoom(pxPerSecond() * 1.3);
    else if (action === 'zoomOut') setZoom(pxPerSecond() / 1.3);
    else if (action === 'fit') {
      const fit = (scroll.clientWidth - 26) / Math.max(1, timelineLength());
      setZoom(fit);
      scroll.scrollLeft = 0;
    }
  });

  if (zoomRange) {
    zoomRange.addEventListener('input', () => {
      setZoom(Number(zoomRange.value));
    });
  }

  fileInput.addEventListener('change', async () => {
    if (fileInput.files?.length) await importFiles(fileInput.files);
    fileInput.value = '';
  });

  subscriptions.push(on('project', render));
  subscriptions.push(on('selection', updateSelection));
  subscriptions.push(on('tick', updatePlayhead));
  subscriptions.push(on('playing', () => {
    if (!state.playing) lastPlayheadTime = -1;
    updateToolbar();
  }));
  subscriptions.push(on('zoom', render));
  subscriptions.push(on('snap', updateToolbar));

  const onResize = () => {
    if (!state.project) return;
    const px = pxPerSecond();
    const length = timelineLength();
    content.style.width = `${Math.max(length * px, scroll.clientWidth)}px`;
  };
  window.addEventListener('resize', onResize);

  render();

  return {
    element: host,
    destroy() {
      destroyed = true;
      window.removeEventListener('resize', onResize);
      subscriptions.forEach((off) => off());
    },
  };
}
