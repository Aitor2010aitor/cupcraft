import {
  ASPECTS,
  RESOLUTIONS,
  state,
  projectDuration,
  formatTime,
} from './store.js';
import { MediaPool, renderFrame, renderStill } from './renderer.js';
import { getGLPost } from './gl-post.js';
import { openModal } from './modal.js';
import { icon } from './icons.js';
import { downloadBlob, toast } from './utils.js';
import { fixWebmDuration } from './webm.js';

export function sanitizeName(value) {
  return String(value || 'openCUP')
    .replace(/[^\w\sáéíóúüñÁÉÍÓÚÜÑ-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .slice(0, 60) || 'openCUP';
}

export function exportDimensions(project, resolutionKey = '1080p') {
  const maxSide = {
    '480p': 854,
    '720p': 1280,
    '1080p': 1920,
    '4K': 3840,
  }[resolutionKey] || 1920;
  const ratio = ASPECTS[project?.ratio] || 16 / 9;
  let width;
  let height;
  if (ratio >= 1) {
    width = maxSide;
    height = Math.round(width / ratio);
  } else {
    height = maxSide;
    width = Math.round(height * ratio);
  }
  width = Math.max(2, Math.round(width / 2) * 2);
  height = Math.max(2, Math.round(height / 2) * 2);
  return { width, height };
}

function bitrateFor(quality, width, height, fps) {
  const pixels = width * height * fps;
  const factor = { baja: 0.055, media: 0.1, alta: 0.17, máxima: 0.26 }[quality] ?? 0.1;
  return Math.round(Math.max(800_000, Math.min(80_000_000, pixels * factor)));
}

async function preloadProject(pool, project) {
  const jobs = [];
  for (const track of project.tracks) {
    for (const clip of track.clips) {
      if (clip.kind === 'image') {
        const image = pool.image(clip.mediaId);
        if (image && !image.complete) {
          jobs.push(new Promise((resolve) => {
            image.addEventListener('load', resolve, { once: true });
            image.addEventListener('error', resolve, { once: true });
            setTimeout(resolve, 8000);
          }));
        }
      } else if (clip.kind === 'video') {
        const video = pool.video(clip);
        if (video) {
          jobs.push(new Promise((resolve) => {
            const ready = () => {
              try { video.currentTime = clip.in || 0; } catch { /* ignorado */ }
              const finish = () => resolve();
              video.addEventListener('seeked', finish, { once: true });
              setTimeout(finish, 3500);
            };
            if (video.readyState >= 2) ready();
            else {
              video.addEventListener('loadeddata', ready, { once: true });
              video.addEventListener('error', resolve, { once: true });
              setTimeout(resolve, 8000);
            }
          }));
        }
      } else if (clip.kind === 'audio') {
        const audio = pool.audio(clip);
        if (audio && audio.readyState === 0) audio.load();
      }
    }
  }
  await Promise.allSettled(jobs);
}

/** Aplica al canvas de salida los filtros GL del clip en `time` (LUT de CapCut y shader propio). */
function applyClipFilters(canvas, ctx, project, time) {
  const clips = (project?.tracks ?? [])
    .filter((track) => track.type === 'video' || track.type === 'overlay')
    .flatMap((track) => track.clips);
  const active = clips.find((clip) => time >= clip.start && time < clip.start + clip.duration);
  if (!active) return;
  try {
    const post = getGLPost();
    let done = false;
    if (active.lut) done = post.applyLut(canvas, active.lut, active.lutAmount ?? 1) || done;
    if (active.glFilter) done = post.apply(canvas, active.glFilter, time, active.glFilterAmount ?? 1) || done;
    if (done) {
      ctx.save();
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.drawImage(post.canvasEl, 0, 0);
      ctx.restore();
    }
  } catch { /* la exportacion sigue sin filtro si GL falla */ }
}

async function recordProject({ project, mediaMap, width, height, fps, bitrate, onProgress, controller }) {
  if (!window.MediaRecorder) throw new Error('Tu navegador no soporta MediaRecorder');
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { alpha: false });

  const stream = canvas.captureStream(fps);
  const hasAudio = project.tracks.some((track) => (
    !track.muted && track.clips.some((clip) => (
      (clip.kind === 'audio' || clip.kind === 'video') && (clip.volume ?? 1) > 0
    ))
  ));

  let audioContext = null;
  let destination = null;
  if (hasAudio && (window.AudioContext || window.webkitAudioContext)) {
    const AudioCtor = window.AudioContext || window.webkitAudioContext;
    audioContext = new AudioCtor();
    destination = audioContext.createMediaStreamDestination();
    audioContext.resume().catch(() => {});
    for (const track of destination.stream.getAudioTracks()) stream.addTrack(track);
  }

  const pool = new MediaPool(mediaMap);
  if (audioContext && destination) pool.attachAudio(audioContext, destination);

  try {
    await preloadProject(pool, project);
    if (audioContext?.state === 'suspended') await audioContext.resume().catch(() => {});
  } catch (error) {
    console.warn('No se pudieron precargar todos los medios', error);
  }

  const candidates = [
    'video/webm;codecs=vp9,opus',
    'video/webm;codecs=vp8,opus',
    'video/webm;codecs=vp9',
    'video/webm',
  ];
  const mimeType = candidates.find((type) => MediaRecorder.isTypeSupported?.(type)) || '';

  const recorder = new MediaRecorder(stream, {
    ...(mimeType ? { mimeType } : {}),
    videoBitsPerSecond: bitrate,
    audioBitsPerSecond: 128_000,
  });
  const chunks = [];
  recorder.addEventListener('dataavailable', (event) => {
    if (event.data?.size) chunks.push(event.data);
  });
  const stopped = new Promise((resolve) => {
    recorder.addEventListener('stop', resolve, { once: true });
  });

  const duration = Math.max(0.04, projectDuration());
  recorder.start(250);
  const startedAt = performance.now();
  let lastReport = 0;

  await new Promise((resolve) => {
    const loop = () => {
      if (controller.cancelled) {
        resolve();
        return;
      }
      const elapsed = (performance.now() - startedAt) / 1000;
      const time = Math.min(duration, elapsed);
      renderFrame(ctx, width, height, project, pool, time);
      applyClipFilters(canvas, ctx, project, time);
      pool.syncPlayback(project, time, true, false);
      const progress = Math.min(1, time / duration);
      if (progress - lastReport > 0.002 || progress >= 1) {
        lastReport = progress;
        onProgress?.(progress, time, duration);
      }
      if (time >= duration) {
        resolve();
        return;
      }
      requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
  });

  if (recorder.state !== 'inactive') recorder.stop();
  await stopped;
  pool.syncPlayback(project, 0, false, true);
  pool.destroy();
  for (const track of stream.getTracks()) track.stop();
  if (audioContext) await audioContext.close().catch(() => {});

  if (controller.cancelled) return null;
  const blob = new Blob(chunks, { type: (mimeType.split(';')[0] || 'video/webm') });
  return fixWebmDuration(blob, duration);
}

export async function captureScreenshot() {
  const project = state.project;
  if (!project) return;
  const { width, height } = exportDimensions(project, project.resolution || '1080p');
  const canvas = await renderStill(project, state.media, state.playhead, width, height);
  const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
  if (!blob) throw new Error('No se pudo generar la imagen');
  const time = formatTime(state.playhead).replace(/:/g, '-');
  downloadBlob(blob, `${sanitizeName(project.name)}-${time}.png`);
  toast('Fotograma exportado en PNG', 'success');
}

export function openExportDialog() {
  if (!state.project) return;
  if (projectDuration() < 0.05) {
    toast('Añade material a la línea de tiempo antes de exportar', 'warning');
    return;
  }
  if (!window.MediaRecorder) {
    toast('Tu navegador no soporta la grabación de video', 'error');
    return;
  }

  const body = `
    <div class="field-row">
      <div class="field">
        <label>Nombre del archivo</label>
        <input class="input" data-export-name value="${state.project.name.replace(/"/g, '&quot;')}" maxlength="70" />
      </div>
      <div class="field">
        <label>Formato</label>
        <select class="input" data-export-format disabled><option>WebM · VP9 + Opus</option></select>
      </div>
    </div>
    <div class="field-row">
      <div class="field">
        <label>Resolución</label>
        <select class="input" data-export-resolution>
          ${Object.keys(RESOLUTIONS).map((key) => `<option value="${key}" ${key === state.project.resolution ? 'selected' : ''}>${key}</option>`).join('')}
        </select>
      </div>
      <div class="field">
        <label>Fotogramas por segundo</label>
        <select class="input" data-export-fps>
          ${[24, 30, 60].map((fps) => `<option value="${fps}" ${fps === state.project.fps ? 'selected' : ''}>${fps} fps</option>`).join('')}
        </select>
      </div>
    </div>
    <div class="field">
      <label>Calidad</label>
      <select class="input" data-export-quality>
        <option value="baja">Baja · archivo ligero</option>
        <option value="media">Media · recomendada</option>
        <option value="alta" selected>Alta</option>
        <option value="máxima">Máxima</option>
      </select>
    </div>
    <div class="modal-note">
      ${icon('info', 13)} La exportación se graba en tiempo real: tarda aproximadamente lo que dura el proyecto.
      El resultado se descarga como <b>WebM</b>, compatible con Chrome, Edge y la mayoría de reproductores.
      El audio de videos y pistas de audio se incluye automáticamente.
    </div>
    <div class="export-progress hidden" data-export-progress>
      <div class="row"><span data-export-label>Preparando medios…</span><span data-export-percent>0%</span></div>
      <div class="progress"><div data-export-bar></div></div>
      <div class="row"><span data-export-info></span><span data-export-time></span></div>
    </div>`;

  let exporting = false;
  let controller = { cancelled: false };
  let modal = null;

  const api = openModal({
    title: 'Exportar video',
    width: 540,
    dismissible: false,
    body,
    footer: `<button class="btn" data-export-cancel>Cancelar</button><button class="btn btn-primary" data-export-start>${icon('export', 14)} Exportar</button>`,
    onMount(instance) {
      modal = instance;
      const startButton = instance.foot.querySelector('[data-export-start]');
      const cancelButton = instance.foot.querySelector('[data-export-cancel]');
      const progress = instance.body.querySelector('[data-export-progress]');
      const bar = instance.body.querySelector('[data-export-bar]');
      const percent = instance.body.querySelector('[data-export-percent]');
      const label = instance.body.querySelector('[data-export-label]');
      const info = instance.body.querySelector('[data-export-info]');
      const timeLabel = instance.body.querySelector('[data-export-time]');
      const closeButton = instance.root.querySelector('[data-modal-close]');

      const resetUi = () => {
        exporting = false;
        controller = { cancelled: false };
        startButton.disabled = false;
        startButton.innerHTML = `${icon('export', 14)} Exportar`;
        cancelButton.textContent = 'Cancelar';
        closeButton.style.display = '';
        for (const field of instance.body.querySelectorAll('input, select')) field.disabled = false;
        instance.body.querySelector('[data-export-format]').disabled = true;
      };

      cancelButton.addEventListener('click', () => {
        if (exporting) {
          controller.cancelled = true;
          label.textContent = 'Cancelando…';
          return;
        }
        api.close();
      });

      startButton.addEventListener('click', async () => {
        if (exporting) return;
        const name = instance.body.querySelector('[data-export-name]').value.trim() || 'openCUP';
        const resolutionKey = instance.body.querySelector('[data-export-resolution]').value;
        const fps = Number(instance.body.querySelector('[data-export-fps]').value) || 30;
        const quality = instance.body.querySelector('[data-export-quality]').value;
        const { width, height } = exportDimensions(state.project, resolutionKey);
        const bitrate = bitrateFor(quality, width, height, fps);

        exporting = true;
        controller = { cancelled: false };
        startButton.disabled = true;
        startButton.innerHTML = `${icon('clock', 14)} Exportando…`;
        cancelButton.textContent = 'Cancelar exportación';
        closeButton.style.display = 'none';
        for (const field of instance.body.querySelectorAll('input, select')) field.disabled = true;
        progress.classList.remove('hidden');
        bar.style.width = '0%';
        percent.textContent = '0%';
        label.textContent = 'Preparando medios…';
        info.textContent = `WebM · ${width}×${height} · ${fps} fps`;
        timeLabel.textContent = `00:00 / ${formatTime(projectDuration())}`;

        try {
          const blob = await recordProject({
            project: state.project,
            mediaMap: state.media,
            width,
            height,
            fps,
            bitrate,
            controller,
            onProgress(progressValue, time, total) {
              const pct = Math.round(progressValue * 100);
              bar.style.width = `${pct}%`;
              percent.textContent = `${pct}%`;
              label.textContent = progressValue >= 1 ? 'Finalizando archivo…' : 'Grabando video…';
              timeLabel.textContent = `${formatTime(time)} / ${formatTime(total)}`;
            },
          });
          if (!blob || controller.cancelled) {
            label.textContent = 'Exportación cancelada';
            resetUi();
            return;
          }
          downloadBlob(blob, `${sanitizeName(name)}.webm`);
          label.textContent = '¡Listo! Archivo descargado';
          percent.textContent = '100%';
          bar.style.width = '100%';
          toast('Video exportado correctamente', 'success');
          setTimeout(() => {
            if (exporting) return;
            api.close();
          }, 900);
          resetUi();
          cancelButton.textContent = 'Cerrar';
        } catch (error) {
          console.error(error);
          label.textContent = error?.message || 'No se pudo exportar el video';
          toast('Error al exportar el video', 'error');
          resetUi();
        }
      });
    },
  });

  return api;
}
