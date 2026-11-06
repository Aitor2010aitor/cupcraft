import {
  state,
  newProject,
  updateProject,
  importExternalMedia,
  saveNow,
  seek,
  selectClip,
  setPanel,
} from './store.js';
import { icon } from './icons.js';
import { openModal } from './modal.js';
import { escapeHtml, formatTime, toast, uid } from './utils.js';

const RESOLUTION_FOR_HEIGHT = [
  { min: 1900, value: '4K' },
  { min: 1000, value: '1080p' },
  { min: 680, value: '720p' },
  { min: 0, value: '480p' },
];

function resolutionFor(width, height) {
  const short = Math.min(Number(width) || 1920, Number(height) || 1080);
  return RESOLUTION_FOR_HEIGHT.find((entry) => short >= entry.min)?.value || '1080p';
}

export async function listCapCutProjects() {
  const response = await fetch('/api/capcut/projects');
  if (!response.ok) throw new Error(`No se pudo leer CapCut (HTTP ${response.status})`);
  const data = await response.json();
  if (!data.ok) throw new Error(data.error || 'No se pudo leer CapCut');
  return { projects: data.projects || [], roots: data.roots || [] };
}

async function fetchMediaFile(media) {
  const response = await fetch(media.fileUrl);
  if (!response.ok) throw new Error(`No se pudo leer ${media.name || 'el archivo'} (HTTP ${response.status})`);
  const blob = await response.blob();
  return new File([blob], media.name || `archivo-${Date.now()}`, {
    type: media.mime || blob.type || 'application/octet-stream',
  });
}

function remapTracks(tracks, mapping, warnings) {
  let dropped = 0;
  const used = [];
  for (const track of tracks || []) {
    const clips = [];
    for (const clip of track.clips || []) {
      if (!clip.mediaId) {
        if (clip.kind === 'text' || clip.kind === 'sticker') {
          clips.push({ ...clip, id: uid('clip') });
        }
        continue;
      }
      const mediaId = mapping.get(clip.mediaId);
      const media = mediaId ? state.media.get(mediaId) : null;
      if (!media) {
        dropped += 1;
        continue;
      }
      let duration = Math.max(0.1, Number(clip.duration) || 1);
      if (media.kind !== 'image' && media.kind !== 'sticker') {
        const inPoint = Math.max(0, Number(clip.in) || 0);
        const speed = Math.max(0.25, Number(clip.speed) || 1);
        const sourceLeft = Math.max(0.1, (media.duration || duration) - inPoint) / speed;
        duration = Math.max(0.1, Math.min(duration, sourceLeft));
      }
      const next = {
        ...clip,
        id: uid('clip'),
        mediaId,
        kind: media.kind,
        name: media.name || clip.name || 'Clip',
        duration,
      };
      if (next.kind === 'image' || next.kind === 'sticker') next.in = 0;
      if (!next.adjust) next.adjust = { brightness: 100, contrast: 100, saturate: 100 };
      if (!next.filter) next.filter = 'none';
      clips.push(next);
    }
    if (clips.length) {
      used.push({
        id: uid('track'),
        type: track.type,
        name: track.name || (track.type === 'audio' ? 'Audio' : track.type === 'text' ? 'Texto' : 'Video'),
        clips,
      });
    }
  }
  if (dropped) warnings.push(`${dropped} clip${dropped === 1 ? '' : 's'} sin archivo se omitieron`);
  for (const [type, name] of [['text', 'Texto 1'], ['video', 'Video 1'], ['audio', 'Audio 1']]) {
    if (!used.some((track) => track.type === type)) {
      used.push({ id: uid('track'), type, name, clips: [] });
    }
  }
  return used;
}

