import {
  state,
  on,
  setPanel,
  updateProject,
  newProject,
  saveNow,
  goHome,
  importFiles,
  exportProjectFile,
  importProjectFile,
} from './store.js';
import { openLogDialog } from './log.js';
import { icon, logo } from './icons.js';
import { mountLeftPanel } from './panels.js';
import { mountDetailsPanel } from './panels.js';
import { mountPreview } from './preview.js';
import { mountTimeline } from './timeline.js';
import { openExportDialog, captureScreenshot } from './export.js';
import { openShortcutsDialog, openAboutDialog } from './dialogs.js';
import { openCapCutImportDialog } from './capcut.js';
import { openAiBridgeDialog } from './ai-bridge.js';
import { autoEditPanelHtml, wireAutoEditPanel } from './auto-edit.js';
import { openModal, promptDialog } from './modal.js';
import { escapeHtml, toast } from './utils.js';

const TABS = [
  ['media', 'Archivos multimedia', 'film', 'Archivos'],
  ['audio', 'Audio', 'music', 'Audio'],
  ['text', 'Texto', 'text', 'Texto'],
  ['stickers', 'Stickers', 'sticker', 'Stickers'],
  ['effects', 'Efectos', 'sparkles', 'Efectos'],
  ['transitions', 'Transiciones', 'transition', 'Transic.'],
  ['captions', 'Subtítulos', 'captions', 'Subtít.'],
  ['filters', 'Filtros', 'palette', 'Filtros'],
  ['adjust', 'Ajuste', 'adjust', 'Ajustar'],
  ['templates', 'Plantillas', 'templates', 'Plantillas'],
  ['avatar', 'Avatar de IA', 'avatar', 'Avatar'],
  ['auto', 'Editor automático IA', 'sparkles', 'Auto IA'],
];

