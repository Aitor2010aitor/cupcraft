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
await page.goto(URL, { waitUntil: 'networkidle2' });
await wait(600);

// Proyecto nuevo
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'GL Filter Test'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(500);

// Importar imagen
const chooser = page.waitForFileChooser();
await page.click('.media-drop');
(await chooser).accept([path.join(MEDIA, 'foto.png')]);
await wait(1200);

// Seleccionar el clip
await page.click('.clip');
await wait(300);

const snap = async () => page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((c) => c.id === window.openCUP.state.selection);
  return clip ? { id: clip.id, filter: clip.filter, lut: clip.lut ?? null, glFilter: clip.glFilter ?? null, glFilterName: clip.glFilterName ?? null } : null;
});

console.log('antes:', JSON.stringify(await snap()));

// Aplicar un filtro CapCut de familia curve
await page.click('[data-panel="filters"]');
await wait(200);
const applied = await page.evaluate(() => {
  const cards = [...document.querySelectorAll('[data-capcut-filter]')];
  const card = cards.find((c) => /curve|grain/i.test(c.dataset.capcutFilter || '')) || cards[0];
  if (!card) return null;
  card.click();
  return card.dataset.capcutFilter;
});
await wait(400);
console.log('tarjeta pulsada:', applied);
const after = await snap();
console.log('después:', JSON.stringify(after));

// ¿Pinta el canvas? (muestrear pixeles antes/después ya no es posible; comprobamos que
// el shader GL no falla y que el lienzo tiene contenido no uniforme)
const painted = await page.evaluate(() => {
  const canvas = document.querySelector('.preview-canvas, canvas.stage-canvas, .stage canvas');
  if (!canvas) return { ok: false, why: 'sin canvas' };
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return { ok: false, why: 'sin ctx2d' };
  const w = canvas.width, h = canvas.height;
  const data = ctx.getImageData(0, 0, w, h).data;
  let min = 255, max = 0;
  for (let i = 0; i < data.length; i += 4 * 97) {
    const v = data[i];
    if (v < min) min = v;
    if (v > max) max = v;
  }
  return { ok: true, w, h, min, max, spread: max - min };
});
console.log('lienzo:', JSON.stringify(painted));

// Probar también un filtro LUT y uno grain explícitamente por API
const api = await page.evaluate(() => {
  const items = window.openCUP.CAPCUT_FILTER_ITEMS || null;
  return items ? items.length : 'no expuesto';
});
console.log('CAPCUT_FILTER_ITEMS:', api);

console.log('errores:', errors.length ? errors.slice(0, 8) : []);
await browser.close();