export async function importCapCutProject(id, { onStatus } = {}) {
  const report = (message, progress) => {
    try { onStatus?.(message, progress); } catch { /* la interfaz no debe romper la importación */ }
  };
  report('Leyendo el borrador de CapCut…', 0.04);
  const response = await fetch('/api/capcut/import', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id }),
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok || !data.ok) throw new Error(data.error || 'No se pudo leer el borrador de CapCut');
  const raw = data.project;
  const warnings = [...(raw.warnings || [])];
  const mediaList = Array.isArray(raw.media) ? raw.media : [];

  report(`Creando “${raw.name || 'Proyecto de CapCut'}”…`, 0.08);
  await newProject(raw.name || 'Proyecto de CapCut');
  updateProject({
    ratio: raw.ratio || '16:9',
    resolution: resolutionFor(raw.width, raw.height),
    fps: Number(raw.fps) || 30,
    source: raw.source || { type: 'capcut' },
  });

  const mapping = new Map();
  let importedCount = 0;
  for (let index = 0; index < mediaList.length; index += 1) {
    const media = mediaList[index];
    const fraction = 0.1 + ((index + 1) / Math.max(1, mediaList.length)) * 0.72;
    report(`Importando ${media.name || 'archivo'} (${index + 1}/${mediaList.length})…`, fraction);
    try {
      const file = await fetchMediaFile(media);
      const meta = await importExternalMedia(file, { silent: true });
      mapping.set(media.capcutId, meta.id);
      importedCount += 1;
    } catch (error) {
      warnings.push(`${media.name || 'Archivo'}: ${error?.message || 'no se pudo importar'}`);
    }
  }

  report('Reconstruyendo la línea de tiempo…', 0.88);
  const tracks = remapTracks(raw.tracks, mapping, warnings);
  updateProject({
    tracks,
    ...(state.project?.thumb ? {} : {}),
    source: {
      ...(raw.source || {}),
      importedAt: Date.now(),
      mediaFound: importedCount,
      mediaTotal: mediaList.length,
    },
  });
  await saveNow();
  const firstClip = tracks.flatMap((track) => track.clips)[0];
  if (firstClip) selectClip(firstClip.id);
  seek(0);
  setPanel('media');
  report('Listo', 1);
  toast(
    importedCount
      ? `Proyecto de CapCut importado: ${importedCount} archivo${importedCount === 1 ? '' : 's'}`
      : 'Proyecto de CapCut importado',
    'success',
    3600,
  );
  return {
    id: state.project?.id,
    name: state.project?.name,
    tracks: tracks.map((track) => ({ id: track.id, type: track.type, clips: track.clips.length })),
    imported: importedCount,
    warnings,
  };
}

function projectCard(project, index) {
  const cover = project.coverUrl
    ? `<img src="${project.coverUrl}" alt="" loading="lazy" />`
    : `<span class="capcut-cover-fallback" style="background:linear-gradient(${135 + (index * 17) % 80}deg,#16414a,#0c171a)">${icon('film', 24)}</span>`;
  const counts = project.counts || {};
  const totalMedia = (counts.video || 0) + (counts.image || 0) + (counts.audio || 0);
  const empty = project.clips === 0;
  return `
    <article class="capcut-card${empty ? ' empty' : ''}" data-capcut-id="${project.id}">
      <div class="capcut-cover">
        ${cover}
        <span class="capcut-ratio">${project.ratio || '16:9'}</span>
        ${empty ? '<span class="capcut-empty-badge">Vacío</span>' : ''}
      </div>
      <div class="capcut-info">
        <strong title="${escapeHtml(project.name)}">${escapeHtml(project.name)}</strong>
        <span>${formatTime(project.duration || 0)} · ${project.clips || 0} clip${project.clips === 1 ? '' : 's'} · ${totalMedia} archivo${totalMedia === 1 ? '' : 's'}</span>
        <small>${project.width || 1920} × ${project.height || 1080} · ${project.fps || 30} fps</small>
      </div>
      <button class="btn btn-small capcut-open" data-capcut-open="${project.id}">${icon('folderOpen', 13)} Abrir</button>
    </article>`;
}

