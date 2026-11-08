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
  const id = bar.dataset.clipFilter;
  const clip = [...document.querySelectorAll('.clip')].find((c) => c.dataset.clipId === id);
  return {
    name: bar.querySelector('.clip-filter-name')?.textContent ?? '',
    amount: bar.querySelector('.clip-filter-amount')?.textContent ?? '',
    hasFilterClass: clip?.classList.contains('has-filter') ?? false,
    insideClip: !!bar.closest('.clip'),
    inOwnRow: !!bar.closest('.track-row.filter-row'),
  };
});

// --- 1. Aplicar un filtro de la línea cupcraft y comprobar la barra sobre el clip. ---
await page.click('[data-panel="filters"]');
await wait(250);
const applied = await page.evaluate(() => {
  const card = document.querySelector('[data-filter="burgundyHaze"]');
  if (!card) return null;
  card.click();
  return card.dataset.filter;
});
await wait(700);
check(applied === 'burgundyHaze', 'se aplica el filtro de la línea "Bruma borgoña"');

const bar1 = await barInfo();
check(!!bar1, 'la barra .clip-filter aparece en la línea de tiempo', JSON.stringify(bar1));
check(bar1?.name === 'Bruma borgoña', 'la barra muestra el nombre del filtro', JSON.stringify(bar1));
check(bar1?.amount === '100%', 'la barra muestra la intensidad 100%', JSON.stringify(bar1));
check(bar1?.hasFilterClass === true, 'el clip tiene la clase has-filter');
check(bar1?.insideClip === false, 'la barra NO está dentro del clip (está fuera de las pistas de vídeo)', JSON.stringify(bar1));
check(bar1?.inOwnRow === true, 'la barra vive en su propia fila "Filtros"', JSON.stringify(bar1));

// La fila "Filtros" se coloca justo debajo de la pista de vídeo (y su etiqueta también).
const rowOrder = await page.evaluate(() => {
  const videoId = window.openCUP.state.project.tracks.find((t) => t.type === 'video')?.id || 'track_video';
  const rows = [...document.querySelectorAll('[data-tracks] > .track-row')].map((r) => (
    r.classList.contains('filter-row') ? 'filter'
      : r.classList.contains('fx-row') ? 'fx'
        : r.dataset.trackId || 'track'
  ));
  const labelNames = [...document.querySelectorAll('[data-labels] .track-label')].map((l) => (
    l.classList.contains('fx-label') ? 'fx'
      : l.classList.contains('filter-label') ? 'filter'
        : l.dataset.trackId || l.querySelector('.t-name')?.textContent || 'label'
  ));
  const labels = [...document.querySelectorAll('[data-labels] .track-label')];
  const fi = labels.findIndex((l) => l.classList.contains('filter-label'));
  const filterLabelVisible = fi >= 0 ? labels[fi].getBoundingClientRect().height > 0 : false;
  return { videoId, rows, labelNames, filterLabelVisible };
});
check(rowOrder.rows.includes('filter') && rowOrder.rows.indexOf('filter') === rowOrder.rows.indexOf(rowOrder.videoId) + 1,
  'la fila "Filtros" queda justo debajo de la pista de vídeo', JSON.stringify(rowOrder.rows));
check(rowOrder.labelNames.includes('filter') && rowOrder.labelNames.indexOf('filter') === rowOrder.labelNames.indexOf(rowOrder.videoId) + 1,
  'la etiqueta "Filtros" acompaña a su fila en la columna de etiquetas', JSON.stringify(rowOrder.labelNames));
check(rowOrder.filterLabelVisible === true, 'la etiqueta "Filtros" es visible');

