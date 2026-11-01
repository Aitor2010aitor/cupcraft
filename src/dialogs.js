import { icon } from './icons.js';
import { openModal } from './modal.js';

const SHORTCUTS = [
  ['Reproducir / pausar', 'Espacio'],
  ['Fotograma anterior / siguiente', '← / →'],
  ['Ir al inicio / final', 'Inicio / Fin'],
  ['Dividir clip en el cursor', 'B'],
  ['Duplicar clip seleccionado', 'Ctrl + D'],
  ['Eliminar clip seleccionado', 'Supr'],
  ['Deshacer', 'Ctrl + Z'],
  ['Rehacer', 'Ctrl + Y'],
  ['Guardar proyecto', 'Ctrl + S'],
  ['Importar medios', 'Ctrl + I'],
  ['Cerrar ventanas emergentes', 'Esc'],
];

export function openShortcutsDialog() {
  openModal({
    title: 'Atajos de teclado',
    width: 460,
    body: `<div class="kbd-list">${SHORTCUTS.map(([label, keys]) => `
      <div class="kbd-row"><span>${label}</span><kbd>${keys}</kbd></div>`).join('')}
    </div>
    <div class="modal-note" style="margin-top:14px">Consejo: mantén <b>Alt</b> mientras arrastras un clip para desactivar temporalmente el imán.</div>`,
    footer: `<button class="btn btn-primary" data-close-shortcuts>Entendido</button>`,
    onMount(modal) {
      modal.foot.querySelector('[data-close-shortcuts]').addEventListener('click', modal.close);
    },
  });
}

export function openAboutDialog() {
  openModal({
    title: 'Acerca de openCUP',
    width: 460,
    body: `
      <div style="display:flex;gap:14px;align-items:center;margin-bottom:14px">
        <div style="width:52px;height:52px;border-radius:14px;background:linear-gradient(135deg,#00e5d0,#39bdf8);display:flex;align-items:center;justify-content:center;color:#04120f">
          ${icon('play', 26)}
        </div>
        <div>
          <div style="font-size:17px;font-weight:800">openCUP</div>
          <div class="muted" style="font-size:11px;margin-top:2px">Editor de video local · versión 0.4.0</div>
        </div>
      </div>
      <p style="font-size:11.5px;line-height:1.65;color:#b7b7c0;margin:0 0 12px">
        Un editor de video pensado para el navegador: importa tus archivos, monta la línea de tiempo,
        ajusta color, texto y audio, y exporta tu proyecto sin subir nada a un servidor.
      </p>
      <div class="modal-note">
        openCUP está inspirado en el flujo de trabajo de editores de video modernos, pero su código,
        interfaz y recursos son originales. No está afiliado a CapCut ni a ByteDance.
        Tus proyectos se guardan en el almacenamiento local de este navegador.
      </div>`,
    footer: `<button class="btn btn-primary" data-close-about>Cerrar</button>`,
    onMount(modal) {
      modal.foot.querySelector('[data-close-about]').addEventListener('click', modal.close);
    },
  });
}