export function mountEditor(root) {
  if (!state.project) {
    goHome();
    return { destroy() {} };
  }
  const project = state.project;
  root.innerHTML = `
    <div class="editor">
      <header class="titlebar">
        <div class="titlebar-left">
          <button class="app-brand" data-act="home" title="Volver al inicio">${logo}<span>openCUP</span></button>
          <button class="menu-btn" data-act="menu">Menú ${icon('chevronDown', 12)}</button>
          <button class="menu-btn" data-act="log" title="Ver el registro de la app">Registro</button>
        </div>
        <input class="project-title" data-project-name value="${escapeHtml(project.name)}" maxlength="80" aria-label="Nombre del proyecto" />
        <div class="titlebar-right">
          <span class="save-state" data-save-state></span>
          <button class="win-icon" data-act="shot" title="Capturar fotograma">${icon('monitor', 15)}${icon('chevronDown', 9)}</button>
          <button class="win-icon" data-act="maximize" title="Pantalla completa">${icon('fullscreen', 14)}</button>
          <button class="pro-banner" data-act="pro"><span class="pro-gem">◆</span> Obtén 7 días gratis de la versión Pro</button>
          <button class="share-btn" data-act="share">${icon('share', 14)} Compartir</button>
          <button class="export-btn" data-act="export">${icon('export', 14)} Exportar</button>
          <div class="win-controls">
            <button data-act="minimize" title="Minimizar">${icon('minus', 14)}</button>
            <button data-act="maximize" title="Maximizar">${icon('square', 12)}</button>
            <button class="close" data-act="close" title="Cerrar">${icon('x', 14)}</button>
          </div>
        </div>
      </header>
      <div class="workspace">
        <div class="left-col">
          <nav class="tools-strip" data-tabs>
            ${TABS.map(([id, label, iconName, short]) => `
              <button class="tab${state.panel === id ? ' active' : ''}" data-panel="${id}" title="${label}">
                ${icon(iconName, 17)}<span>${short}</span>
              </button>`).join('')}
          </nav>
          <aside data-left></aside>
        </div>
        <section data-center></section>
        <aside data-details></aside>
      </div>
      <section data-timeline></section>
      <input type="file" class="hidden" multiple data-import-input accept="video/*,image/*,audio/*,.mkv,.mov,.m4v,.webm,.mp3,.wav,.m4a,.aac,.ogg" />
      <input type="file" class="hidden" data-project-input accept=".json,.opencup,application/json" />
    </div>`;

  const tabs = root.querySelector('[data-tabs]');
  const projectName = root.querySelector('[data-project-name]');
  const saveState = root.querySelector('[data-save-state]');
  const importInput = root.querySelector('[data-import-input]');
  const projectInput = root.querySelector('[data-project-input]');
  const editRoot = root.querySelector('.editor');
  const leftHost = root.querySelector('[data-left]');
  const detailsHost = root.querySelector('[data-details]');

  const left = mountLeftPanel(leftHost);
  const preview = mountPreview(root.querySelector('[data-center]'));
  const details = mountDetailsPanel(detailsHost);
  const timeline = mountTimeline(root.querySelector('[data-timeline]'));

  const subscriptions = [];
  let popover = null;
  let popoverCleanup = null;
  let sidePanelOpen = false;

  function closePopover() {
    popover?.remove();
    popover = null;
    popoverCleanup?.();
    popoverCleanup = null;
  }

  function showPopover(anchor, items, { width = 228 } = {}) {
    closePopover();
    const element = document.createElement('div');
    element.className = 'popover';
    element.style.width = `${width}px`;
    element.innerHTML = items.map((item) => {
      if (item === '-') return '<div class="sep"></div>';
      if (item.label && item.header) return `<div class="popover-label">${escapeHtml(item.label)}</div>`;
      return `<button data-menu-id="${item.id}">${icon(item.icon || 'info', 15)}<span>${escapeHtml(item.label)}</span></button>`;
    }).join('');
    document.body.append(element);
    const rect = anchor.getBoundingClientRect();
    const leftPos = Math.max(8, Math.min(rect.left, window.innerWidth - width - 10));
    const top = Math.min(rect.bottom + 6, window.innerHeight - element.offsetHeight - 10);
    element.style.left = `${leftPos}px`;
    element.style.top = `${Math.max(8, top)}px`;
    popover = element;

    const onDown = (event) => {
      if (!element.contains(event.target) && event.target !== anchor) closePopover();
    };
    const onKey = (event) => {
      if (event.key === 'Escape') closePopover();
    };
    requestAnimationFrame(() => {
      document.addEventListener('mousedown', onDown);
      document.addEventListener('keydown', onKey);
    });
    popoverCleanup = () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };

    element.addEventListener('click', async (event) => {
      const item = event.target.closest('[data-menu-id]');
      if (!item) return;
      closePopover();
      await runMenuAction(item.dataset.menuId);
    });
  }

  async function createNewProject() {
    const name = await promptDialog({
      title: 'Nuevo proyecto',
      label: 'Nombre del proyecto',
      value: 'Proyecto sin título',
      confirmLabel: 'Crear proyecto',
    });
    if (!name) return;
    await saveNow();
    await newProject(name);
  }

  async function runMenuAction(action) {
    if (action === 'home') goHome();
    else if (action === 'new') await createNewProject();
    else if (action === 'save') {
      await saveNow();
      toast('Proyecto guardado', 'success');
    } else if (action === 'import') importInput.click();
    else if (action === 'saveFile') {
      try {
        await saveNow();
        const result = await exportProjectFile();
        const note = result.skipped
          ? ` · ${result.skipped} medio(s) grandes quedan solo en este navegador`
          : '';
        toast(`Proyecto guardado como archivo${note}`, 'success', 4200);
      } catch (error) {
        toast(error?.message || 'No se pudo guardar el proyecto', 'error');
      }
    } else if (action === 'openFile') projectInput.click();
    else if (action === 'capcut') openCapCutImportDialog();
    else if (action === 'aibridge') void openAiBridgeDialog();
    else if (action === 'export') openExportDialog();
    else if (action === 'shortcuts') openShortcutsDialog();
    else if (action === 'about') openAboutDialog();
    else if (action === 'storage') {
      const estimate = await navigator.storage?.estimate?.().catch(() => null);
      const used = estimate?.usage ? `${(estimate.usage / 1024 / 1024).toFixed(1)} MB` : 'desconocido';
      toast(`Almacenamiento local usado: ${used}`, 'info', 3800);
    }
  }

  function openMainMenu(anchor) {
    showPopover(anchor, [
      { header: true, label: 'Proyecto' },
      { id: 'new', label: 'Nuevo proyecto', icon: 'plus' },
      { id: 'save', label: 'Guardar ahora', icon: 'save' },
      { id: 'saveFile', label: 'Guardar proyecto como archivo', icon: 'download' },
      { id: 'openFile', label: 'Abrir proyecto desde archivo', icon: 'folderOpen' },
      { id: 'import', label: 'Importar medios', icon: 'import' },
      { id: 'capcut', label: 'Abrir proyecto de CapCut', icon: 'folderOpen' },
      { id: 'aibridge', label: 'Puente IA · puerto local', icon: 'link' },
      { id: 'export', label: 'Exportar video', icon: 'export' },
      '-',
      { id: 'home', label: 'Ir al inicio', icon: 'home' },
      { id: 'shortcuts', label: 'Atajos de teclado', icon: 'keyboard' },
      { id: 'about', label: 'Acerca de openCUP', icon: 'info' },
    ]);
  }

  function openSettingsMenu(anchor) {
    showPopover(anchor, [
      { header: true, label: 'Preferencias' },
      { id: 'shortcuts', label: 'Atajos de teclado', icon: 'keyboard' },
      { id: 'storage', label: 'Espacio de almacenamiento', icon: 'cloud' },
      '-',
      { id: 'about', label: 'Acerca de openCUP', icon: 'info' },
    ], { width: 240 });
  }

  function showProDialog() {
    openModal({
      title: 'openCUP Pro',
      width: 440,
      body: `
        <div style="font-size:12px;line-height:1.6;color:#c7c7cf;margin-bottom:12px">
          Esta versión de openCUP incluye todas las funciones <b>sin suscripción</b> y funciona completamente en tu navegador.
        </div>
        <div class="modal-note">
          No hay pagos, cuentas ni límites de exportación. Tus archivos nunca salen de tu equipo.
        </div>`,
      footer: `<button class="btn btn-primary" data-close-pro>Genial</button>`,
      onMount(modal) {
        modal.foot.querySelector('[data-close-pro]').addEventListener('click', modal.close);
      },
    });
  }

  function showShareDialog() {
    openModal({
      title: 'Compartir proyecto',
      width: 440,
      body: `
        <div style="font-size:12px;line-height:1.6;color:#c7c7cf;margin-bottom:12px">
          openCUP guarda este proyecto únicamente en tu navegador. Para compartirlo, exporta el video y envía el archivo.
        </div>
        <div class="modal-note">Nombre del proyecto: <b>${escapeHtml(state.project?.name || '')}</b></div>`,
      footer: `<button class="btn" data-share-close>Cerrar</button><button class="btn btn-primary" data-share-export>Exportar video</button>`,
      onMount(modal) {
        modal.foot.querySelector('[data-share-close]').addEventListener('click', modal.close);
        modal.foot.querySelector('[data-share-export]').addEventListener('click', () => {
          modal.close();
          openExportDialog();
        });
      },
    });
  }

  function updateTabs() {
    for (const tab of tabs.querySelectorAll('[data-panel]')) {
      tab.classList.toggle('active', tab.dataset.panel === state.panel);
    }
    leftHost.classList.toggle('showing-tool', state.panel !== 'media');
  }

  function updateSaveState() {
    if (!saveState) return;
    saveState.classList.toggle('saving', state.saving);
    saveState.innerHTML = state.saving ? '<span class="save-dot"></span> Guardando' : '';
  }

  function syncProjectName() {
    if (document.activeElement === projectName) return;
    if (state.project && projectName.value !== state.project.name) projectName.value = state.project.name;
    document.title = state.project ? `${state.project.name} — openCUP` : 'openCUP — Editor de video';
  }

  function setDetailsVisible(visible) {
    editRoot.classList.toggle('details-hidden', !visible);
  }

  function toggleNarrowPanel(force) {
    sidePanelOpen = typeof force === 'boolean' ? force : !sidePanelOpen;
    leftHost.classList.toggle('open', sidePanelOpen);
  }

  editRoot.addEventListener('click', async (event) => {
    const tab = event.target.closest('.tools-strip [data-panel]');
    if (tab) {
      if (window.matchMedia('(max-width: 980px)').matches) {
        if (state.panel === tab.dataset.panel && sidePanelOpen) {
          toggleNarrowPanel(false);
          return;
        }
        toggleNarrowPanel(true);
      }
      setPanel(tab.dataset.panel);
      if (window.matchMedia('(min-width: 1161px)').matches) setDetailsVisible(true);
      return;
    }
    const action = event.target.closest('[data-act]')?.dataset.act;
    if (!action) return;
    if (action === 'home') goHome();
    else if (action === 'menu') openMainMenu(event.target.closest('[data-act]'));
    else if (action === 'log') openLogDialog();
    else if (action === 'export') openExportDialog();
    else if (action === 'share') showShareDialog();
    else if (action === 'pro') showProDialog();
    else if (action === 'minimize' || action === 'close') goHome();
    else if (action === 'maximize') {
      if (document.fullscreenElement) document.exitFullscreen();
      else editRoot.requestFullscreen?.().catch(() => {});
    } else if (action === 'shot') {
      try {
        await captureScreenshot();
      } catch (error) {
        console.error(error);
        toast('No se pudo capturar el fotograma', 'error');
      }
    } else if (action === 'settings') openSettingsMenu(event.target.closest('[data-act]'));
    else if (action === 'panel') toggleNarrowPanel();
    else if (action === 'details') {
      const hidden = editRoot.classList.contains('details-hidden');
      setDetailsVisible(hidden);
      document.querySelector('[data-act="details"]')?.classList.toggle('active', hidden);
    }
  });

  // El botón de detalles vive dentro del reproductor; se delega desde la raíz.
  projectName.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') projectName.blur();
    if (event.key === 'Escape') {
      projectName.value = state.project?.name || '';
      projectName.blur();
    }
  });

  projectName.addEventListener('change', () => {
    const value = projectName.value.trim();
    if (!value || !state.project || value === state.project.name) {
      projectName.value = state.project?.name || '';
      return;
    }
    updateProject({ name: value });
  });

  importInput.addEventListener('change', async () => {
    if (importInput.files?.length) await importFiles(importInput.files);
    importInput.value = '';
  });

  projectInput.addEventListener('change', async () => {
    const file = projectInput.files?.[0];
    projectInput.value = '';
    if (!file) return;
    try {
      await saveNow();
      const result = await importProjectFile(file);
      toast(`Proyecto “${result.name}” abierto desde el archivo`, 'success', 3600);
    } catch (error) {
      toast(error?.message || 'No se pudo abrir el archivo de proyecto', 'error', 4200);
    }
  });

  subscriptions.push(on('panel', updateTabs));
  subscriptions.push(on('saving', updateSaveState));
  subscriptions.push(on('project', () => {
    syncProjectName();
    updateSaveState();
  }));

  updateSaveState();
  syncProjectName();
  updateTabs();
  const narrowDetailsQuery = window.matchMedia('(max-width: 1160px)');
  if (narrowDetailsQuery.matches) setDetailsVisible(false);
  const onNarrowDetails = (event) => setDetailsVisible(!event.matches);
  narrowDetailsQuery.addEventListener('change', onNarrowDetails);

  return {
    element: editRoot,
    destroy() {
      closePopover();
      narrowDetailsQuery.removeEventListener('change', onNarrowDetails);
      subscriptions.forEach((off) => off());
      left.destroy();
      preview.destroy();
      details.destroy();
      timeline.destroy();
    },
  };
}