// --- 1b. El indicador es un bloque sólido en su propia fila, cubriendo todo el clip. ---
const blockPos = () => page.evaluate(() => {
  const block = document.querySelector('.clip-filter');
  const scroller = document.querySelector('[data-scroll]');
  if (!block || !scroller) return null;
  const id = block.dataset.clipFilter;
  const clip = [...document.querySelectorAll('.clip')].find((c) => c.dataset.clipId === id);
  if (!clip) return null;
  const row = block.closest('.track-row');
  const br = block.getBoundingClientRect();
  const lr = clip.getBoundingClientRect();
  const rr = row.getBoundingClientRect();
  const vr = scroller.getBoundingClientRect();
  return {
    scrollLeft: scroller.scrollLeft,
    isBlock: block.classList.contains('filter-block'),
    hasRemove: !!block.querySelector('[data-filter-remove]'),
    rect: { left: br.left, right: br.right, top: br.top, bottom: br.bottom, width: br.width },
    clipRect: { left: lr.left, right: lr.right, top: lr.top, bottom: lr.bottom, width: lr.width },
    rowRect: { left: rr.left, right: rr.right, top: rr.top, bottom: rr.bottom },
    viewRect: { left: vr.left, right: vr.right, top: vr.top, bottom: vr.bottom },
  };
});

const pos0 = await blockPos();
check(pos0?.isBlock === true, 'el indicador es un bloque sólido (.filter-block)', JSON.stringify(pos0 && { isBlock: pos0.isBlock }));
check(pos0?.hasRemove === true, 'el bloque lleva su botón × para quitar el filtro');
check(pos0 && Math.abs(pos0.rect.left - pos0.clipRect.left) <= 1.5 && Math.abs(pos0.rect.width - pos0.clipRect.width) <= 2,
  'el bloque empieza exactamente donde empieza el clip', JSON.stringify(pos0 && { rect: pos0.rect, clipRect: pos0.clipRect }));
check(pos0 && pos0.rect.top >= pos0.rowRect.top - 0.5 && pos0.rect.bottom <= pos0.rowRect.bottom + 0.5,
  'el bloque queda dentro de su propia fila "Filtros"', JSON.stringify(pos0 && { rect: pos0.rect, rowRect: pos0.rowRect }));
check(pos0 && pos0.rect.left < pos0.viewRect.right && pos0.rect.right > pos0.viewRect.left,
  'el bloque es visible dentro de la línea de tiempo', JSON.stringify(pos0 && { rect: pos0.rect, viewRect: pos0.viewRect }));

const editorBeforeClick = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(!editorBeforeClick, 'el editor está cerrado antes de hacer clic en el bloque');

// Un clic sobre el bloque (no sobre la ×) abre el editor de intensidad.
await page.click('.clip-filter');
await wait(300);
const clickOpens = await page.evaluate(() => !document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(clickOpens, 'un clic sobre el bloque abre el editor de intensidad');
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
check(editor?.name === 'Bruma borgoña', 'el editor muestra el nombre del filtro', JSON.stringify(editor));
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
check(Math.abs((st3?.glFilterAmount ?? 1) - 0.4) < 0.001, 'el deslizador escribe glFilterAmount=0.4 en el clip', JSON.stringify(st3));
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
check(Math.abs((st4?.glFilterAmount ?? 0) - 1) < 0.001, 'Ctrl+Z revierte la intensidad al valor anterior', JSON.stringify(st4 && { glFilterAmount: st4.glFilterAmount }));

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
  document.querySelector('[data-filter="burgundyHaze"]')?.click();
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
check(bar6?.name === 'Bruma borgoña', 'la barra sigue visible tras el recorte', JSON.stringify(bar6));
const span6 = await blockPos();
check(span6 && Math.abs(span6.rect.left - span6.clipRect.left) <= 1.5 && Math.abs(span6.rect.width - span6.clipRect.width) <= 2,
  'tras recortar el bloque vuelve a cubrir exactamente el clip', JSON.stringify(span6 && { rect: span6.rect, clipRect: span6.clipRect }));

// --- 7. Escape cierra el editor. ---
await page.click('.clip-filter');
await wait(250);
await page.keyboard.press('Escape');
await wait(250);
const escHidden = await page.evaluate(() => document.querySelector('[data-filter-editor]')?.classList.contains('hidden'));
check(escHidden === true, 'Escape cierra el editor de intensidad');

// --- 8. Clip largo (minutos): el bloque cubre el tramo completo y se ve siempre. ---
const clipId8 = await page.evaluate(() => window.openCUP.state.selection);
const longDone = await page.evaluate(async (id) => {
  try {
    await window.openCUP.command('updateClip', { id, patch: { duration: 60 } });
    const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === id);
    return c ? c.duration : null;
  } catch (e) { return 'ERR: ' + e.message; }
}, clipId8);
await wait(500);
const wide = await blockPos();
check(longDone === 60, 'el clip pasa a durar 60 s (más ancho que la línea de tiempo)', JSON.stringify({ longDone }));
check(wide && (wide.clipRect.right - wide.clipRect.left) > (wide.viewRect.right - wide.viewRect.left),
  'el clip es más ancho que la zona visible', JSON.stringify(wide && { clipW: Math.round(wide.clipRect.right - wide.clipRect.left), viewW: Math.round(wide.viewRect.right - wide.viewRect.left) }));
