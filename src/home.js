import {
  state,
  on,
  newProject,
  openProject,
  deleteProject,
  renameProject,
  importFiles,
  updateProject,
  addTextClip,
  setPanel,
  seek,
  exportProjectFile,
  importProjectFile,
} from './store.js';
import { icon, logo } from './icons.js';
import { openModal, confirmDialog, promptDialog } from './modal.js';
import { openShortcutsDialog, openAboutDialog } from './dialogs.js';
import { TEMPLATES } from './templates.js';
import { openCapCutImportDialog } from './capcut.js';
import { openAiBridgeDialog } from './ai-bridge.js';
import { escapeHtml, formatTime, toast } from './utils.js';

const SHOWCASES = [
  {
    id: 'studio',
    title: 'Estudio de video',
    subtitle: 'Crea videos de 30 segundos con IA',
    art: 'linear-gradient(120deg,#0d5b6b 0%,#1fa7b5 45%,#8fe7ff 58%,#33506b 100%)',
    mark: icon('studio', 42),
    caption: 'Genera un clip de 30 segundos',
  },
  {
    id: 'design',
    title: 'Estudio de diseño',
    subtitle: 'Convierte en realidad lo que imaginas',
    art: 'linear-gradient(120deg,#111f55 0%,#3246b8 48%,#8db8ff 52%,#111c44 100%)',
    mark: '<span class="art-word">Aa</span>',
    caption: 'Diseña con texto e imágenes',
  },
  {
    id: 'ads',
    title: 'Creador de anuncios con IA',
    subtitle: 'Crea anuncios ganadores y escala rápido',
    art: 'linear-gradient(125deg,#1c2050 0%,#454fc0 40%,#a3b7ff 58%,#232552 100%)',
    mark: icon('sparkles', 40),
    caption: 'Anuncios que destacan',
  },
  {
    id: 'shorts',
    title: 'De videos largos a cortos',
    subtitle: 'Convierte videos largos en clips cortos',
    art: 'linear-gradient(120deg,#0b5570 0%,#1498b4 42%,#7fe8f0 55%,#2d5e7c 100%)',
    mark: icon('scissors', 38),
    caption: 'Cortes verticales en segundos',
  },
  {
    id: 'templates',
    title: 'Plantillas destacadas',
    subtitle: 'Composiciones listas para tu historia',
    art: 'linear-gradient(120deg,#2a163e 0%,#7b3fa8 46%,#f0a6ff 60%,#2d1a44 100%)',
    mark: '<span class="art-word">openCUP</span>',
    caption: 'Empieza con una plantilla',
  },
];

const THUMB_GRADIENTS = [
  'linear-gradient(135deg,#1a1a1a,#111111)',
  'linear-gradient(135deg,#1b1d22,#111214)',
  'linear-gradient(135deg,#1f1b18,#121110)',
  'linear-gradient(135deg,#171d24,#0f1216)',
];

function timeAgo(timestamp) {
  if (!timestamp) return 'hace un momento';
  const seconds = Math.max(0, (Date.now() - timestamp) / 1000);
  if (seconds < 60) return 'hace un momento';
  if (seconds < 3600) return `hace ${Math.floor(seconds / 60)} min`;
  if (seconds < 86400) return `hace ${Math.floor(seconds / 3600)} h`;
  if (seconds < 604800) return `hace ${Math.floor(seconds / 86400)} d`;
  return new Date(timestamp).toLocaleDateString('es', { day: 'numeric', month: 'short' });
}

function durationOf(project) {
  let end = 0;
  for (const track of project.tracks || []) {
    for (const clip of track.clips || []) end = Math.max(end, clip.start + clip.duration);
  }
  return end;
}

function projectCard(project, index) {
  const duration = durationOf(project);
  const thumb = project.thumb
    ? `<img src="${project.thumb}" alt="" />`
    : `<span style="display:flex;align-items:center;justify-content:center;color:#3a3a3a">${icon('film', 26)}</span>`;
  return `
    <article class="project-card" data-project-id="${project.id}" tabindex="0" role="button" aria-label="Abrir ${escapeHtml(project.name)}">
      <div class="project-thumb" style="background:${THUMB_GRADIENTS[index % THUMB_GRADIENTS.length]}">
        ${thumb}
        ${duration > 0 ? `<span class="project-duration">${formatTime(duration)}</span>` : ''}
      </div>
      <div class="card-actions">
        <button class="icon-btn" data-project-rename="${project.id}" title="Renombrar">${icon('pencil', 13)}</button>
        <button class="icon-btn" data-project-export="${project.id}" title="Guardar proyecto como archivo">${icon('download', 13)}</button>
        <button class="icon-btn" data-project-delete="${project.id}" title="Eliminar">${icon('trash', 13)}</button>
      </div>
      <div class="project-info">
        <div class="project-name">${escapeHtml(project.name)}</div>
        <div class="project-meta">
          <span>${project.resolution || '1080p'}</span>
          <span>${formatTime(duration)}</span>
          <span>${timeAgo(project.updatedAt)}</span>
        </div>
      </div>
    </article>`;
}

