// Marco de escritorio de cupcraft.
//
// La app nativa (Rust + WebView2) expone `window.ipc.postMessage` para
// controlar la ventana. En un navegador normal todo esto se desactiva y la
// interfaz se comporta como antes.

// Puente con la aplicación nativa (Rust + WebView2). En un navegador normal
// no existe y la barra de título se muestra pero sin controles nativos.
// Puente con la aplicación nativa (Rust + WebView2). En un navegador normal
// no existe y la barra de título se muestra pero sin controles nativos.
const bridge = typeof window.ipc?.postMessage === 'function' ? window.ipc : null;

const send = (message) => {
  if (!bridge) return;
  try {
    bridge.postMessage(message);
  } catch (error) {
    console.warn('No se pudo enviar el mensaje a la ventana:', message, error);
  }
};

// La barra de título siempre se muestra; los controles nativos solo se
// activan cuando el puente está disponible.
document.documentElement.classList.add('app-desktop');

// Asegurar que el puente esté disponible incluso si el script de
// inicialización de wry no se ejecutó correctamente.
if (!bridge && typeof window.ipc === 'object') {
  window.ipc.postMessage = function(message) {
    try {
      if (window.chrome && window.chrome.webview && typeof window.chrome.webview.postMessage === 'function') {
        window.chrome.webview.postMessage(String(message));
      }
    } catch (e) {}
  };
}

let maximized = false;

const ICON_MAXIMIZE =
  '<svg viewBox="0 0 10 10" aria-hidden="true"><rect x="2.5" y="2.5" width="5" height="5" rx="0.5" /></svg>';
const ICON_RESTORE =
  '<svg viewBox="0 0 10 10" aria-hidden="true"><path d="M3.5 1.9h4.6v4.6" /><rect x="1.9" y="3.5" width="4.6" height="4.6" rx="0.5" /></svg>';

const desktop = {
  minimize: () => send('win:minimize'),
  toggleMaximize: () => send('win:maximize'),
  close: () => send('win:close'),
  drag: () => send('win:drag'),
  resize: (direction) => send(`win:resize:${direction}`),
  sync: () => send('win:state'),
  open: (url) => send(`open-url:${url}`),
  setMaximized(value) {
    maximized = Boolean(value);
    const button = document.querySelector('[data-win="maximize"]');
    if (button) {
      button.innerHTML = maximized ? ICON_RESTORE : ICON_MAXIMIZE;
      const label = maximized ? 'Restaurar' : 'Maximizar';
      button.title = label;
      button.setAttribute('aria-label', label);
    }
    document.documentElement.classList.toggle('app-maximized', maximized);
  },
};

window.cupcraftDesktop = desktop;

