import path from 'node:path';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:8787/';
const MEDIA = path.resolve('tools/fixtures');

const browser = await puppeteer.launch({ executablePath: CHROME, headless: true, args: ['--window-size=1440,900'] });
const page = await browser.newPage();
await page.setViewport({ width: 1440, height: 900 });
const errors = [];
page.on('pageerror', (e) => errors.push('pageerror: ' + e.message));
page.on('console', (m) => { if (m.type() === 'error') errors.push('console: ' + m.text()); });

const wait = (ms) => new Promise((r) => setTimeout(r, ms));
let pass = 0;
let fail = 0;
const check = (ok, label, extra = '') => {
  if (ok) { pass++; console.log('  ok  ' + label); }
  else { fail++; console.log('  FALLA ' + label + (extra ? ' → ' + extra : '')); }
};

await page.goto(URL, { waitUntil: 'networkidle2' });
await wait(600);
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Filter Bar Test'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(500);

// Importar un clip de imagen.
const chooser = page.waitForFileChooser();
await page.click('.media-drop');
(await chooser).accept([path.join(MEDIA, 'foto.png')]);
await wait(1200);
await page.click('.clip');
await wait(300);

const clipState = () => page.evaluate(() => {
  const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
  return c ? {
    id: c.id, filter: c.filter, lut: c.lut ?? null, lutName: c.lutName ?? null, lutAmount: c.lutAmount ?? 1,
    glFilter: c.glFilter ?? null, glFilterName: c.glFilterName ?? null, glFilterAmount: c.glFilterAmount ?? 1,
  } : null;
});

const barInfo = () => page.evaluate(() => {
  const bar = document.querySelector('.clip-filter');
  if (!bar) return null;
  return {
    name: bar.querySelector('.clip-filter-name')?.textContent ?? '',
    amount: bar.querySelector('.clip-filter-amount')?.textContent ?? '',
    hasFilterClass: bar.closest('.clip')?.classList.contains('has-filter') ?? false,
  };
});

// --- 1. Aplicar un filtro CapCut y comprobar la barra sobre el clip. ---
await page.click('[data-panel="filters"]');
await wait(250);
const applied = await page.evaluate(() => {
  const card = [...document.querySelectorAll('[data-capcut-filter]')].find((c) => c.dataset.capcutName === 'Calabaza');
  if (!card) return null;
  card.click();
  return card.dataset.capcutName;
});
await wait(700);
check(applied === 'Calabaza', 'se aplica el filtro CapCut "Calabaza"');

const bar1 = await barInfo();
check(!!bar1, 'la barra .clip-filter aparece sobre el clip', JSON.stringify(bar1));
check(bar1?.name === 'Calabaza', 'la barra muestra el nombre del filtro', JSON.stringify(bar1));
check(bar1?.amount === '100%', 'la barra muestra la intensidad 100%', JSON.stringify(bar1));
check(bar1?.hasFilterClass === true, 'el clip tiene la clase has-filter');

// --- 1b. El indicador es un chip flotante que se arrastra para recolocarlo. ---
const chipPos = () => page.evaluate(() => {
  const chip = document.querySelector('.clip-filter');
  const clip = chip?.closest('.clip');
  const scroller = document.querySelector('[data-scroll]');
  if (!chip || !clip || !scroller) return null;
  const id = chip.dataset.clipFilter;
  const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === id);
  const cr = chip.getBoundingClientRect();
  const lr = clip.getBoundingClientRect();
  const vr = scroller.getBoundingClientRect();
  const dataX = parseFloat(chip.dataset.chipX);
  return {
    x: Number.isFinite(c?.filterChipX) ? c.filterChipX : dataX,
    y: Number.isFinite(c?.filterChipY) ? c.filterChipY : 0.5,
    dataX,
    scrollLeft: scroller.scrollLeft,
    rect: { left: cr.left, right: cr.right, top: cr.top, bottom: cr.bottom },
    clipRect: { left: lr.left, right: lr.right, top: lr.top, bottom: lr.bottom },
    viewRect: { left: vr.left, right: vr.right, top: vr.top, bottom: vr.bottom },
  };
});

const pos0 = await chipPos();
check(pos0 && pos0.dataX > 0 && pos0.dataX <= 0.5,
  'el chip arranca cerca del inicio del clip (no en un centro lejano)', JSON.stringify(pos0 && { dataX: pos0.dataX }));
check(pos0 && pos0.rect.left >= pos0.viewRect.left - 0.5 && pos0.rect.right <= pos0.viewRect.right + 0.5,
  'el chip es visible dentro de la línea de tiempo', JSON.stringify(pos0 && { rect: pos0.rect, viewRect: pos0.viewRect }));

const editorBeforeDrag = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(!editorBeforeDrag, 'el editor está cerrado antes de arrastrar el chip');

const chipEl = await page.$('.clip-filter');
const chipBox = await chipEl.boundingBox();
await page.mouse.move(chipBox.x + chipBox.width / 2, chipBox.y + chipBox.height / 2);
await page.mouse.down();
await page.mouse.move(chipBox.x + chipBox.width / 2 + 30, chipBox.y + chipBox.height / 2 - 6, { steps: 6 });
await page.mouse.up();
await wait(500);

