import {
  ASPECTS,
  state,
  on,
  projectDuration,
  togglePlay,
  seek,
  stepFrame,
  skipToEdge,
  selectClip,
  selectedClip,
  updateClip,
  updateProject,
  importFiles,
  formatTime,
} from './store.js';
import { MediaPool, renderFrame, activeLayers, clipLayerBox } from './renderer.js';
import { getGLPost } from './gl-post.js';
import { transformAt, hasKeyframes, setKeyframe, writeProp } from './keyframes.js';
import { icon } from './icons.js';
import { toast } from './utils.js';
import { captureScreenshot } from './export.js';

export function mountPreview(host) {
  host.classList.add('center', 'preview-panel');
  host.innerHTML = `
    <div class="preview-head">
      <div class="center-title"><span>Reproductor-Línea de tiempo 01</span></div>
      <div class="head-right">
        <span class="head-menu" title="Opciones del reproductor">${icon('menu', 16)}</span>
        <span class="head-dot"></span>
      </div>
    </div>
    <div class="stage-wrap">
      <div class="stage">
        <canvas class="stage-canvas"></canvas>
        <div class="stage-overlay" data-overlay hidden>
          <div class="layer-frame" data-frame hidden>
            <span class="layer-handle nw" data-handle="nw"></span>
            <span class="layer-handle ne" data-handle="ne"></span>
            <span class="layer-handle sw" data-handle="sw"></span>
            <span class="layer-handle se" data-handle="se"></span>
            <span class="layer-rotate" data-handle="rotate"></span>
          </div>
          <div class="snap-guide v" data-guide="v" hidden></div>
          <div class="snap-guide h" data-guide="h" hidden></div>
        </div>
        <div class="preview-controls">
          <div class="pc-left">
            <span class="timecode"><span data-current>00:00:00:00</span><i>/</i><span data-total>00:00:00:00</span></span>
            <button class="icon-btn sm" data-act="mute" title="Silenciar">${icon('volume', 14)}</button>
          </div>
          <div class="pc-center">
            <button class="play-btn" data-act="play" title="Reproducir (Espacio)" aria-label="Reproducir">${icon('play', 15)}</button>
          </div>
          <div class="pc-right">
            <button class="text-ctrl" data-act="fit" title="Ajustar la vista">Completa</button>
            <button class="icon-btn sm" data-act="fit" title="Ajustar la vista">${icon('fit', 13)}</button>
            <button class="text-ctrl" data-act="ratio" title="Cambiar relación de aspecto">Relación</button>
            <button class="icon-btn sm" data-act="ratio" title="Cambiar relación de aspecto">${icon('aspect', 13)}</button>
            <button class="icon-btn sm" data-act="fullscreen" title="Pantalla completa">${icon('fullscreen', 13)}</button>
          </div>
        </div>
      </div>
    </div>
    <input type="file" class="hidden" multiple data-preview-import accept="video/*,image/*,audio/*,.mkv,.mov,.m4v,.webm,.mp3,.wav,.m4a,.aac,.ogg" />`;

  const stageWrap = host.querySelector('.stage-wrap');
  const stage = host.querySelector('.stage');
  const canvas = host.querySelector('.stage-canvas');
  const ctx = canvas.getContext('2d');
  const playBtn = host.querySelector('[data-act="play"]');
  const muteBtn = host.querySelector('[data-act="mute"]');
  const currentEl = host.querySelector('[data-current]');
  const totalEl = host.querySelector('[data-total]');
  const importInput = host.querySelector('[data-preview-import]');
  const pool = new MediaPool(state.media);
  const subscriptions = [];
  let muted = false;
  let drawRequest = 0;
  let animRequest = 0;
  let animClockStart = 0;
  let animClockValue = 0;
  let destroyed = false;
  let lastCanvasWidth = 0;
  let lastCanvasHeight = 0;

  const ratio = () => ASPECTS[state.project?.ratio] || 16 / 9;

  function hasVisualContent() {
    return Boolean(state.project?.tracks?.some((track) => track.clips.some((clip) => clip.kind !== 'audio')));
  }

  function contentRect() {
    const stageWidth = Math.max(1, stage.clientWidth);
    const stageHeight = Math.max(1, stage.clientHeight);
    const value = ratio();
    let width = stageWidth;
    let height = width / value;
    if (height > stageHeight) {
      height = stageHeight;
      width = height * value;
    }
    return {
      width: Math.max(1, Math.floor(width)),
      height: Math.max(1, Math.floor(height)),
      left: Math.max(0, Math.floor((stageWidth - width) / 2)),
      top: Math.max(0, Math.floor((stageHeight - height) / 2)),
    };
  }

  const overlay = host.querySelector('[data-overlay]');
  const frame = host.querySelector('[data-frame]');
  const guideV = host.querySelector('[data-guide="v"]');
  const guideH = host.querySelector('[data-guide="h"]');
  let drag = null;

  function hideGuides() {
    guideV.hidden = true;
    guideH.hidden = true;
  }

  function boxFor(clip, rect) {
    return clipLayerBox(clip, rect.width, rect.height, state.playhead);
  }

  function selectedVisual() {
    const found = selectedClip();
    if (!found || !['text', 'sticker'].includes(found.clip.kind)) return null;
    const clip = found.clip;
    const active = state.playhead >= clip.start - 0.0001 && state.playhead < clip.start + clip.duration;
    return active ? clip : null;
  }

  function updateOverlay() {
    if (!state.project || !hasVisualContent() || state.playing) {
      overlay.hidden = true;
      frame.hidden = true;
      hideGuides();
      return;
    }
    const rect = contentRect();
    overlay.style.left = `${rect.left}px`;
    overlay.style.top = `${rect.top}px`;
    overlay.style.width = `${rect.width}px`;
    overlay.style.height = `${rect.height}px`;
    overlay.hidden = false;
    const clip = selectedVisual();
    if (!clip) {
      frame.hidden = true;
      return;
    }
    const box = boxFor(clip, rect);
    if (!box) {
      frame.hidden = true;
      return;
    }
    frame.hidden = false;
    frame.dataset.kind = box.kind;
    frame.style.left = `${box.cx}px`;
    frame.style.top = `${box.cy}px`;
    frame.style.width = `${Math.max(18, box.width * (box.scaleX ?? box.scale))}px`;
    frame.style.height = `${Math.max(18, box.height * (box.scaleY ?? box.scale))}px`;
    frame.style.transform = `translate(-50%, -50%) rotate(${box.rotation}deg)`;
  }

  function topLayerAt(point, rect) {
    const layers = activeLayers(state.project, state.playhead);
    for (let index = layers.length - 1; index >= 0; index -= 1) {
      const layer = layers[index];
      if (!['text', 'sticker'].includes(layer.clip.kind)) continue;
      const box = boxFor(layer.clip, rect);
      if (!box) continue;
      const dx = point.x - box.cx;
      const dy = point.y - box.cy;
      const theta = (box.rotation * Math.PI) / 180;
      const localX = (dx * Math.cos(theta) + dy * Math.sin(theta)) / (box.scale || 1);
      const localY = (-dx * Math.sin(theta) + dy * Math.cos(theta)) / (box.scale || 1);
      if (Math.abs(localX) <= box.width / 2 + 5 && Math.abs(localY) <= box.height / 2 + 5) {
        return layer;
      }
    }
    return null;
  }

  function pointerInOverlay(event) {
    const bounds = overlay.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  }

  function startDrag(mode, clip, point, rect) {
    const box = boxFor(clip, rect);
    if (!box) return;
    const tr = transformAt(clip, state.playhead);
    drag = {
      mode,
      clip,
      rect,
      box,
      moved: false,
      start: {
        pointer: point,
        x: tr.x,
        y: tr.y,
        scale: tr.scaleX,
        scaleX: tr.scaleX,
        scaleY: tr.scaleY,
        uniform: clip.uniform !== false,
        rotate: tr.rotate,
        base: {
          x: clip.x,
          y: clip.y,
          scale: clip.scale,
          scaleX: clip.scaleX,
          scaleY: clip.scaleY,
          rotate: clip.rotate,
        },
        keyframes: JSON.parse(JSON.stringify(clip.keyframes || {})),
        distance: Math.max(6, Math.hypot(point.x - box.cx, point.y - box.cy)),
        angle: Math.atan2(point.y - box.cy, point.x - box.cx),
      },
    };
    frame.classList.add('is-dragging');
    window.addEventListener('pointermove', onDragMove);
    window.addEventListener('pointerup', endDrag);
    window.addEventListener('pointercancel', endDrag);
  }

  function onDragMove(event) {
    if (!drag) return;
    const point = pointerInOverlay(event);
    const { clip, rect, box, start } = drag;
    if (drag.mode === 'move') {
      let x = start.x + ((point.x - start.pointer.x) / rect.width) * 100;
      let y = start.y + ((point.y - start.pointer.y) / rect.height) * 100;
      const extraX = clip.kind === 'text' ? (clip.text?.x || 0) : (clip.sticker?.x || 0);
      const extraY = clip.kind === 'text' ? (clip.text?.y || 0) : (clip.sticker?.y || 0);
      const snapX = -extraX;
      const snapY = -extraY;
      if (Math.abs(x - snapX) < 1.2) {
        x = snapX;
        guideV.style.left = `${rect.width / 2}px`;
        guideV.hidden = false;
      } else {
        guideV.hidden = true;
      }
      if (Math.abs(y - snapY) < 1.2) {
        y = snapY;
        guideH.style.top = `${rect.height / 2}px`;
        guideH.hidden = false;
      } else {
        guideH.hidden = true;
      }
      const nx = Math.round(x * 100) / 100;
      const ny = Math.round(y * 100) / 100;
      writeProp(clip, 'x', nx, state.playhead);
      writeProp(clip, 'y', ny, state.playhead);
      drag.moved = drag.moved || Math.abs(nx - start.x) > 0.01 || Math.abs(ny - start.y) > 0.01;
    } else if (drag.mode === 'scale') {
      const distance = Math.max(6, Math.hypot(point.x - box.cx, point.y - box.cy));
      const next = Math.max(0.08, Math.min(8, start.scale * (distance / start.distance)));
      if (start.uniform) {
        const value = Math.round(next * 1000) / 1000;
        writeProp(clip, 'scale', value, state.playhead);
        drag.moved = drag.moved || Math.abs(value - start.scale) > 0.001;
      } else {
        const factor = next / start.scale;
        const valueX = Math.round(start.scaleX * factor * 1000) / 1000;
        const valueY = Math.round(start.scaleY * factor * 1000) / 1000;
        writeProp(clip, 'scaleX', valueX, state.playhead);
        writeProp(clip, 'scaleY', valueY, state.playhead);
        drag.moved = drag.moved || Math.abs(valueX - start.scaleX) > 0.001 || Math.abs(valueY - start.scaleY) > 0.001;
      }
    } else if (drag.mode === 'rotate') {
      const angle = Math.atan2(point.y - box.cy, point.x - box.cx);
      let delta = ((angle - start.angle) * 180) / Math.PI;
      if (delta > 180) delta -= 360;
      if (delta < -180) delta += 360;
      let next = start.rotate + delta;
      if (event.shiftKey) next = Math.round(next / 15) * 15;
      const value = Math.round(next * 10) / 10;
      writeProp(clip, 'rotate', value, state.playhead);
      drag.moved = drag.moved || Math.abs(value - start.rotate) > 0.01;
    }
    schedulePaint();
    event.preventDefault();
  }

  function endDrag() {
    window.removeEventListener('pointermove', onDragMove);
    window.removeEventListener('pointerup', endDrag);
    window.removeEventListener('pointercancel', endDrag);
    frame.classList.remove('is-dragging');
    hideGuides();
    if (!drag) return;
    const { clip, moved, start } = drag;
    drag = null;
    if (!moved) return;
    const nextKeyframes = clip.keyframes;
    const patch = { x: clip.x, y: clip.y, scale: clip.scale, rotate: clip.rotate, keyframes: nextKeyframes };
    if (clip.scaleX !== undefined) patch.scaleX = clip.scaleX;
    if (clip.scaleY !== undefined) patch.scaleY = clip.scaleY;
    // Se restaura el estado inicial para que el historial guarde el cambio como un solo paso.
    Object.assign(clip, start.base);
    clip.keyframes = start.keyframes;
    updateClip(clip.id, patch, { history: true });
  }

  overlay.addEventListener('pointerdown', (event) => {
    if (event.button !== 0 || state.playing || !state.project) return;
    const rect = contentRect();
    const point = pointerInOverlay(event);
    const handle = event.target.closest('[data-handle]');
    const active = selectedVisual();
    if (handle && active) {
      startDrag(handle.dataset.handle === 'rotate' ? 'rotate' : 'scale', active, point, rect);
      event.preventDefault();
      return;
    }
    const hit = topLayerAt(point, rect);
    if (hit) {
      if (state.selection !== hit.clip.id) selectClip(hit.clip.id);
      startDrag('move', hit.clip, point, rect);
    } else {
      selectClip(null);
    }
    event.preventDefault();
  });

  overlay.addEventListener('pointermove', (event) => {
    if (drag || state.playing) return;
    const rect = contentRect();
    overlay.classList.toggle('can-move', Boolean(topLayerAt(pointerInOverlay(event), rect)));
  });

  overlay.addEventListener('dblclick', (event) => {
    if (event.target === overlay) togglePlay();
  });

  function layout() {
    if (!state.project) return;
    paint();
  }

  function refreshTimecode() {
    if (!state.project) return;
    const fps = state.project.fps || 30;
    currentEl.textContent = formatTime(state.playhead, { fps, frames: true, padHours: true });
    totalEl.textContent = formatTime(projectDuration(), { fps, frames: true, padHours: true });
  }

  function paint() {
    if (destroyed || !state.project) return;
    const visual = hasVisualContent();
    stage.classList.toggle('has-content', visual);
    if (!visual) {
      canvas.style.display = 'none';
      pool.syncPlayback(state.project, state.playhead, state.playing, muted);
      updateOverlay();
      return;
    }
    canvas.style.display = 'block';
    const rect = contentRect();
    canvas.style.left = `${rect.left}px`;
    canvas.style.top = `${rect.top}px`;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    const dpr = Math.min(2.5, window.devicePixelRatio || 1);
    const pixelWidth = Math.round(rect.width * dpr);
    const pixelHeight = Math.round(rect.height * dpr);
    if (pixelWidth !== lastCanvasWidth || pixelHeight !== lastCanvasHeight) {
      canvas.width = pixelWidth;
      canvas.height = pixelHeight;
      lastCanvasWidth = pixelWidth;
      lastCanvasHeight = pixelHeight;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    pool.syncPlayback(state.project, state.playhead, state.playing, muted);
    renderFrame(ctx, rect.width, rect.height, state.project, pool, state.playhead, {
      animClock: state.playing ? 0 : animClockValue,
    });
    const activeFx = (state.project?.effectClips ?? []).find((fx) => state.playhead >= fx.start && state.playhead < fx.start + fx.duration);
    const glEffect = activeFx?.glEffect ?? state.glEffect;
    if (glEffect) {
      try {
        const post = getGLPost();
        if (post.apply(canvas, glEffect, state.playhead, activeFx?.amount ?? state.glAmount ?? 1)) {
          ctx.save();
          ctx.setTransform(1, 0, 0, 1, 0, 0);
          ctx.drawImage(post.canvasEl, 0, 0);
          ctx.restore();
        }
      } catch { /* el previo sigue funcionando sin GL */ }
    }
    updateOverlay();
  }

  function schedulePaint() {
    if (drawRequest || destroyed) return;
    drawRequest = requestAnimationFrame(() => {
      drawRequest = 0;
      paint();
    });
  }

  window.addEventListener('opencup:glfx', schedulePaint);

  /* Las animaciones de texto en bucle se previsualizan aunque esté pausado. */
  function hasLiveTextAnimation() {
    if (!state.project) return false;
    const time = state.playhead;
    for (const track of state.project.tracks) {
      if (track.hidden || track.locked) continue;
      for (const clip of track.clips) {
        if (clip.kind !== 'text') continue;
        const anim = clip.text?.animation;
        if (!anim || anim === 'none') continue;
        if (time >= clip.start && time < clip.start + clip.duration) return true;
      }
    }
    return false;
  }

  function animationLoop() {
    animRequest = 0;
    if (destroyed || state.playing || !hasLiveTextAnimation()) return;
    animClockValue = animClockStart ? (performance.now() - animClockStart) / 1000 : 0;
    paint();
    animRequest = requestAnimationFrame(animationLoop);
  }

  function scheduleAnimationPreview() {
    animClockStart = performance.now();
    animClockValue = 0;
    if (animRequest || destroyed || state.playing) return;
    animRequest = requestAnimationFrame(animationLoop);
  }

  function updateTransport() {
    if (!state.project) return;
    const playing = state.playing;
    playBtn.innerHTML = icon(playing ? 'pause' : 'play', 15);
    playBtn.title = playing ? 'Pausar (Espacio)' : 'Reproducir (Espacio)';
    playBtn.setAttribute('aria-label', playing ? 'Pausar' : 'Reproducir');
    playBtn.classList.toggle('is-playing', playing);
    refreshTimecode();
    schedulePaint();
  }

  pool.onReady = schedulePaint;

  const observer = new ResizeObserver(() => layout());
  observer.observe(stageWrap);
  window.addEventListener('resize', layout);

  subscriptions.push(on('tick', () => {
    refreshTimecode();
    schedulePaint();
    scheduleAnimationPreview();
  }));
  subscriptions.push(on('project', () => {
    layout();
    refreshTimecode();
    updateTransport();
    scheduleAnimationPreview();
  }));
  subscriptions.push(on('playing', () => {
    updateTransport();
    scheduleAnimationPreview();
  }));
  subscriptions.push(on('selection', schedulePaint));

  importInput.addEventListener('change', async () => {
    if (importInput.files?.length) await importFiles(importInput.files);
    importInput.value = '';
  });

  host.addEventListener('click', async (event) => {
    const button = event.target.closest('[data-act]');
    if (!button) {
      if (event.target === canvas || event.target === stage) selectClip(null);
      return;
    }
    const action = button.dataset.act;
    if (action === 'play') togglePlay();
    else if (action === 'start') skipToEdge('start');
    else if (action === 'end') skipToEdge('end');
    else if (action === 'prev') stepFrame(-1);
    else if (action === 'next') stepFrame(1);
    else if (action === 'import') importInput.click();
    else if (action === 'mute') {
      muted = !muted;
      pool.muted = muted;
      muteBtn.innerHTML = icon(muted ? 'mute' : 'volume', 14);
      muteBtn.classList.toggle('active', muted);
      muteBtn.title = muted ? 'Activar sonido' : 'Silenciar';
      schedulePaint();
    } else if (action === 'ratio') {
      const values = Object.keys(ASPECTS);
      const index = values.indexOf(state.project?.ratio || '16:9');
      const next = values[(index + 1) % values.length];
      updateProject({ ratio: next });
      toast(`Relación de aspecto: ${next}`, 'info', 1600);
    } else if (action === 'fit') {
      toast('Vista ajustada al reproductor', 'info', 1400);
    } else if (action === 'fullscreen') {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await stageWrap.requestFullscreen?.().catch(() => {});
    } else if (action === 'shot') {
      button.disabled = true;
      try {
        await captureScreenshot();
      } catch (error) {
        console.error(error);
        toast('No se pudo capturar el fotograma', 'error');
      } finally {
        button.disabled = false;
      }
    }
  });

  host.addEventListener('dblclick', (event) => {
    if (event.target === canvas) togglePlay();
  });

  const onFullscreen = () => setTimeout(layout, 80);
  document.addEventListener('fullscreenchange', onFullscreen);

  updateTransport();
  requestAnimationFrame(layout);

  return {
    element: host,
    redraw: schedulePaint,
    destroy() {
      destroyed = true;
      if (drag) endDrag();
      cancelAnimationFrame(drawRequest);
      cancelAnimationFrame(animRequest);
      observer.disconnect();
      window.removeEventListener('resize', layout);
      document.removeEventListener('fullscreenchange', onFullscreen);
      subscriptions.forEach((off) => off());
      pool.destroy();
    },
  };
}