const native = Boolean(bridge);
if (!native) {
  // Navegador normal: barra visible, pero sin controles nativos.
} else {

  // Arrastre y control de la ventana unificado desde la barra superior de la app
  document.addEventListener('mousedown', (event) => {
    if (event.button !== 0) return;
    if (event.target.closest('button, input, textarea, a, select, [data-resize], .tools-strip, .left-col, .stage-wrap, .timeline')) return;
    if (event.target.closest('.home-toolbar, .titlebar, [data-win-drag], [data-app-titlebar]')) {
      desktop.drag();
    }
  });

  document.addEventListener('dblclick', (event) => {
    if (event.target.closest('button, input, textarea, a, select, .tools-strip, .left-col, .stage-wrap, .timeline')) return;
    if (event.target.closest('.home-toolbar, .titlebar, [data-win-drag], [data-app-titlebar]')) {
      desktop.toggleMaximize();
    }
  });

  document.addEventListener('click', (event) => {
    const button = event.target.closest('button[data-win]');
    if (!button) return;
    event.preventDefault();
    if (button.dataset.win === 'minimize') desktop.minimize();
    else if (button.dataset.win === 'maximize') desktop.toggleMaximize();
    else if (button.dataset.win === 'close') desktop.close();
  });

  // Desactivar menú contextual del navegador para que se sienta 100% aplicación nativa
  document.addEventListener('contextmenu', (event) => {
    if (!event.target.closest('input, textarea, [contenteditable="true"]')) {
      event.preventDefault();
    }
  });

  // Bordes de la ventana: sin barra del sistema hay que redimensionar a mano.
  const EDGES = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'];
  for (const direction of EDGES) {
    const handle = document.createElement('div');
    handle.className = 'app-resize';
    handle.dataset.dir = direction;
    handle.addEventListener('mousedown', (event) => {
      if (event.button !== 0 || maximized) return;
      event.preventDefault();
      event.stopPropagation();
      desktop.resize(direction);
    });
    document.body.append(handle);
  }

  // Mantener el icono del botón maximizar en sintonía con la ventana real
  // (doble clic en la barra, arrastrar al borde superior, etc.).
  let resizeTimer = 0;
  window.addEventListener('resize', () => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => desktop.sync(), 90);
  });
  window.addEventListener('focus', () => desktop.sync());
  window.setTimeout(() => desktop.sync(), 600);

  // ---------------------------------------------------------------- teclado
  // Atajos que pertenecen a un navegador, no a la aplicación.
  const blockedKeys = new Set(['F3', 'F5', 'F7', 'F12']);
  const blockedCtrl = new Set(['r', 'p', 'u', 'f', 'j', 'h', 'n', 't', 'w']);

  window.addEventListener(
    'keydown',
    (event) => {
      const key = event.key;
      const mod = event.ctrlKey || event.metaKey;
      const blocked =
        blockedKeys.has(key) ||
        (mod && blockedCtrl.has(key.toLowerCase())) ||
        (mod && event.shiftKey && ['i', 'j', 'c', 'r'].includes(key.toLowerCase()));
      if (!blocked) return;
      event.preventDefault();
      event.stopPropagation();
    },
    true,
  );

  // Sin zoom del navegador con Ctrl + rueda.
  window.addEventListener(
    'wheel',
    (event) => {
      if (event.ctrlKey) event.preventDefault();
    },
    { passive: false },
  );

  // -------------------------------------------------------- enlaces externos
  // Los enlaces http/https se abren en el navegador del sistema, nunca dentro
  // de la ventana de la aplicación.
  const isExternal = (url) => {
    try {
      return new URL(url, window.location.href).origin !== window.location.origin;
    } catch {
      return false;
    }
  };

  document.addEventListener(
    'click',
    (event) => {
      const anchor = event.target.closest?.('a[href]');
      if (!anchor) return;
      const href = anchor.getAttribute('href') || '';
      if (!/^https?:/i.test(href) && !/^https?:/i.test(anchor.href)) return;
      if (!isExternal(anchor.href)) return;
      event.preventDefault();
      event.stopPropagation();
      desktop.open(anchor.href);
    },
    true,
  );

  const nativeOpen = window.open.bind(window);
  window.open = (url, ...rest) => {
    if (url && isExternal(String(url))) {
      desktop.open(new URL(String(url), window.location.href).href);
      return null;
    }
    return nativeOpen(url, ...rest);
  };

  // -------------------------------------------------------- menú contextual
  // WebView2 ya no muestra el menú de Edge. En los campos de texto se ofrece
  // uno pequeño con las acciones habituales.
  let menu = null;

  const closeMenu = () => {
    menu?.remove();
    menu = null;
  };

  const copySelection = async () => {
    const text = window.getSelection?.().toString() || '';
    if (!text) return false;
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      return document.execCommand('copy');
    }
  };

  const pasteInto = async (editable) => {
    try {
      const text = await navigator.clipboard.readText();
      if (typeof editable.value === 'string') {
        const start = editable.selectionStart ?? editable.value.length;
        const end = editable.selectionEnd ?? editable.value.length;
        editable.setRangeText(text, start, end, 'end');
      } else {
        document.execCommand('insertText', false, text);
      }
      editable.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    } catch {
      return false;
    }
  };

  const selectAll = (editable) => {
    if (typeof editable.select === 'function') editable.select();
    else document.execCommand('selectAll');
  };

  const openMenu = (editable, x, y) => {
    closeMenu();

    const isTextField = typeof editable.value === 'string';
    const hasSelection = isTextField
      ? editable.selectionStart !== editable.selectionEnd
      : Boolean(window.getSelection?.().toString());

    menu = document.createElement('div');
    menu.className = 'desktop-menu';
    menu.innerHTML = `
      <button type="button" data-menu-action="cut" ${hasSelection ? '' : 'disabled'}>Cortar <kbd>Ctrl+X</kbd></button>
      <button type="button" data-menu-action="copy" ${hasSelection ? '' : 'disabled'}>Copiar <kbd>Ctrl+C</kbd></button>
      <button type="button" data-menu-action="paste">Pegar <kbd>Ctrl+V</kbd></button>
      <button type="button" data-menu-action="select-all">Seleccionar todo <kbd>Ctrl+A</kbd></button>`;

    document.body.append(menu);
    const rect = menu.getBoundingClientRect();
    menu.style.left = `${Math.max(6, Math.min(x, window.innerWidth - rect.width - 6))}px`;
    menu.style.top = `${Math.max(6, Math.min(y, window.innerHeight - rect.height - 6))}px`;

    menu.addEventListener('mousedown', (event) => event.stopPropagation());
    menu.addEventListener('click', async (event) => {
      const button = event.target.closest('button[data-menu-action]');
      if (!button || button.disabled) return;
      const action = button.dataset.menuAction;
      closeMenu();
      editable.focus();
      if (action === 'cut') {
        document.execCommand('cut');
      } else if (action === 'copy') {
        await copySelection();
      } else if (action === 'paste') {
        const ok = await pasteInto(editable);
        if (!ok) console.warn('Usa Ctrl+V para pegar: el portapapeles no está disponible.');
      } else if (action === 'select-all') {
        selectAll(editable);
      }
    });
  };

  document.addEventListener('contextmenu', (event) => {
    event.preventDefault();
    const editable = event.target.closest?.('input, textarea, [contenteditable="true"]');
    if (!editable || editable.disabled || editable.readOnly) return;
    event.stopPropagation();
    openMenu(editable, event.clientX, event.clientY);
  });

  window.addEventListener('mousedown', (event) => {
    if (menu && !menu.contains(event.target)) closeMenu();
  });
  window.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') closeMenu();
  });
  window.addEventListener('blur', closeMenu);

  // Evitar que arrastrar una imagen o un enlace abra una página nueva.
  document.addEventListener(
    'dragstart',
    (event) => {
      if (event.target.closest?.('img, a, video')) event.preventDefault();
    },
    true,
  );
}