const pos1 = await chipPos();
check(pos1 && pos0 && (Math.abs(pos1.x - pos0.x) > 0.01 || Math.abs(pos1.y - pos0.y) > 0.01),
  'el arrastre guarda la nueva posición (filterChipX / filterChipY)', JSON.stringify(pos1 && { x: pos1.x, y: pos1.y }));
check(pos1 && pos1.rect.left >= pos1.clipRect.left - 0.5 && pos1.rect.right <= pos1.clipRect.right + 0.5
  && pos1.rect.top >= pos1.clipRect.top - 0.5 && pos1.rect.bottom <= pos1.clipRect.bottom + 0.5,
  'el chip queda dentro del rectángulo del clip', JSON.stringify(pos1 && { rect: pos1.rect, clipRect: pos1.clipRect }));
const editorAfterDrag = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(!editorAfterDrag, 'el arrastre NO abre el editor de intensidad');

// Un clic (sin mover) tras arrastrar sí abre el editor.
await wait(400);
await page.click('.clip-filter');
await wait(300);
const clickAfterDrag = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(clickAfterDrag, 'un clic sobre el chip tras arrastrarlo abre el editor');
await page.keyboard.press('Escape');
await wait(250);
const closedClean = await page.evaluate(() => document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(closedClean === true, 'Escape deja el editor cerrado para el resto del test');

// --- 2. Clic en la barra abre el editor de intensidad. ---
await page.click('.clip-filter');
await wait(300);
const editor = await page.evaluate(() => {
  const ed = document.querySelector('[data-filter-editor]');
  if (!ed) return null;
  return {
    hidden: ed.classList.contains('hidden'),
    name: ed.querySelector('[data-fe-name]')?.textContent ?? '',
    range: ed.querySelector('[data-fe-range]')?.value ?? '',
    val: ed.querySelector('[data-fe-val]')?.textContent ?? '',
    rowHidden: ed.querySelector('[data-fe-row]')?.classList.contains('hidden') ?? null,
    rect: ed.getBoundingClientRect().toJSON(),
  };
});
check(editor && !editor.hidden, 'el editor de intensidad se abre', JSON.stringify(editor));
check(editor?.name === 'Calabaza', 'el editor muestra el nombre del filtro', JSON.stringify(editor));
check(editor?.range === '100' && editor?.val === '100%', 'el editor arranca en 100%', JSON.stringify(editor));
check(editor?.rowHidden === false, 'la fila del deslizador es visible');
check(editor && editor.rect.width > 100 && editor.rect.left >= 0, 'el editor queda dentro de la línea de tiempo', JSON.stringify(editor && editor.rect));

// --- 3. Mover el deslizador cambia la intensidad en vivo. ---
await page.evaluate(() => {
  const r = document.querySelector('[data-fe-range]');
  r.value = '40';
  r.dispatchEvent(new Event('input', { bubbles: true }));
});
await wait(400);
const st3 = await clipState();
check(Math.abs((st3?.lutAmount ?? 1) - 0.4) < 0.001, 'el deslizador escribe lutAmount=0.4 en el clip', JSON.stringify(st3));
const bar3 = await barInfo();
check(bar3?.amount === '40%', 'la barra actualiza la intensidad a 40%', JSON.stringify(bar3));
const val3 = await page.evaluate(() => document.querySelector('[data-fe-val]')?.textContent);
check(val3 === '40%', 'el editor muestra 40%', String(val3));

// --- 4. El cambio es UN solo paso de deshacer (Ctrl+Z vuelve a 100%). ---
await page.evaluate(() => {
  const r = document.querySelector('[data-fe-range]');
  r.dispatchEvent(new Event('change', { bubbles: true }));
});
await wait(300);
await page.keyboard.down('Control');
await page.keyboard.press('KeyZ');
await page.keyboard.up('Control');
await wait(500);
const st4 = await clipState();
check(Math.abs((st4?.lutAmount ?? 0) - 1) < 0.001, 'Ctrl+Z revierte la intensidad al valor anterior', JSON.stringify(st4 && { lutAmount: st4.lutAmount }));

// Reabrir y dejarlo en 40% para el resto del test.
await page.keyboard.press('Escape'); // estado limpio: editor cerrado
await wait(200);
await page.click('.clip-filter');
await wait(250);
const reopened = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(reopened, 'el editor vuelve a abrirse tras Escape');
await page.evaluate(() => {
  const r = document.querySelector('[data-fe-range]');
  r.value = '40';
  r.dispatchEvent(new Event('input', { bubbles: true }));
  r.dispatchEvent(new Event('change', { bubbles: true }));
});
await wait(300);

// --- 5. "Quitar filtro" limpia la barra y el clip. ---
const editorOpenBeforeClear = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(editorOpenBeforeClear, 'el editor está abierto antes de pulsar "Quitar filtro"');
const cleared = await page.evaluate(() => {
  const btn = document.querySelector('[data-fe-clear]');
  if (!btn) return false;
  btn.click();
  return true;
});
await wait(500);
const st5 = await clipState();
const bar5 = await barInfo();
check(cleared, 'el botón "Quitar filtro" existe');
check(st5?.filter === 'none' && !st5?.lut && !st5?.glFilter, 'el clip queda sin filtro', JSON.stringify(st5));
check(bar5 === null, 'la barra desaparece del clip', JSON.stringify(bar5));
const editorHidden = await page.evaluate(() => document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(editorHidden === true, 'el editor se cierra al quitar el filtro');

// --- 6. Reaplicar y comprobar que los tiradores de recorte siguen funcionando. ---
await page.evaluate(() => {
  const card = [...document.querySelectorAll('[data-capcut-filter]')].find((c) => c.dataset.capcutName === 'Calabaza');
  card?.click();
});
await wait(700);
const before = await clipState();
const durBefore = await page.evaluate(() => {
  const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
  return c ? { start: c.start, duration: c.duration } : null;
});
const handle = await page.$('.clip-handle.right');
check(!!handle, 'los tiradores de recorte siguen en el clip');
if (handle) {
  const box = await handle.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x - 60, box.y + box.height / 2, { steps: 8 });
  await page.mouse.up();
  await wait(400);
  const durAfter = await page.evaluate(() => {
    const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
    return c ? { start: c.start, duration: c.duration } : null;
  });
  check(durAfter && durBefore && (durAfter.duration !== durBefore.duration || durAfter.start !== durBefore.start),
    'el tirador de recorte arrastra con la barra de filtro presente',
    JSON.stringify({ durBefore, durAfter }));
}
const after = await clipState();
check(!!after?.glFilter || !!after?.lut, 'el filtro sigue aplicado tras el recorte', JSON.stringify(after));
const bar6 = await barInfo();
check(bar6?.name === 'Calabaza', 'la barra sigue visible tras el recorte', JSON.stringify(bar6));

// --- 7. Escape cierra el editor. ---
await page.click('.clip-filter');
await wait(250);
await page.keyboard.press('Escape');
await wait(250);
const escHidden = await page.evaluate(() => document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(escHidden === true, 'Escape cierra el editor de intensidad');

// --- 8. Clip largo (minutos): el chip sigue visible al desplazar la línea de tiempo. ---
const clipId8 = await page.evaluate(() => window.openCUP.state.selection);
const longDone = await page.evaluate(async (id) => {
  try {
    await window.openCUP.command('updateClip', { id, patch: { duration: 60 } });
    const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === id);
    return c ? c.duration : null;
  } catch (e) { return 'ERR: ' + e.message; }
}, clipId8);
await wait(500);
const wide = await chipPos();
check(longDone === 60, 'el clip pasa a durar 60 s (más ancho que la línea de tiempo)', JSON.stringify({ longDone }));
check(wide && (wide.clipRect.right - wide.clipRect.left) > (wide.viewRect.right - wide.viewRect.left),
  'el clip es más ancho que la zona visible', JSON.stringify(wide && { clipW: Math.round(wide.clipRect.right - wide.clipRect.left), viewW: Math.round(wide.viewRect.right - wide.viewRect.left) }));
check(wide && wide.rect.left >= wide.viewRect.left - 0.5 && wide.rect.right <= wide.viewRect.right + 0.5,
  'con el clip largo el chip se ve junto a su inicio', JSON.stringify(wide && { rect: wide.rect, viewRect: wide.viewRect }));

// Desplazar la línea de tiempo hacia el interior del clip.
const scrolled = await page.evaluate(() => {
  const scroller = document.querySelector('[data-scroll]');
  scroller.scrollLeft = Math.min(2000, Math.max(0, scroller.scrollWidth - scroller.clientWidth));
  return scroller.scrollLeft;
});
await wait(450);
const deep = await chipPos();
check(typeof scrolled === 'number' && scrolled > 500, 'la línea de tiempo queda desplazada dentro del clip', JSON.stringify({ scrolled, scrollLeft: deep && deep.scrollLeft }));
check(deep && deep.rect.left >= deep.viewRect.left - 0.5 && deep.rect.right <= deep.viewRect.right + 0.5,
  'el chip se reajusta a la zona visible tras el scroll', JSON.stringify(deep && { rect: deep.rect, viewRect: deep.viewRect }));

// Volver al principio: el chip recupera su sitio junto al inicio del clip.
await page.evaluate(() => { document.querySelector('[data-scroll]').scrollLeft = 0; });
await wait(450);
const back = await chipPos();
check(back && back.rect.left >= back.viewRect.left - 0.5 && back.rect.right <= back.viewRect.right + 0.5,
  'el chip vuelve a verse junto al inicio al volver al principio', JSON.stringify(back && { rect: back.rect, viewRect: back.viewRect }));

console.log(`\nfilter-bar-test: ${pass} pass / ${fail} fail`);
console.log('errores de consola:', errors.length ? errors.slice(0, 8) : 'ninguno');
await browser.close();
process.exit(fail === 0 && errors.length === 0 ? 0 : 1);