function emptyProjects() {
  return `
    <div class="empty-projects">
      <strong>Aún no hay proyectos</strong>
      Crea tu primer video o importa un archivo para empezar.
      <div><button class="btn btn-primary" data-home-action="new">${icon('plus', 14)} Crear proyecto</button></div>
    </div>`;
}

function showcaseMarkup(item) {
  return `
    <button class="showcase-card" data-showcase="${item.id}">
      <div class="showcase-art" style="background:${item.art}">
        <span class="art-glyph">${item.mark}</span>
        ${item.caption ? `<span class="art-caption">${item.caption}</span>` : ''}
      </div>
      <div class="showcase-info">
        <strong>${item.title}</strong>
        <span>${item.subtitle}</span>
      </div>
    </button>`;
}

function openNewProjectDialog() {
  const modal = openModal({
    title: 'Crear proyecto',
    width: 500,
    body: `
      <div class="field">
        <label>Nombre del proyecto</label>
        <input class="input" data-new-name value="Proyecto sin título" maxlength="80" />
      </div>
      <div class="field-row">
        <div class="field">
          <label>Relación de aspecto</label>
          <select class="input" data-new-ratio>
            ${['16:9', '9:16', '1:1', '4:3', '21:9', '4:5'].map((ratio) => `<option>${ratio}</option>`).join('')}
          </select>
        </div>
        <div class="field">
          <label>Resolución</label>
          <select class="input" data-new-resolution>
            ${['480p', '720p', '1080p', '4K'].map((key) => `<option ${key === '1080p' ? 'selected' : ''}>${key}</option>`).join('')}
          </select>
        </div>
      </div>
      <div class="field">
        <label>Fotogramas por segundo</label>
        <select class="input" data-new-fps>
          ${[24, 30, 60].map((fps) => `<option value="${fps}" ${fps === 30 ? 'selected' : ''}>${fps} fps</option>`).join('')}
        </select>
      </div>
      <div class="modal-note">
        El proyecto se guarda automáticamente en este navegador. Podrás importar videos,
        imágenes y audio desde el editor.
      </div>`,
    footer: `<button class="btn" data-new-cancel>Cancelar</button><button class="btn btn-primary" data-new-create>${icon('plus', 14)} Crear proyecto</button>`,
    onMount(instance) {
      const nameInput = instance.body.querySelector('[data-new-name]');
      nameInput.focus();
      nameInput.select();
      const create = async () => {
        const name = nameInput.value.trim() || 'Proyecto sin título';
        const ratio = instance.body.querySelector('[data-new-ratio]').value;
        const resolution = instance.body.querySelector('[data-new-resolution]').value;
        const fps = Number(instance.body.querySelector('[data-new-fps]').value) || 30;
        instance.close();
        await newProject(name);
        updateProject({ ratio, resolution, fps });
        toast('Proyecto creado', 'success', 1800);
      };
      instance.foot.querySelector('[data-new-create]').addEventListener('click', create);
      instance.foot.querySelector('[data-new-cancel]').addEventListener('click', instance.close);
      nameInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') create();
      });
    },
  });
  return modal;
}

