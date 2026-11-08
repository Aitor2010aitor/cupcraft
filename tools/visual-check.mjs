import fs from 'node:fs';
import path from 'node:path';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:8787/';
const MEDIA = path.resolve('tools/fixtures');
const OUT = 'C:/Users/aitor/AppData/Local/Temp/opencode/cupcraft-shots';
fs.mkdirSync(OUT, { recursive: true });

const browser = await puppeteer.launch({ executablePath: CHROME, headless: true, args: ['--window-size=1440,900'] });
const page = await browser.newPage();
await page.setViewport({ width: 1440, height: 900 });
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

await page.goto(URL, { waitUntil: 'networkidle2' });
await wait(800);
await page.screenshot({ path: path.join(OUT, 'home-logo.png') });

await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Visual Check'; });
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
await page.evaluate(() => {
  document.querySelector('.filter-grid [data-filter="burgundyHaze"]')?.click();
});
await wait(700);
await page.screenshot({ path: path.join(OUT, 'clip-filter-bar.png') });

// El indicador vive en su propia fila "Filtros", FUERA de las pistas de vídeo.
const rowEl = await page.$('.track-row.filter-row');
if (rowEl) await rowEl.screenshot({ path: path.join(OUT, 'clip-filter-chip.png') });
const clipEl = await page.$('.clip.has-filter');
if (clipEl) await clipEl.screenshot({ path: path.join(OUT, 'clip-filter-clip.png') });

// …y captura del bloque tras abrir su editor.
const chipEl = await page.$('.clip-filter');
if (chipEl) {
  await chipEl.click();
  await wait(400);
  const row2 = await page.$('.track-row.filter-row');
  if (row2) await row2.screenshot({ path: path.join(OUT, 'clip-filter-chip-moved.png') });
  await page.keyboard.press('Escape');
  await wait(300);
}

await page.click('.clip-filter');
await wait(300);
await page.evaluate(() => {
  const r = document.querySelector('[data-fe-range]');
  r.value = '65';
  r.dispatchEvent(new Event('input', { bubbles: true }));
});
await wait(400);
await page.screenshot({ path: path.join(OUT, 'filter-editor.png') });

await browser.close();
console.log('capturas en', OUT);