check(wide && Math.abs(wide.rect.left - wide.clipRect.left) <= 1.5 && Math.abs(wide.rect.width - wide.clipRect.width) <= 2,
  'con el clip largo el bloque sigue cubriendo todo su tramo', JSON.stringify(wide && { rect: wide.rect, clipRect: wide.clipRect }));
check(wide && wide.rect.left < wide.viewRect.right && wide.rect.right > wide.viewRect.left,
  'con el clip largo el bloque se ve en la línea de tiempo', JSON.stringify(wide && { rect: wide.rect, viewRect: wide.viewRect }));

// Desplazar la línea de tiempo hacia el interior del clip.
const scrolled = await page.evaluate(() => {
  const scroller = document.querySelector('[data-scroll]');
  scroller.scrollLeft = Math.min(2000, Math.max(0, scroller.scrollWidth - scroller.clientWidth));
  return scroller.scrollLeft;
});
await wait(450);
const deep = await blockPos();
check(typeof scrolled === 'number' && scrolled > 500, 'la línea de tiempo queda desplazada dentro del clip', JSON.stringify({ scrolled, scrollLeft: deep && deep.scrollLeft }));
check(deep && deep.rect.left < deep.viewRect.right && deep.rect.right > deep.viewRect.left,
  'el bloque sigue visible tras el scroll', JSON.stringify(deep && { rect: deep.rect, viewRect: deep.viewRect }));

// Volver al principio: el bloque vuelve a cubrir el clip desde su inicio.
await page.evaluate(() => { document.querySelector('[data-scroll]').scrollLeft = 0; });
await wait(450);
const back = await blockPos();
check(back && back.rect.left < back.viewRect.right && back.rect.right > back.viewRect.left,
  'el bloque sigue visible al volver al principio', JSON.stringify(back && { rect: back.rect, viewRect: back.viewRect }));

// --- 9. La × del bloque quita el filtro y la fila desaparece. ---
const removeBtn = await page.$('.filter-block [data-filter-remove]');
check(!!removeBtn, 'el bloque tiene su botón ×');
if (removeBtn) {
  await removeBtn.click();
  await wait(600);
  const st9 = await clipState();
  const rowGone = await page.evaluate(() => !document.querySelector('.track-row.filter-row'));
  check(st9?.filter === 'none' && !st9?.lut && !st9?.glFilter, 'la × limpia el filtro del clip', JSON.stringify(st9));
  check(rowGone, 'la fila "Filtros" desaparece al no haber ningún filtro');
}

console.log(`\nfilter-bar-test: ${pass} pass / ${fail} fail`);
console.log('errores de consola:', errors.length ? errors.slice(0, 8) : 'ninguno');
await browser.close();
process.exit(fail === 0 && errors.length === 0 ? 0 : 1);