export function openCapCutImportDialog() {
  let listData = { projects: [], roots: [] };
  const modal = openModal({
    title: 'Proyectos de CapCut',
    width: 760,
    className: 'capcut-modal',
    body: `<div class="bridge-loading">${icon('folderOpen', 22)} Buscando borradores de CapCut…</div>`,
    footer: `<button class="btn" data-capcut-refresh>${icon('rotate', 14)} Volver a buscar</button><button class="btn btn-primary" data-capcut-close>Listo</button>`,
    onMount(api) {
      const body = api.body;
      let busy = false;

      const renderList = () => {
        const projects = listData.projects || [];
        const roots = listData.roots || [];
        const folder = roots[0] || '';
        body.innerHTML = `
          <div class="capcut-intro">
            <div>${icon('folderOpen', 18)}<span>Borradores detectados en <code>${escapeHtml(folder || 'CapCut')}</code></span></div>
            <span class="capcut-count">${projects.length} proyecto${projects.length === 1 ? '' : 's'}</span>
          </div>
          ${projects.length
            ? `<div class="capcut-list">${projects.map(projectCard).join('')}</div>`
            : `<div class="capcut-empty">
                <strong>No hay proyectos de CapCut</strong>
                Abre CapCut, crea un borrador y vuelve a buscar. También puedes indicar otra carpeta con la variable <code>openCUP_CAPCUT_ROOT</code> antes de iniciar el servidor.
              </div>`}
          <div class="capcut-note">${icon('info', 13)} cupcraft lee <code>draft_content.json</code> y copia los medios a su almacenamiento local. Los archivos originales de CapCut no se modifican.</div>`;
        if (!projects.length) return;
        body.querySelectorAll('[data-capcut-open]').forEach((button) => {
          button.addEventListener('click', () => {
            const project = projects.find((item) => item.id === button.dataset.capcutOpen);
            if (project) void runImport(project);
          });
        });
      };

      const load = async () => {
        if (busy) return;
        busy = true;
        body.innerHTML = `<div class="bridge-loading">${icon('folderOpen', 22)} Buscando borradores de CapCut…</div>`;
        try {
          listData = await listCapCutProjects();
          renderList();
        } catch (error) {
          body.innerHTML = `<div class="bridge-offline">${icon('alert', 22)}<strong>No se pudo leer CapCut</strong><span>${escapeHtml(error?.message || 'Error desconocido')}</span></div>`;
        } finally {
          busy = false;
        }
      };

      const runImport = async (project) => {
        if (busy) return;
        busy = true;
        body.innerHTML = `
          <div class="capcut-progress">
            <div class="capcut-progress-head">${icon('import', 20)}<div><strong>Importando “${escapeHtml(project.name)}”</strong><span data-capcut-status>Preparando…</span></div></div>
            <div class="capcut-bar"><i data-capcut-bar style="width:4%"></i></div>
            <div class="capcut-progress-meta"><span>${project.clips || 0} clips</span><span>${project.width || 1920} × ${project.height || 1080}</span><span>${project.ratio || '16:9'}</span></div>
          </div>`;
        const status = body.querySelector('[data-capcut-status]');
        const bar = body.querySelector('[data-capcut-bar]');
        try {
          const result = await importCapCutProject(project.id, {
            onStatus(message, progress) {
              if (status) status.textContent = message;
              if (bar) bar.style.width = `${Math.round((progress || 0) * 100)}%`;
            },
          });
          api.close();
          if (result.warnings?.length) {
            setTimeout(() => {
              toast(`Importado con ${result.warnings.length} aviso${result.warnings.length === 1 ? '' : 's'}: ${result.warnings[0]}`, 'warning', 5200);
            }, 500);
          }
        } catch (error) {
          console.error(error);
          body.innerHTML = `<div class="bridge-offline">${icon('alert', 22)}<strong>La importación falló</strong><span>${escapeHtml(error?.message || 'Error desconocido')}</span></div>`;
          busy = false;
        }
      };

      api.foot.querySelector('[data-capcut-close]').addEventListener('click', api.close);
      api.foot.querySelector('[data-capcut-refresh]').addEventListener('click', load);
      void load();
    },
  });
  return modal;
}
