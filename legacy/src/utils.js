export const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

let counter = 0;
export function uid(prefix = 'id') {
  counter = (counter + 1) % 100000;
  return `${prefix}_${Date.now().toString(36)}${counter.toString(36)}${Math.random().toString(36).slice(2, 7)}`;
}

export function formatTime(seconds, { fps = 30, frames = false, padHours = false } = {}) {
  const safe = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
  const total = Math.floor(safe);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const base = h > 0 || padHours
    ? `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
    : `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  if (!frames) return base;
  const f = Math.floor((safe - total) * fps);
  return `${base}:${String(f).padStart(2, '0')}`;
}

export function formatDuration(seconds) {
  return formatTime(seconds);
}

export function formatBytes(bytes) {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${(bytes / 1024 ** i).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function escapeHtml(value) {
  return String(value ?? '').replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

export function debounce(fn, wait = 400) {
  let timer = 0;
  const wrapped = (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), wait);
  };
  wrapped.flush = (...args) => {
    clearTimeout(timer);
    fn(...args);
  };
  wrapped.cancel = () => clearTimeout(timer);
  return wrapped;
}

export function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}

export function toast(message, kind = 'info', timeout = 3200) {
  const host = document.getElementById('toasts');
  if (!host) return;
  const last = toast._last ?? { message: '', at: 0 };
  const now = Date.now();
  if (last.message === message && now - last.at < 2500) return;
  toast._last = { message, at: now };
  while (host.children.length >= 3) host.firstElementChild?.remove();
  const node = document.createElement('div');
  node.className = `toast toast-${kind}`;
  node.textContent = message;
  host.append(node);
  requestAnimationFrame(() => node.classList.add('show'));
  setTimeout(() => {
    node.classList.remove('show');
    setTimeout(() => node.remove(), 250);
  }, timeout);
}

export function wait(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function canvasToBlob(canvas, type = 'image/jpeg', quality = 0.72) {
  return new Promise((resolve) => canvas.toBlob(resolve, type, quality));
}

export function isEditableTarget(target) {
  return target instanceof HTMLElement && (
    target.isContentEditable ||
    ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)
  );
}