function openTemplatesModal() {
  const modal = openModal({
    title: 'Plantillas',
    width: 620,
    body: `
      <div class="template-grid">
        ${TEMPLATES.map((template) => `
          <button class="template-card" data-home-template="${template.id}">
            <div class="template-art" style="background:${template.art};display:flex;flex-direction:column;align-items:center;justify-content:center;gap:7px;color:rgba(255,255,255,.94);font-weight:900;font-size:18px;text-align:center;padding:14px">
              <span>${escapeHtml(template.content.replace('\n', ' '))}</span>
              <small style="font-size:9px;font-weight:500;color:${template.accent || '#fff'};opacity:.9">${escapeHtml(template.subtitle || '')}</small>
            </div>
            <span>${template.name} · ${template.ratio}</span>
          </button>`).join('')}
      </div>
      <div class="panel-empty" style="padding-top:4px">Elige una plantilla para crear un proyecto nuevo con una composición inicial. Después podrás añadir tus medios.</div>`,
    footer: `<button class="btn" data-templates-close>Cerrar</button>`,
    onMount(instance) {
      instance.foot.querySelector('[data-templates-close]').addEventListener('click', instance.close);
      instance.body.addEventListener('click', async (event) => {
        const button = event.target.closest('[data-home-template]');
        if (!button) return;
        const template = TEMPLATES.find((item) => item.id === button.dataset.homeTemplate);
        if (!template) return;
        instance.close();
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
        setPanel('text');
        toast('Plantilla aplicada. Añade tus medios para completarla.', 'success', 3600);
      });
    },
  });
  return modal;
}

function openSettingsModal() {
  openModal({
    title: 'Ajustes',
    width: 480,
    body: `
      <div class="details-section" style="margin-top:0">Almacenamiento</div>
      <div class="detail-item"><span class="d-label">Modo</span><span class="d-value">Local · sin cuenta</span></div>
      <div class="detail-item"><span class="d-label">Proyectos</span><span class="d-value">${state.projects.length}</span></div>
      <div class="detail-item"><span class="d-label">Espacio usado</span><span class="d-value" data-storage-size>Calculando…</span></div>
      <div class="details-sep"></div>
      <div class="details-section">Preferencias</div>
      <div class="detail-item"><span class="d-label">Tema</span><span class="d-value">Oscuro</span></div>
      <div class="detail-item"><span class="d-label">Idioma</span><span class="d-value">Español</span></div>
      <div class="details-sep"></div>
      <div class="details-section">Integraciones</div>
      <div class="detail-item"><span class="d-label">Proyectos de CapCut</span><button class="btn btn-small" data-settings-capcut>${icon('folderOpen', 13)} Abrir</button></div>
      <div class="detail-item"><span class="d-label">Puente IA</span><button class="btn btn-small" data-settings-ai>${icon('link', 13)} Ver puerto</button></div>
      <div class="modal-note" style="margin-top:10px">
        Tus proyectos y medios viven en el almacenamiento local de este navegador. Si borras los datos del sitio, también se borrarán.
      </div>`,
    footer: `<button class="btn" data-settings-shortcuts>${icon('keyboard', 14)} Atajos</button><button class="btn btn-primary" data-settings-close>Listo</button>`,
    onMount(instance) {
      instance.foot.querySelector('[data-settings-close]').addEventListener('click', instance.close);
      instance.foot.querySelector('[data-settings-shortcuts]').addEventListener('click', openShortcutsDialog);
      instance.body.querySelector('[data-settings-capcut]')?.addEventListener('click', () => {
        instance.close();
        openCapCutImportDialog();
      });
      instance.body.querySelector('[data-settings-ai]')?.addEventListener('click', () => {
        instance.close();
        void openAiBridgeDialog();
      });
      navigator.storage?.estimate?.().then((estimate) => {
        const target = instance.body.querySelector('[data-storage-size]');
        if (target) target.textContent = estimate?.usage ? `${(estimate.usage / 1024 / 1024).toFixed(1)} MB` : 'No disponible';
      }).catch(() => {});
    },
  });
}

function openProModal() {
  openModal({
    title: 'openCUP Pro',
    width: 440,
    body: `
      <div style="font-size:12px;line-height:1.6;color:#c7c7c7;margin-bottom:12px">
        openCUP es un editor local y gratuito. No hay cuentas, anuncios ni límites de exportación.
      </div>
      <div class="modal-note">
        Todas las funciones de esta versión están disponibles sin suscripción. Tus archivos nunca salen de tu equipo.
      </div>`,
    footer: `<button class="btn btn-primary" data-pro-close>Genial</button>`,
    onMount(instance) {
      instance.foot.querySelector('[data-pro-close]').addEventListener('click', instance.close);
    },
  });
}

function openNotificationsModal() {
  openModal({
    title: 'Notificaciones',
    width: 420,
    body: `<div class="panel-empty" style="padding:28px 14px">${icon('bell', 30)}<strong>Estás al día</strong>No hay notificaciones nuevas. openCUP funciona sin conexión.</div>`,
    footer: `<button class="btn btn-primary" data-notifications-close>Cerrar</button>`,
    onMount(instance) {
      instance.foot.querySelector('[data-notifications-close]').addEventListener('click', instance.close);
    },
  });
}

