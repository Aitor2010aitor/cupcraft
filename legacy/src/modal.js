import { icon } from './icons.js';
import { escapeHtml } from './utils.js';

let active = null;

export function closeModal() {
  if (!active) return;
  const { backdrop, cleanup } = active;
  active = null;
  cleanup?.();
  backdrop.remove();
}

export function openModal({ title, body = '', footer = '', width = 560, onMount, dismissible = true, className = '' }) {
  closeModal();
  const host = document.getElementById('modal-root');
  const backdrop = document.createElement('div');
  backdrop.className = 'modal-backdrop';
  backdrop.innerHTML = `
    <div class="modal ${className}" style="width:min(94vw, ${width}px)" role="dialog" aria-modal="true" aria-label="${escapeHtml(title)}">
      <div class="modal-head">
        <h2>${escapeHtml(title)}</h2>
        <button class="icon-btn" data-modal-close aria-label="Cerrar">${icon('x', 16)}</button>
      </div>
      <div class="modal-body">${body}</div>
      ${footer ? `<div class="modal-foot">${footer}</div>` : ''}
    </div>`;
  host.append(backdrop);

  const onKey = (event) => {
    if (event.key === 'Escape' && dismissible) closeModal();
  };
  document.addEventListener('keydown', onKey);
  const onBackdrop = (event) => {
    if (event.target === backdrop && dismissible) closeModal();
  };
  backdrop.addEventListener('mousedown', onBackdrop);
  backdrop.querySelector('[data-modal-close]')?.addEventListener('click', closeModal);

  const api = {
    backdrop,
    root: backdrop.querySelector('.modal'),
    body: backdrop.querySelector('.modal-body'),
    foot: backdrop.querySelector('.modal-foot'),
    close: closeModal,
    cleanup: () => document.removeEventListener('keydown', onKey),
  };
  active = api;

  onMount?.(api);
  return api;
}

export function confirmDialog({ title = '¿Seguro?', message = '', confirmLabel = 'Confirmar', cancelLabel = 'Cancelar', danger = false }) {
  return new Promise((resolve) => {
    let settled = false;
    const finish = (value) => {
      if (settled) return;
      settled = true;
      api.close();
      resolve(value);
    };
    const api = openModal({
      title,
      width: 420,
      body: `<p style="margin:0;font-size:12px;line-height:1.6;color:#b7b7c0">${escapeHtml(message)}</p>`,
      footer: `<button class="btn" data-cancel>${escapeHtml(cancelLabel)}</button><button class="btn ${danger ? 'btn-danger' : 'btn-primary'}" data-confirm>${escapeHtml(confirmLabel)}</button>`,
      onMount(modal) {
        modal.foot.querySelector('[data-cancel]').addEventListener('click', () => finish(false));
        modal.foot.querySelector('[data-confirm]').addEventListener('click', () => finish(true));
        modal.backdrop.addEventListener('mousedown', (event) => {
          if (event.target === modal.backdrop) finish(false);
        });
      },
    });
  });
}

export function promptDialog({ title = 'Renombrar', label = 'Nombre', value = '', confirmLabel = 'Guardar' }) {
  return new Promise((resolve) => {
    let settled = false;
    const finish = (result) => {
      if (settled) return;
      settled = true;
      api.close();
      resolve(result);
    };
    const api = openModal({
      title,
      width: 420,
      body: `<div class="field"><label>${escapeHtml(label)}</label><input class="input" data-prompt-input value="${escapeHtml(value)}" maxlength="80" /></div>`,
      footer: `<button class="btn" data-cancel>Cancelar</button><button class="btn btn-primary" data-confirm>${escapeHtml(confirmLabel)}</button>`,
      onMount(modal) {
        const input = modal.body.querySelector('[data-prompt-input]');
        input.focus();
        input.select();
        const submit = () => finish(input.value.trim() || null);
        modal.foot.querySelector('[data-confirm]').addEventListener('click', submit);
        modal.foot.querySelector('[data-cancel]').addEventListener('click', () => finish(null));
        input.addEventListener('keydown', (event) => {
          if (event.key === 'Enter') submit();
        });
        modal.backdrop.addEventListener('mousedown', (event) => {
          if (event.target === modal.backdrop) finish(null);
        });
      },
    });
  });
}
