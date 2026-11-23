import {
  ASPECTS,
  RESOLUTIONS,
  state,
  projectDuration,
  formatTime,
} from '../store.js';
import { MediaPool, renderFrame, renderStill } from '../renderer.js';
import { getGLPost } from '../gl-post.js';
import { openModal } from '../modal.js';
import { icon } from '../icons.js';
import { downloadBlob, escapeHtml, toast } from '../utils.js';
import { fixWebmDuration } from '../webm.js';
import { encodeGif } from './gif-encoder.js';

export function sanitizeName(value) {
  return String(value || 'cupcraft')
    .replace(/[^\w\sáéíóúüñÁÉÍÓÚÜÑ-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .slice(0, 60) || 'cupcraft';
}

/** Acepta una clave de resolución, 'same' o un número de lado máximo en píxeles. */
export function exportDimensions(project, resolutionKey = '1080p') {
  let maxSide;
  if (typeof resolutionKey === 'number') {
    maxSide = resolutionKey;
  } else {
    const key = resolutionKey === 'same' ? (project?.resolution || '1080p') : resolutionKey;
    maxSide = {
      '480p': 854,
      '720p': 1280,
      '1080p': 1920,
      '1440p': 2560,
      '4K': 3840,
    }[key] || 1920;
  }
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

function timelineAudioClips(project) {
  return (project?.tracks ?? [])
    .filter((track) => !track.muted)
    .flatMap((track) => track.clips)
    .filter((clip) => (
      (clip.kind === 'audio' || clip.kind === 'video')
      && (clip.volume ?? 1) > 0
    ));
}

async function recordProject({
  project,
  mediaMap,
  width,
  height,
  fps,
  bitrate,
  codec = 'vp9',
  audioBitrate = 128_000,
  onProgress,
  controller,
}) {
  if (!window.MediaRecorder) throw new Error('Tu navegador no soporta MediaRecorder');
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { alpha: false });

  const stream = canvas.captureStream(fps);
  const hasAudio = timelineAudioClips(project).length > 0;

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

  // BackgroundMattingV2 tarda ~130 ms por fotograma: se calculan todos antes
  // de grabar para que el recorte IA aparezca también en el vídeo exportado.
  try {
    await pool.prefetchMatting?.(project);
  } catch (error) {
    console.warn('No se pudo precachear el recorte IA', error);
  }

  const candidates = codec === 'vp8'
    ? ['video/webm;codecs=vp8,opus', 'video/webm;codecs=vp8', 'video/webm']
    : ['video/webm;codecs=vp9,opus', 'video/webm;codecs=vp8,opus', 'video/webm;codecs=vp9', 'video/webm'];
  const mimeType = candidates.find((type) => MediaRecorder.isTypeSupported?.(type)) || '';

  const recorder = new MediaRecorder(stream, {
    ...(mimeType ? { mimeType } : {}),
    videoBitsPerSecond: bitrate,
    audioBitsPerSecond: audioBitrate,
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

/** Mezcla de audio real, sin pista de vídeo: MediaRecorder sobre un MediaStreamDestination. */
async function recordAudioOnly({ project, mediaMap, audioBitrate = 128_000, onProgress, controller }) {
  if (!window.MediaRecorder) throw new Error('Tu navegador no soporta MediaRecorder');
  if (!timelineAudioClips(project).length) {
    throw new Error('La línea de tiempo no tiene audio que exportar');
  }
  const AudioCtor = window.AudioContext || window.webkitAudioContext;
  if (!AudioCtor) throw new Error('Tu navegador no soporta AudioContext');

  const audioContext = new AudioCtor();
  const destination = audioContext.createMediaStreamDestination();
  audioContext.resume().catch(() => {});
  const pool = new MediaPool(mediaMap);
  pool.attachAudio(audioContext, destination);

  try {
    await preloadProject(pool, project);
    if (audioContext.state === 'suspended') await audioContext.resume().catch(() => {});
  } catch (error) {
    console.warn('No se pudieron precargar todos los medios de audio', error);
  }

  const candidates = ['audio/webm;codecs=opus', 'audio/webm'];
  const mimeType = candidates.find((type) => MediaRecorder.isTypeSupported?.(type)) || '';
  const recorder = new MediaRecorder(destination.stream, {
    ...(mimeType ? { mimeType } : {}),
    audioBitsPerSecond: audioBitrate,
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
  for (const track of destination.stream.getTracks()) track.stop();
  await audioContext.close().catch(() => {});

  if (controller.cancelled) return null;
  const blob = new Blob(chunks, { type: (mimeType.split(';')[0] || 'audio/webm') });
  return fixWebmDuration(blob, duration);
}

/** GIF89a real: renderiza cada instante y lo codifica con el codificador propio. */
async function recordGif({
  project,
  mediaMap,
  width,
  height,
  fps,
  startTime = 0,
  duration,
  onProgress,
  controller,
}) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { alpha: false, willReadFrequently: true });
  const pool = new MediaPool(mediaMap);

  try {
    await preloadProject(pool, project);
    await pool.prefetchMatting?.(project);
  } catch (error) {
    console.warn('No se pudieron precargar todos los medios del GIF', error);
  }

  const frameCount = Math.max(1, Math.ceil(duration * fps));
  const delay = Math.max(2, Math.round(100 / fps)); // centésimas de segundo
  const frames = [];

  try {
    for (let index = 0; index < frameCount; index += 1) {
      if (controller.cancelled) return null;
      const time = startTime + Math.min(duration, index / fps);
      await pool.prepare(project, time);
      renderFrame(ctx, width, height, project, pool, time);
      applyClipFilters(canvas, ctx, project, time);
      frames.push({
        data: ctx.getImageData(0, 0, width, height).data,
        delay,
      });
      const progress = (index + 1) / frameCount;
      onProgress?.(progress, index + 1, frameCount);
      // Deja respirar la interfaz para que la barra avance y Cancelar responda.
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
  } finally {
    pool.destroy();
  }

  if (controller.cancelled) return null;
  return encodeGif(frames, width, height);
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

const VIDEO_FPS = [24, 25, 29.97, 30, 48, 50, 59.94, 60, 120];
const GIF_FPS = [15, 24, 25, 30, 48, 50, 59.94, 60, 120, 240];

function optionsForFps(values, current, disabledAbove = Infinity) {
  return values.map((fps) => {
    const selected = Number(current) === fps ? ' selected' : '';
    const disabled = fps > disabledAbove ? ' disabled' : '';
    return `<option value="${fps}"${selected}${disabled}>${fps} fps</option>`;
  }).join('');
}

export function openExportDialog() {
  if (!state.project) return null;
  const total = projectDuration();
  if (total < 0.05) {
    toast('Añade material a la línea de tiempo antes de exportar', 'warning');
    return null;
  }

  const canRecord = Boolean(window.MediaRecorder);
  const project = state.project;
  const gifDuration = Math.min(60, Math.max(0.5, Math.round(total * 10) / 10));

  const resolutionOptions = [
    '<option value="same">Igual que el proyecto</option>',
    ...Object.keys(RESOLUTIONS).map((key) => {
      const selected = key === project.resolution ? ' selected' : '';
      const label = key === '480p' ? '480P · 852×480'
        : key === '720p' ? '720P · 1280×720'
          : key === '1080p' ? '1080P · 1920×1080'
            : key === '4K' ? '4K · 3840×2160' : `${key}`;
      return `<option value="${key}"${selected}>${label}</option>`;
    }),
  ].join('');

  const body = `
    <div class="export-tabs" role="tablist" aria-label="Formato de exportación">
      <button type="button" class="export-tab${canRecord ? ' active' : ''}" data-export-tab="video" role="tab" ${canRecord ? '' : 'disabled'}>${icon('video', 15)} Vídeo</button>
      <button type="button" class="export-tab" data-export-tab="audio" role="tab" ${canRecord ? '' : 'disabled'}>${icon('audio', 15)} Audio</button>
      <button type="button" class="export-tab${canRecord ? '' : ' active'}" data-export-tab="gif" role="tab">${icon('gif', 15)} GIF</button>
    </div>
    <p class="export-intro">Guarda lo que ves en la vista previa. Elige un tamaño: la forma de la imagen se mantiene.</p>

    <div class="field-row">
      <div class="field">
        <label>Nombre del archivo</label>
        <input class="input" data-export-name value="${escapeHtml(project.name || 'cupcraft')}" maxlength="70" />
      </div>
      <div class="field">
        <label>Formato</label>
        <select class="input" data-export-format disabled><option>WebM · VP9 + Opus</option></select>
      </div>
    </div>

    <section class="export-panel" data-export-panel="video" ${canRecord ? '' : 'hidden'}>
      <div class="field-row">
        <div class="field">
          <label>Reducir escala</label>
          <select class="input" data-export-resolution>${resolutionOptions}</select>
        </div>
        <div class="field">
          <label>Velocidad de fotogramas</label>
          <select class="input" data-export-fps>
            <option value="same">Igual que el proyecto (${project.fps || 30} fps)</option>
            ${optionsForFps(VIDEO_FPS, project.fps || 30)}
          </select>
        </div>
      </div>
      <details class="export-advanced">
        <summary>${icon('settings', 13)} Avanzado</summary>
        <div class="field-row">
          <div class="field">
            <label>Codificador de vídeo</label>
            <select class="input" data-export-codec>
              <optgroup label="Disponible en CUPCRAFT">
                <option value="vp9" selected>WebM · VP9 (real)</option>
                <option value="vp8">WebM · VP8 (real)</option>
              </optgroup>
              <optgroup label="Solo en CapCut · no disponible en WebView2" disabled>
                <option>H.264 (x264)</option><option>H.264 (NVIDIA)</option>
                <option>H.264 (Intel)</option><option>H.264 (AMD)</option>
                <option>H.264 10-bit (x264)</option><option>H.265 (x265)</option>
                <option>H.265 (NVIDIA)</option><option>H.265 10-bit (x265)</option>
              </optgroup>
            </select>
          </div>
          <div class="field">
            <label>Calidad constante</label>
            <select class="input" data-export-quality>
              <option value="baja">Baja · RF 28 · archivo ligero</option>
              <option value="media">Media · RF 23</option>
              <option value="alta" selected>Alta · RF 18</option>
              <option value="máxima">Máxima · RF 14</option>
            </select>
          </div>
        </div>
        <div class="field-row">
          <div class="field">
            <label>Ajuste predeterminado</label>
            <select class="input" data-export-preset disabled>
              <option>medium</option><option>superfast</option><option>veryfast</option>
              <option>faster</option><option>fast</option><option>slow</option>
              <option>slower</option><option>veryslow</option>
            </select>
          </div>
          <div class="field">
            <label>Codificador de audio</label>
            <select class="input" data-export-audio-codec>
              <option value="opus" selected>Opus (real)</option>
              <option value="aac" disabled>AAC · no disponible</option>
              <option value="mp3" disabled>MP3 · no disponible</option>
              <option value="ac3" disabled>AC3 · no disponible</option>
              <option value="flac" disabled>FLAC · no disponible</option>
            </select>
          </div>
        </div>
        <div class="field">
          <label>Tasa de bits de audio</label>
          <select class="input" data-export-audio-bitrate>
            <option value="96000">96 kbps</option><option value="128000" selected>128 kbps</option>
            <option value="160000">160 kbps</option><option value="192000">192 kbps</option>
            <option value="256000">256 kbps</option><option value="320000">320 kbps</option>
          </select>
        </div>
        <p class="export-warning">${icon('info', 12)} WebView2 solo puede grabar WebM con VP8/VP9 y Opus. Las opciones H.264, H.265, AAC, MP3, AC3 y FLAC se muestran como referencia, pero no están disponibles de verdad.</p>
      </details>
      <p class="modal-note">El resultado se descarga como <b>WebM</b>. La grabación es en tiempo real: tarda aproximadamente lo que dura el proyecto. El audio de vídeos y pistas de audio se incluye automáticamente.</p>
    </section>

    <section class="export-panel" data-export-panel="audio" hidden>
      <p class="export-lead">${icon('audio', 14)} Exporta únicamente la mezcla de audio de la línea de tiempo — sin pista de vídeo.</p>
      <div class="field">
        <label>Calidad de audio</label>
        <select class="input" data-export-audio-quality>
          <option value="96000">96 kbps · ligero</option>
          <option value="128000" selected>128 kbps · recomendada</option>
          <option value="192000">192 kbps</option>
          <option value="256000">256 kbps</option>
          <option value="320000">320 kbps · máxima</option>
        </select>
      </div>
      <details class="export-advanced">
        <summary>${icon('settings', 13)} Avanzado · Etiquetas</summary>
        <div class="export-tags">
          <div class="field"><label>Título</label><input class="input" data-tag-title placeholder="Título de la pista" /></div>
          <div class="field"><label>Artista</label><input class="input" data-tag-artist placeholder="Nombre del artista" /></div>
          <div class="field"><label>Álbum</label><input class="input" data-tag-album placeholder="Nombre del álbum" /></div>
          <div class="field"><label>Comentario</label><input class="input" data-tag-comment placeholder="Comentario" /></div>
        </div>
        <p class="export-warning">${icon('info', 12)} WebM no guarda estas etiquetas en WebView2. Se muestran para seguir el flujo de CapCut, pero no se incrustan en el archivo.</p>
      </details>
      <p class="modal-note">Exporta una mezcla real en <b>WebM/Opus</b> con los volúmenes, efectos de voz, fundidos y velocidad de la línea de tiempo.</p>
    </section>

    <section class="export-panel" data-export-panel="gif" ${canRecord ? 'hidden' : ''}>
      <p class="export-lead">${icon('gif', 14)} GIF animado — sin audio. Marca un área de trabajo para bucles cortos o exporta hasta 60 segundos.</p>
      <div class="field-row">
        <div class="field">
          <label>Área de trabajo · inicio (s)</label>
          <input class="input" type="number" data-gif-start min="0" max="${Math.max(0, total - 0.1).toFixed(1)}" step="0.1" value="0" />
        </div>
        <div class="field">
          <label>Duración (s, máx. 60)</label>
          <input class="input" type="number" data-gif-duration min="0.1" max="60" step="0.1" value="${gifDuration}" />
        </div>
      </div>
      <div class="field-row">
        <div class="field">
          <label>Reducir escala</label>
          <select class="input" data-gif-size>
            <option value="same">Igual que el proyecto</option>
            <option value="640">640 px · recomendado</option>
            <option value="480" selected>480 px</option>
            <option value="320">320 px · ligero</option>
          </select>
        </div>
        <div class="field">
          <label>Velocidad de fotogramas</label>
          <select class="input" data-gif-fps>
            <option value="15" selected>15 fps · recomendado</option>
            ${optionsForGifFps()}
          </select>
        </div>
      </div>
      <div class="field">
        <label>fps personalizado (1–60)</label>
        <input class="input" type="number" data-gif-custom-fps min="1" max="60" step="1" value="15" disabled />
      </div>
      <p class="modal-note">Se genera un <b>GIF89a real</b> con bucle infinito y 256 colores. El audio no se incluye. 120 y 240 fps se desactivan para evitar archivos inviables.</p>
    </section>

    <div class="export-progress hidden" data-export-progress>
      <div class="row"><span data-export-label>Preparando medios…</span><span data-export-percent>0%</span></div>
      <div class="progress"><div data-export-bar></div></div>
      <div class="row"><span data-export-info></span><span data-export-time></span></div>
    </div>`;

  let activeTab = canRecord ? 'video' : 'gif';
  let exporting = false;
  let controller = { cancelled: false };
  let modal = null;

  const api = openModal({
    title: 'Exportar',
    width: 560,
    dismissible: false,
    body,
    footer: `<button class="btn" data-export-cancel>Cancelar</button><button class="btn btn-primary" data-export-start>${icon('export', 14)} Exportar vídeo</button>`,
    onMount(instance) {
      modal = instance;
      const tabButtons = [...instance.body.querySelectorAll('[data-export-tab]')];
      const panels = [...instance.body.querySelectorAll('[data-export-panel]')];
      const formatSelect = instance.body.querySelector('[data-export-format]');
      const startButton = instance.foot.querySelector('[data-export-start]');
      const cancelButton = instance.foot.querySelector('[data-export-cancel]');
      const progress = instance.body.querySelector('[data-export-progress]');
      const bar = instance.body.querySelector('[data-export-bar]');
      const percent = instance.body.querySelector('[data-export-percent]');
      const label = instance.body.querySelector('[data-export-label]');
      const info = instance.body.querySelector('[data-export-info]');
      const timeLabel = instance.body.querySelector('[data-export-time]');
      const closeButton = instance.root.querySelector('[data-modal-close]');
      const gifFps = instance.body.querySelector('[data-gif-fps]');
      const gifCustomFps = instance.body.querySelector('[data-gif-custom-fps]');
      const presetSelect = instance.body.querySelector('[data-export-preset]');

      const startLabel = () => ({
        video: 'Exportar vídeo',
        audio: 'Exportar audio',
        gif: 'Exportar GIF',
      }[activeTab] || 'Exportar');

      const startIcon = () => (activeTab === 'gif' ? 'gif' : activeTab === 'audio' ? 'audio' : 'export');

      const updateFormat = () => {
        if (!formatSelect) return;
        let text = 'WebM · VP9 + Opus';
        if (activeTab === 'audio') text = 'WebM · Opus (sin vídeo)';
        else if (activeTab === 'gif') text = 'GIF animado · sin audio';
        else if (instance.body.querySelector('[data-export-codec]')?.value === 'vp8') text = 'WebM · VP8 + Opus';
        formatSelect.innerHTML = `<option>${text}</option>`;
      };

      const setTab = (tab) => {
        activeTab = tab;
        for (const button of tabButtons) button.classList.toggle('active', button.dataset.exportTab === tab);
        for (const panel of panels) panel.hidden = panel.dataset.exportPanel !== tab;
        startButton.innerHTML = `${icon(startIcon(), 14)} ${startLabel()}`;
        updateFormat();
      };

      const syncGifFps = () => {
        if (!gifCustomFps) return;
        const custom = gifFps?.value === 'custom';
        gifCustomFps.disabled = !custom;
        if (custom && !Number(gifCustomFps.value)) gifCustomFps.value = '15';
      };

      for (const button of tabButtons) {
        button.addEventListener('click', () => {
          if (!exporting) setTab(button.dataset.exportTab);
        });
      }
      instance.body.querySelector('[data-export-codec]')?.addEventListener('change', updateFormat);
      gifFps?.addEventListener('change', syncGifFps);
      setTab(activeTab);
      syncGifFps();

      const resetUi = () => {
        exporting = false;
        controller = { cancelled: false };
        startButton.disabled = false;
        startButton.innerHTML = `${icon(startIcon(), 14)} ${startLabel()}`;
        cancelButton.textContent = 'Cancelar';
        closeButton.style.display = '';
        for (const field of instance.body.querySelectorAll('input, select')) field.disabled = false;
        for (const button of tabButtons) button.disabled = false;
        formatSelect.disabled = true;
        if (presetSelect) presetSelect.disabled = true;
        for (const button of tabButtons) button.disabled = !canRecord && button.dataset.exportTab !== 'gif';
        syncGifFps();
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
        const name = instance.body.querySelector('[data-export-name]').value.trim() || 'cupcraft';
        let width = 0;
        let height = 0;
        let extension = 'webm';
        let infoText = '';
        let call;
        let args;

        try {
          if (activeTab === 'video') {
            if (!canRecord) throw new Error('Tu navegador no soporta la grabación de video');
            const resolutionKey = instance.body.querySelector('[data-export-resolution]').value;
            const fpsRaw = instance.body.querySelector('[data-export-fps]').value;
            const fps = fpsRaw === 'same' ? (project.fps || 30) : Number(fpsRaw);
            const quality = instance.body.querySelector('[data-export-quality]').value;
            const codec = instance.body.querySelector('[data-export-codec]').value;
            const audioBitrate = Number(instance.body.querySelector('[data-export-audio-bitrate]').value) || 128_000;
            ({ width, height } = exportDimensions(project, resolutionKey));
            const bitrate = bitrateFor(quality, width, height, fps);
            extension = 'webm';
            infoText = `WebM · ${codec.toUpperCase()} · ${width}×${height} · ${fps} fps`;
            call = recordProject;
            args = { project, mediaMap: state.media, width, height, fps, bitrate, codec, audioBitrate };
          } else if (activeTab === 'audio') {
            if (!canRecord) throw new Error('Tu navegador no soporta la grabación de audio');
            const audioBitrate = Number(instance.body.querySelector('[data-export-audio-quality]').value) || 128_000;
            extension = 'webm';
            infoText = `WebM · Opus · ${Math.round(audioBitrate / 1000)} kbps · sin vídeo`;
            call = recordAudioOnly;
            args = { project, mediaMap: state.media, audioBitrate };
          } else {
            const sizeRaw = instance.body.querySelector('[data-gif-size]').value;
            const fpsRaw = instance.body.querySelector('[data-gif-fps]').value;
            let fps = fpsRaw === 'custom'
              ? Number(gifCustomFps?.value)
              : Number(fpsRaw);
            fps = Math.min(60, Math.max(1, fps || 15));
            const start = Math.min(
              Math.max(0, Number(instance.body.querySelector('[data-gif-start]').value) || 0),
              Math.max(0, total - 0.1),
            );
            const gifLength = Math.min(
              60,
              Math.max(0.1, Number(instance.body.querySelector('[data-gif-duration]').value) || gifDuration),
              total - start,
            );
            ({ width, height } = exportDimensions(project, sizeRaw === 'same' ? 'same' : Number(sizeRaw)));
            extension = 'gif';
            infoText = `GIF · ${width}×${height} · ${fps} fps · ${gifLength.toFixed(1)} s`;
            call = recordGif;
            args = {
              project,
              mediaMap: state.media,
              width,
              height,
              fps,
              startTime: start,
              duration: gifLength,
            };
          }
        } catch (error) {
          label.textContent = error?.message || 'No se pudo preparar la exportación';
          toast('No se pudo preparar la exportación', 'error');
          return;
        }

        exporting = true;
        controller = { cancelled: false };
        startButton.disabled = true;
        startButton.innerHTML = `${icon('clock', 14)} Exportando…`;
        cancelButton.textContent = 'Cancelar exportación';
        closeButton.style.display = 'none';
        for (const field of instance.body.querySelectorAll('input, select')) field.disabled = true;
        for (const button of tabButtons) button.disabled = true;
        progress.classList.remove('hidden');
        bar.style.width = '0%';
        percent.textContent = '0%';
        label.textContent = 'Preparando medios…';
        info.textContent = infoText;
        timeLabel.textContent = activeTab === 'gif' ? `0 / ? fotogramas` : `00:00 / ${formatTime(total)}`;

        const verb = activeTab === 'gif' ? 'Generando GIF…'
          : activeTab === 'audio' ? 'Grabando audio…' : 'Grabando video…';

        try {
          const blob = await call({
            ...args,
            controller,
            onProgress(value, time, limit) {
              const pct = Math.round(Math.min(1, value) * 100);
              bar.style.width = `${pct}%`;
              percent.textContent = `${pct}%`;
              label.textContent = value >= 1 ? 'Finalizando archivo…' : verb;
              timeLabel.textContent = activeTab === 'gif'
                ? `${time} / ${limit} fotogramas`
                : `${formatTime(time)} / ${formatTime(limit)}`;
            },
          });
          if (!blob || controller.cancelled) {
            label.textContent = 'Exportación cancelada';
            resetUi();
            return;
          }
          downloadBlob(blob, `${sanitizeName(name)}.${extension}`);
          label.textContent = '¡Listo! Archivo descargado';
          percent.textContent = '100%';
          bar.style.width = '100%';
          toast(`${activeTab === 'gif' ? 'GIF' : activeTab === 'audio' ? 'Audio' : 'Video'} exportado correctamente`, 'success');
          setTimeout(() => {
            if (exporting) return;
            api.close();
          }, 900);
          resetUi();
          cancelButton.textContent = 'Cerrar';
        } catch (error) {
          console.error(error);
          label.textContent = error?.message || 'No se pudo exportar el archivo';
          toast('Error al exportar el archivo', 'error');
          resetUi();
        }
      });
    },
  });

  return api;
}

function optionsForGifFps() {
  return GIF_FPS.filter((fps) => fps !== 15).map((fps) => {
    const disabled = fps > 60 ? ' disabled' : '';
    return `<option value="${fps}"${disabled}>${fps} fps</option>`;
  }).join('') + '<option value="custom">Personalizado…</option>';
}
