// Hoja de contactos de los 8 filtros "other" portados: aplica cada uno al clip
// y pega su fotograma en una sola imagen para revisar el aspecto.
import path from 'node:path';
import fs from 'node:fs';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:8787/';
const MEDIA = path.resolve('tools/fixtures');
const OUT = process.argv[2] || 'C:/Users/aitor/AppData/Local/Temp/opencode/glfilter-look.png';

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
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'GL Look Test'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(500);
const chooser = page.waitForFileChooser();
await page.click('.media-drop');
(await chooser).accept([path.join(MEDIA, 'foto.png')]);
await wait(1200);
await page.click('.clip');
await wait(300);
await page.click('[data-panel="filters"]');
await wait(300);

const NAMES = ['Lienzo embrujado', 'Cariño retro', 'Sueño caqui', 'Jengibre suave',
  'Bruma borgoña', 'Atardecer de ensueño', 'Brillo brumoso', 'Píxel estilo noir'];

const grab = () => page.evaluate(() => {
  const canvas = [...document.querySelectorAll('canvas')].find((c) => c.width > 200);
  return canvas ? canvas.toDataURL('image/png') : null;
});

const clear = () => page.evaluate(() => {
  const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
  if (c) { c.filter = 'none'; c.lut = null; c.glFilter = null; c.glFilterName = null; }
  window.dispatchEvent(new CustomEvent('opencup:glfx'));
});

const tiles = [];
for (const name of NAMES) {
  const clicked = await page.evaluate((n) => {
    const card = [...document.querySelectorAll('[data-capcut-filter]')].find((c) => c.dataset.capcutName === n);
    if (!card) return null;
    card.click();
    return card.dataset.capcutName;
  }, name);
  await wait(700);
  tiles.push({ name, img: await grab(), ok: !!clicked });
  await clear();
  await wait(350);
}

const sheet = await browser.newPage();
await sheet.setViewport({ width: 1240, height: 700 });
await sheet.setContent(`<!doctype html><meta charset="utf-8"><style>
body{margin:0;background:#111;color:#eee;font:13px sans-serif;display:grid;grid-template-columns:repeat(4,1fr);gap:6px;padding:6px}
figure{margin:0}figcaption{padding:3px 0}img{width:100%;display:block;background:#000}
</style>${tiles.map((t) => `<figure><img src="${t.img}"><figcaption>${t.ok ? '' : '✗ '}${t.name}</figcaption></figure>`).join('')}`);
await wait(400);
await sheet.screenshot({ path: OUT });
console.log('hoja:', OUT, '| tiles:', tiles.filter((t) => t.img).length + '/' + tiles.length);
console.log('errores:', errors.length ? errors.slice(0, 6) : []);
await browser.close();
