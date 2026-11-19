/* Divisores arrastrables del área de trabajo de cupcraft.
 *
 * Reproduce el comportamiento de CapCut: las líneas verticales que separan el
 * panel izquierdo, la vista previa y el panel de detalles se pueden arrastrar
 * para ensanchar o estrechar cada ventana. El reparto se guarda en localStorage
 * y un doble clic vuelve al tamaño por defecto.
 */

const STORAGE_KEY = 'cupcraft.panel-widths';
const SPLIT_W = 6;
const MIN_LEFT = 264;
const MIN_CENTER = 300;
const MIN_RIGHT = 264;

function readStored() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const data = JSON.parse(raw);
    if (!data || !Number.isFinite(data.left) || !Number.isFinite(data.right)) return null;
    return { left: Math.round(data.left), right: Math.round(data.right) };
  } catch {
    return null;
  }
}

function writeStored(widths) {
  try {
    if (!widths) localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, JSON.stringify(widths));
  } catch {
    /* almacenamiento no disponible: se ignora */
  }
}

export function installPanelResizers({ workspace, leftCol, detailsPanel, onChange }) {
  if (!workspace || !leftCol || !detailsPanel) return () => {};

  const leftSplit = workspace.querySelector('[data-split="left"]');
  const rightSplit = workspace.querySelector('[data-split="right"]');
  if (!leftSplit || !rightSplit) return () => {};

  let stored = readStored();

  const setWidth = (name, value) => {
    if (value == null) workspace.style.removeProperty(name);
    else workspace.style.setProperty(name, `${Math.round(value)}px`);
  };

  const detailsVisible = () => {
    if (document.querySelector('.editor')?.classList.contains('details-hidden')) return false;
    const style = getComputedStyle(detailsPanel);
    return style.display !== 'none' && detailsPanel.getBoundingClientRect().width > 1;
  };

  const usableWidth = (withDetails) => {
    const style = getComputedStyle(workspace);
    const padding = (parseFloat(style.paddingLeft) || 0) + (parseFloat(style.paddingRight) || 0);
    const splits = withDetails ? SPLIT_W * 2 : SPLIT_W;
    return Math.max(MIN_LEFT + MIN_CENTER, workspace.clientWidth - padding - splits);
  };

  const notify = () => {
    onChange?.();
  };

  const clear = () => {
    setWidth('--left-col-w', null);
    setWidth('--right-col-w', null);
  };

  /* Aplica los anchos guardados respetando el espacio disponible. */
  function fitStored() {
    if (window.innerWidth <= 1160) {
      clear();
      return;
    }
    if (!stored) return;
    const withDetails = detailsVisible();
    const total = usableWidth(withDetails);
    let left = stored.left;
    let right = withDetails ? stored.right : 0;

    const maxLeft = total - (withDetails ? Math.max(MIN_RIGHT, right) : 0) - MIN_CENTER;
    left = Math.max(MIN_LEFT, Math.min(left, maxLeft));
    setWidth('--left-col-w', left);

    if (withDetails) {
      const maxRight = total - left - MIN_CENTER;
      right = Math.max(MIN_RIGHT, Math.min(right, maxRight));
      setWidth('--right-col-w', right);
    } else {
      setWidth('--right-col-w', null);
    }
  }

  function startDrag(which, event) {
    if (event.button !== 0 && event.pointerType === 'mouse') return;
    if (window.innerWidth <= 1160) return;

    const withDetails = detailsVisible();
    const total = usableWidth(withDetails);
    const startX = event.clientX;
    const startLeft = leftCol.getBoundingClientRect().width;
    const startRight = withDetails ? detailsPanel.getBoundingClientRect().width : 0;
    const splitter = which === 'left' ? leftSplit : rightSplit;

    event.preventDefault();
    splitter.classList.add('dragging');
    try { splitter.setPointerCapture(event.pointerId); } catch { /* ignorado */ }
    document.body.classList.add('resizing-panels');

    let frame = 0;
    const schedule = () => {
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        notify();
      });
    };

    const move = (moveEvent) => {
      const dx = moveEvent.clientX - startX;
      if (which === 'left') {
        const maxLeft = total - (withDetails ? startRight : 0) - MIN_CENTER;
        const left = Math.max(MIN_LEFT, Math.min(maxLeft, startLeft + dx));
        setWidth('--left-col-w', left);
      } else {
        const maxRight = total - startLeft - MIN_CENTER;
        const right = Math.max(MIN_RIGHT, Math.min(maxRight, startRight - dx));
        setWidth('--right-col-w', right);
      }
      schedule();
    };

    const finish = () => {
      splitter.classList.remove('dragging');
      document.body.classList.remove('resizing-panels');
      splitter.removeEventListener('pointermove', move);
      splitter.removeEventListener('pointerup', finish);
      splitter.removeEventListener('pointercancel', finish);
      try { splitter.releasePointerCapture(event.pointerId); } catch { /* ignorado */ }

      const left = Math.round(leftCol.getBoundingClientRect().width);
      const right = withDetails
        ? Math.round(detailsPanel.getBoundingClientRect().width)
        : (stored?.right ?? 340);
      stored = { left, right };
      writeStored(stored);
      notify();
    };

    splitter.addEventListener('pointermove', move);
    splitter.addEventListener('pointerup', finish);
    splitter.addEventListener('pointercancel', finish);
  }

  function reset() {
    stored = null;
    writeStored(null);
    clear();
    notify();
  }

  function nudge(which, direction) {
    if (window.innerWidth <= 1160) return;
    const withDetails = detailsVisible();
    const total = usableWidth(withDetails);
    const current = which === 'left'
      ? leftCol.getBoundingClientRect().width
      : detailsPanel.getBoundingClientRect().width;
    const step = 24 * direction;
    const next = Math.round(current + (which === 'left' ? step : -step));
    if (which === 'left') {
      const maxLeft = total - (withDetails ? detailsPanel.getBoundingClientRect().width : 0) - MIN_CENTER;
      setWidth('--left-col-w', Math.max(MIN_LEFT, Math.min(maxLeft, next)));
    } else {
      const maxRight = total - leftCol.getBoundingClientRect().width - MIN_CENTER;
      setWidth('--right-col-w', Math.max(MIN_RIGHT, Math.min(maxRight, next)));
    }
    const widths = {
      left: Math.round(leftCol.getBoundingClientRect().width),
      right: Math.round(detailsPanel.getBoundingClientRect().width) || (stored?.right ?? 340),
    };
    stored = widths;
    writeStored(widths);
    notify();
  }

  const onLeftDown = (event) => startDrag('left', event);
  const onRightDown = (event) => startDrag('right', event);
  const onLeftKey = (event) => {
    if (event.key === 'ArrowLeft') { event.preventDefault(); nudge('left', -1); }
    else if (event.key === 'ArrowRight') { event.preventDefault(); nudge('left', 1); }
    else if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); reset(); }
  };
  const onRightKey = (event) => {
    if (event.key === 'ArrowLeft') { event.preventDefault(); nudge('right', -1); }
    else if (event.key === 'ArrowRight') { event.preventDefault(); nudge('right', 1); }
    else if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); reset(); }
  };
  const onWindowResize = () => fitStored();

  leftSplit.addEventListener('pointerdown', onLeftDown);
  rightSplit.addEventListener('pointerdown', onRightDown);
  leftSplit.addEventListener('dblclick', reset);
  rightSplit.addEventListener('dblclick', reset);
  leftSplit.addEventListener('keydown', onLeftKey);
  rightSplit.addEventListener('keydown', onRightKey);
  window.addEventListener('resize', onWindowResize);

  fitStored();

  return () => {
    leftSplit.removeEventListener('pointerdown', onLeftDown);
    rightSplit.removeEventListener('pointerdown', onRightDown);
    leftSplit.removeEventListener('dblclick', reset);
    rightSplit.removeEventListener('dblclick', reset);
    leftSplit.removeEventListener('keydown', onLeftKey);
    rightSplit.removeEventListener('keydown', onRightKey);
    window.removeEventListener('resize', onWindowResize);
    document.body.classList.remove('resizing-panels');
  };
}