function openUnavailable(title, message) {
  openModal({
    title,
    width: 440,
    body: `<div class="modal-note" style="margin:0">${escapeHtml(message)}</div>`,
    footer: `<button class="btn btn-primary" data-unavailable-close>Entendido</button>`,
    onMount(instance) {
      instance.foot.querySelector('[data-unavailable-close]').addEventListener('click', instance.close);
    },
  });
}

async function quickStart(kind) {
  if (kind === 'studio' || kind === 'ads') {
    openNewProjectDialog();
    return;
  }
  if (kind === 'design') {
    openTemplatesModal();
    return;
  }
  if (kind === 'shorts') {
    await newProject('Video corto');
    updateProject({ ratio: '9:16', resolution: '1080p' });
    toast('Proyecto vertical 9:16 listo. Importa tu video para recortarlo.', 'success', 3600);
    return;
  }
  if (kind === 'templates') {
    openTemplatesModal();
  }
}

export function mountHome(root) {
  document.title = 'openCUP — Editor de video';
  const home = document.createElement('div');
  home.className = 'home';
  home.innerHTML = `
    <aside class="home-sidebar">
      <button class="home-brand" data-home-action="top">${logo}<span>openCUP</span></button>
      <div class="side-user">
        <div class="side-user-row">
          <div class="avatar">${logo}</div>
          <div class="user-meta"><strong>Iniciar sesión</strong></div>
        </div>
        <button class="btn-gradient" data-home-action="pro">Unirse a Pro</button>
      </div>
      <nav class="side-nav">
        <button class="side-item active" data-home-action="top">${icon('home', 16)}<span>Inicio</span></button>
        <button class="side-item" data-home-action="templates">${icon('templates', 16)}<span>Plantillas</span></button>
        <div class="side-sep"></div>
        <div class="side-label">Crear con IA</div>
        <button class="side-item" data-home-action="new">${icon('studio', 16)}<span>Estudio de video</span></button>
        <button class="side-item" data-home-action="templates">${icon('design', 16)}<span>Estudio de diseño</span></button>
        <div class="side-sub">
          <button class="side-item" data-home-action="more-tools">${icon('grid', 16)}<span>Más herramientas</span>${icon('chevronDown', 12)}</button>
          <div class="side-sub" data-more-tools>
            <button class="side-item" data-home-action="editpilot">${icon('sparkles', 15)}<span>Crear con IA</span></button>
            <button class="side-item" data-home-action="ai-tools">${icon('wand', 15)}<span>Herramientas de edición</span></button>
          </div>
        </div>
        <div class="side-sep"></div>
        <div class="side-label">Administración</div>
        <button class="side-item" data-home-action="library">${icon('library', 16)}<span>Biblioteca</span></button>
        <button class="side-item" data-home-action="capcut">${icon('folderOpen', 16)}<span>Proyectos de CapCut</span></button>
        <button class="side-item side-ai" data-home-action="ai-bridge">${icon('link', 16)}<span>Puente IA</span><span class="side-live-dot" title="Puerto local para IA"></span></button>
        <a class="side-item" href="https://github.com/Aitor2010aitor/openCUP" target="_blank" rel="noopener" title="Repositorio en GitHub">${icon('github', 16)}<span>GitHub</span></a>
      </nav>
      <div class="side-foot">
        <button class="invite-card" data-home-action="invite">${icon('user', 15)}<span style="flex:1">Invitar a amigos</span><span class="invite-new">NUEVO</span></button>
        <button class="promo-card" data-home-action="pro">
          <strong>¿Quieres alargarlo?</strong>
          <span>Amplía tu video con IA</span>
        </button>
      </div>
    </aside>
    <main class="home-main" data-home-main>
      <div class="home-toolbar">
        <button class="icon-btn" data-home-action="shortcuts" title="Ayuda y atajos">${icon('info', 16)}</button>
        <button class="icon-btn" data-home-action="ai-bridge" title="Puente IA">${icon('link', 16)}</button>
        <button class="icon-btn" data-home-action="capcut" title="Abrir proyecto de CapCut">${icon('folderOpen', 16)}</button>
        <button class="icon-btn" data-home-action="notifications" title="Novedades">${icon('bell', 16)}</button>
        <button class="icon-btn" data-home-action="settings" title="Ajustes">${icon('settings', 16)}</button>
        <span class="tl-sep" style="height:14px;background:#333;margin:0 5px"></span>
        <button class="icon-btn" data-home-action="minimize" title="Minimizar">${icon('minus', 15)}</button>
        <button class="icon-btn" data-home-action="maximize" title="Pantalla completa">${icon('square', 13)}</button>
        <button class="icon-btn" data-home-action="close" title="Cerrar">${icon('x', 15)}</button>
      </div>
      <div class="home-inner">
        <button class="hero-create" data-home-action="new">
          <span class="hero-plus">${icon('plus', 18)}</span>
          Crear proyecto
        </button>
        <div class="home-grid">
          <div class="ai-card ai-editpilot">
            <div class="ai-head">${icon('sparkles', 14)} EditPilot</div>
            <h3>¿Qué puedo ayudarte a editar hoy?</h3>
            <form class="ai-input" data-ai-form>
              <input placeholder="Describe lo que quieres editar" data-ai-input maxlength="180" aria-label="Describe tu proyecto" />
              <button class="ai-send" title="Crear borrador">${icon('chevronRight', 14)}</button>
            </form>
          </div>
          <button class="ai-card" data-home-action="ai-image">
            <div class="ai-icon">${icon('image', 17)}</div>
            <h3>Imagen de IA</h3>
            <p>Convierte ideas en imágenes</p>
          </button>
          <button class="ai-card" data-home-action="ai-expand">
            <div class="ai-icon">${icon('fullscreen', 17)}</div>
            <h3>Ampliar video</h3>
            <p>Amplía fácilmente cualquier clip</p>
          </button>
          <button class="ai-card" data-home-action="ai-edit">
            <div class="ai-icon">${icon('scissors', 17)}</div>
            <h3>Editar con IA</h3>
            <p>Edita clips describiéndolos</p>
          </button>
        </div>
        <section class="home-section" data-projects-section>
          <div class="section-head">
            <h2>Proyectos</h2>
            <div class="projects-toolbar">
              <input class="projects-search hidden" data-projects-search-input placeholder="Buscar proyectos" aria-label="Buscar proyectos" />
              <button class="capcut-pill" data-home-action="capcut" title="Importar un proyecto de CapCut">${icon('folderOpen', 14)} CapCut</button>
              <button class="capcut-pill" data-home-action="openProjectFile" title="Abrir un proyecto guardado como archivo">${icon('download', 14)} Abrir archivo</button>
              <button class="icon-btn" data-projects-search title="Buscar">${icon('search', 15)}</button>
              <button class="icon-btn" data-home-action="projects" title="Ver todos los proyectos">${icon('grid', 15)}${icon('chevronDown', 9)}</button>
              <button class="icon-btn" data-projects-trash title="Papelera">${icon('trash', 15)}</button>
              <button class="icon-btn" data-projects-filter title="Ordenar proyectos">${icon('filter', 15)}</button>
            </div>
          </div>
          <div class="projects-grid" data-projects-grid></div>
        </section>
      </div>
    </main>`;
  root.append(home);

  const main = home.querySelector('[data-home-main]');
  const grid = home.querySelector('[data-projects-grid]');
  const showcaseGrid = home.querySelector('[data-showcase-grid]');
  const searchButton = home.querySelector('[data-projects-search]');
  const searchInput = home.querySelector('[data-projects-search-input]');
  const aiForm = home.querySelector('[data-ai-form]');
  const aiInput = home.querySelector('[data-ai-input]');
  const moreToolsButton = home.querySelector('[data-more-tools]');
  const moreToolsPanel = home.querySelector('[data-more-tools]');
  const fileInput = document.createElement('input');
  fileInput.type = 'file';
  fileInput.className = 'hidden';
  fileInput.multiple = true;
  fileInput.accept = 'video/*,image/*,audio/*,.mkv,.mov,.m4v,.webm,.mp3,.wav,.m4a,.aac,.ogg';
  home.append(fileInput);

  const projectFileInput = document.createElement('input');
  projectFileInput.type = 'file';
  projectFileInput.className = 'hidden';
  projectFileInput.accept = '.json,.opencup,application/json';
  home.append(projectFileInput);
  let sortMode = 'recent';
  let searchQuery = '';
  let showcaseOffset = 0;
  const subscriptions = [];

  function sortedProjects() {
    const query = searchQuery.trim().toLowerCase();
    const list = state.projects.filter((project) => !query || String(project.name).toLowerCase().includes(query));
    if (sortMode === 'oldest') list.sort((a, b) => (a.updatedAt || 0) - (b.updatedAt || 0));
    else if (sortMode === 'name') list.sort((a, b) => String(a.name).localeCompare(String(b.name), 'es'));
    else list.sort((a, b) => (b.updatedAt || 0) - (a.updatedAt || 0));
    return list;
  }

  function renderProjects() {
    const list = sortedProjects();
    grid.innerHTML = list.length ? list.map(projectCard).join('') : emptyProjects();
  }

  function renderShowcase() {
    if (!showcaseGrid) return;
    const ordered = SHOWCASES.slice(showcaseOffset).concat(SHOWCASES.slice(0, showcaseOffset)).slice(0, 4);
    showcaseGrid.innerHTML = ordered.map(showcaseMarkup).join('') + `<button class="showcase-nav" data-showcase-next title="Más ideas">${icon('chevronRight', 15)}</button>`;
  }

  async function openProjectCard(id) {
    try {
      await openProject(id);
    } catch (error) {
      console.error(error);
      toast('No se pudo abrir el proyecto', 'error');
    }
  }

  async function handleRename(id) {
    const project = state.projects.find((item) => item.id === id);
    if (!project) return;
    const name = await promptDialog({
      title: 'Renombrar proyecto',
      label: 'Nombre del proyecto',
      value: project.name,
      confirmLabel: 'Guardar',
    });
    if (name && name !== project.name) await renameProject(id, name);
  }

  async function handleDelete(id) {
    const project = state.projects.find((item) => item.id === id);
    if (!project) return;
    const ok = await confirmDialog({
      title: 'Eliminar proyecto',
      message: `Se eliminará “${project.name}” y sus medios de este navegador. Esta acción no se puede deshacer.`,
      confirmLabel: 'Eliminar',
      danger: true,
    });
    if (!ok) return;
    await deleteProject(id);
    toast('Proyecto eliminado', 'success', 2000);
  }

  async function handleExportFile(id) {
    try {
      const isCurrent = state.project?.id === id;
      const source = isCurrent ? null : state.projects.find((item) => item.id === id);
      if (!isCurrent && !source) return;
      const result = await exportProjectFile(source);
      const name = (isCurrent ? state.project?.name : source?.name) || 'Proyecto';
      const note = result.skipped
        ? ` · ${result.skipped} medio(s) grandes quedan solo en este navegador`
        : ` · ${result.embedded} medio(s) incrustados`;
      toast(`“${name}” guardado como archivo${note}`, 'success', 4200);
    } catch (error) {
      toast(error?.message || 'No se pudo guardar el proyecto', 'error', 3800);
    }
  }

  function focusEditPilot() {
    main.scrollTo({ top: 0, behavior: 'smooth' });
    setTimeout(() => aiInput.focus(), 260);
  }

  home.addEventListener('click', (event) => {
    const showcase = event.target.closest('[data-showcase]');
    if (showcase && showcaseGrid) {
      void quickStart(showcase.dataset.showcase);
      return;
    }
    if (event.target.closest('[data-showcase-next]') && showcaseGrid) {
      showcaseOffset = (showcaseOffset + 1) % SHOWCASES.length;
      renderShowcase();
      return;
    }
    if (event.target.closest('[data-projects-search]')) {
      searchInput.classList.toggle('hidden');
      if (!searchInput.classList.contains('hidden')) searchInput.focus();
      else { searchQuery = ''; renderProjects(); }
      return;
    }
    if (event.target.closest('[data-projects-trash]')) {
      openUnavailable('Papelera', 'La papelera está vacía. Los proyectos que elimines se borran definitivamente de este navegador.');
      return;
    }
    if (event.target.closest('[data-projects-filter]')) {
      sortMode = sortMode === 'recent' ? 'name' : sortMode === 'name' ? 'oldest' : 'recent';
      const label = sortMode === 'name' ? 'Nombre' : sortMode === 'oldest' ? 'Más antiguos' : 'Más recientes';
      toast(`Proyectos ordenados por: ${label}`, 'info', 1500);
      renderProjects();
      return;
    }
    const actionEl = event.target.closest('[data-home-action]');
    if (!actionEl) return;
    const action = actionEl.dataset.homeAction;
    if (action === 'top') main.scrollTo({ top: 0, behavior: 'smooth' });
    else if (action === 'projects') main.querySelector('[data-projects-section]')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    else if (action === 'templates') openTemplatesModal();
    else if (action === 'new') openNewProjectDialog();
    else if (action === 'import') fileInput.click();
    else if (action === 'openProjectFile') projectFileInput.click();
    else if (action === 'capcut') openCapCutImportDialog();
    else if (action === 'ai-bridge') void openAiBridgeDialog();
    else if (action === 'library') openUnavailable('Biblioteca', 'Abre un proyecto para ver su biblioteca de medios local. No hay contenido en la nube.');
    else if (action === 'invite') openUnavailable('Invitar a amigos', 'openCUP funciona sin cuentas. Comparte el proyecto exportado como archivo de video para colaborar.');
    else if (action === 'more-tools') moreToolsPanel.classList.toggle('hidden');
    else if (action === 'editpilot') focusEditPilot();
    else if (action === 'ai-tools' || action === 'ai-image' || action === 'ai-expand') {
      openUnavailable('Funciones de IA', 'La generación y el procesamiento con IA necesitan un servicio en la nube. Esta versión local incluye EditPilot como borrador de texto y todas las herramientas de edición manual.');
    } else if (action === 'ai-edit') focusEditPilot();
    else if (action === 'settings' || action === 'storage') openSettingsModal();
    else if (action === 'pro') openProModal();
    else if (action === 'shortcuts') openShortcutsDialog();
    else if (action === 'about') openAboutDialog();
    else if (action === 'notifications') openNotificationsModal();
    else if (action === 'minimize' || action === 'close') toast('Usa los controles de la ventana del navegador para cerrar o minimizar', 'info', 2600);
    else if (action === 'maximize') {
      if (document.fullscreenElement) document.exitFullscreen();
      else document.documentElement.requestFullscreen?.().catch(() => {});
    }
  });

  grid.addEventListener('click', (event) => {
    const rename = event.target.closest('[data-project-rename]');
    if (rename) {
      event.stopPropagation();
      void handleRename(rename.dataset.projectRename);
      return;
    }
    const remove = event.target.closest('[data-project-delete]');
    if (remove) {
      event.stopPropagation();
      void handleDelete(remove.dataset.projectDelete);
      return;
    }
    const share = event.target.closest('[data-project-export]');
    if (share) {
      event.stopPropagation();
      void handleExportFile(share.dataset.projectExport);
      return;
    }
    const card = event.target.closest('[data-project-id]');
    if (card) void openProjectCard(card.dataset.projectId);
  });

  grid.addEventListener('keydown', (event) => {
    const card = event.target.closest('[data-project-id]');
    if (!card) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      void openProjectCard(card.dataset.projectId);
    }
  });

  searchInput.addEventListener('input', () => {
    searchQuery = searchInput.value;
    renderProjects();
  });

  aiForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const prompt = aiInput.value.trim();
    if (!prompt) {
      aiInput.focus();
      return;
    }
    const name = prompt.replace(/\s+/g, ' ').slice(0, 42);
    aiForm.reset();
    await newProject(name);
    setPanel('text');
    addTextClip(prompt, {
      start: 0.4,
      duration: 5,
      preset: { size: 84, weight: 800, color: '#ffffff' },
    });
    toast('Borrador creado con EditPilot', 'success', 3200);
  });

  fileInput.addEventListener('change', async () => {
    const files = Array.from(fileInput.files || []);
    fileInput.value = '';
    if (!files.length) return;
    const base = (files[0].name || 'Proyecto importado').replace(/\.[^.]+$/, '').slice(0, 60);
    await newProject(base || 'Proyecto importado');
    await importFiles(files);
  });

  projectFileInput.addEventListener('change', async () => {
    const file = projectFileInput.files?.[0];
    projectFileInput.value = '';
    if (!file) return;
    try {
      const result = await importProjectFile(file);
      toast(`Proyecto “${result.name}” abierto desde el archivo`, 'success', 3600);
    } catch (error) {
      toast(error?.message || 'No se pudo abrir el archivo de proyecto', 'error', 4200);
    }
  });

  subscriptions.push(on('projects', renderProjects));
  subscriptions.push(on('booted', renderProjects));

  renderProjects();
  renderShowcase();

  return {
    element: home,
    destroy() {
      subscriptions.forEach((off) => off());
    },
  };
}
