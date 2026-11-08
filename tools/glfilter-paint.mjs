// Comprueba que los filtros con shader GL de la línea cupcraft se aplican al
// clip y pican píxeles en el lienzo (antes de / después de pulsar la tarjeta).
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
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'GL Paint Test'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(500);

const chooser = page.waitForFileChooser();
await page.click('.media-drop');
(await chooser).accept([path.join(MEDIA, 'foto.png')]);
await wait(1200);
await page.click('.clip');
await wait(300);

const sample = () => page.evaluate(() => {
  const canvas = [...document.querySelectorAll('canvas')].find((c) => c.width > 200);
  if (!canvas) return null;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  const d = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
  let sum = 0, n = 0;
  const hist = new Array(24).fill(0);
  for (let i = 0; i < d.length; i += 4 * 131) {
    sum += d[i] + d[i + 1] + d[i + 2];
    hist[(d[i] + d[i + 1] + d[i + 2]) >> 5]++;
    n++;
  }
  return { w: canvas.width, h: canvas.height, avg: +(sum / (n * 3)).toFixed(2), hist };
});

const diff = (a, b) => {
  if (!a || !b) return null;
  let d = 0;
  for (let i = 0; i < a.hist.length; i++) d += Math.abs(a.hist[i] - b.hist[i]);
  return d;
};

await page.click('[data-panel="filters"]');
await wait(250);

const cardCount = await page.evaluate(() => document.querySelectorAll('.filter-grid [data-filter]').length);
console.log('tarjetas de la línea:', cardCount);

// Claves GL de la línea (Textura + Firma) comprobadas una a una.
const GL_KEYS = [
  'vignette', 'grain', 'chroma', 'vhs', 'bloom', 'rgbSplit',
  'radialBlur', 'directionalBlur', 'posterize',
  'burgundyHaze', 'dreamSunset', 'mistyGlow', 'pixelNoir',
  'retroLove', 'khakiDream', 'gingerSoft', 'hauntedCanvas',
];

const clear = () => page.evaluate(() => {
  const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
  if (c) { c.filter = 'none'; c.lut = null; c.glFilter = null; c.glFilterName = null; }
  window.dispatchEvent(new CustomEvent('opencup:glfx'));
});

const bad = [];
for (const key of GL_KEYS) {
  const before = errors.length;
  const b = await sample();
  const clicked = await page.evaluate((k) => {
    const card = document.querySelector(`.filter-grid [data-filter="${k}"]`);
    if (!card) return null;
    card.click();
    return card.dataset.filter;
  }, key);
  await wait(700);
  const a = await sample();
  const clip = await page.evaluate(() => {
    const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
    return c ? { glFilter: c.glFilter ?? null, glFilterName: c.glFilterName ?? null, lut: c.lut ?? null, filter: c.filter } : null;
  });
  await clear();
  await wait(400);
  const d = diff(b, a);
  const err = errors.length - before;
  const ok = clicked === key && clip && clip.glFilter && !clip.lut && d > 0 && err === 0;
  if (!ok) bad.push(key);
  console.log(`${key.padEnd(16)} clip=${JSON.stringify(clip)} deltaPixeles=${d} erroresNuevos=${err} ${ok ? 'OK' : 'FALLO'}`);
}

// Un filtro de color (sin shader) también debe aplicarse y limpiar el motor GL.
const colorBefore = await sample();
const colorClicked = await page.evaluate(() => {
  const card = document.querySelector('.filter-grid [data-filter="vivid"]');
  if (!card) return null;
  card.click();
  return card.dataset.filter;
});
await wait(500);
const colorClip = await page.evaluate(() => {
  const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
  return c ? { filter: c.filter, glFilter: c.glFilter ?? null, lut: c.lut ?? null } : null;
});
console.log(`color vivid: tarjeta="${colorClicked}" clip=${JSON.stringify(colorClip)} deltaPixeles=${diff(colorBefore, await sample())}`);
if (colorClicked !== 'vivid' || colorClip?.filter !== 'vivid' || colorClip.glFilter) bad.push('vivid');
await clear();

console.log(`\nglfilter-paint: ${GL_KEYS.length + 1 - bad.length}/${GL_KEYS.length + 1}`);
console.log(bad.length ? 'FALLO: ' + bad.join(', ') : 'OK');
console.log('errores:', errors.length ? errors.slice(0, 8) : []);
await browser.close();
process.exit(bad.length === 0 && errors.length === 0 ? 0 : 1);
