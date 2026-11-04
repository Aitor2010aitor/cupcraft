// Lista los recursos que devuelven 404 en la app (sirve lo que haya en 8787).
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:8787/';
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const browser = await puppeteer.launch({ executablePath: CHROME, headless: true, args: ['--window-size=1440,900'] });
const page = await browser.newPage();
await page.setViewport({ width: 1440, height: 900 });
const bad = new Map();
page.on('response', (r) => {
  if (r.status() >= 400) {
    const u = r.url().replace(/^https?:\/\/[^/]+/, '');
    bad.set(u, (bad.get(u) || 0) + 1);
  }
});
await page.goto(URL, { waitUntil: 'networkidle2' });
await wait(700);
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = '404 check'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(600);

for (const panel of ['filters', 'effects', 'transitions', 'stickers', 'text']) {
  const ok = await page.evaluate((p) => {
    const b = document.querySelector(`[data-panel="${p}"]`);
    if (b) { b.click(); return true; }
    return false;
  }, panel);
  if (!ok) { console.log('sin panel', panel); continue; }
  await wait(900);
  // fuerza la carga de las tarjetas visibles
  await page.evaluate(() => { const el = document.querySelector('.fx-grid, .effect-grid, .panel-body, .rail-section'); if (el) el.scrollTop = 0; });
  await wait(600);
}

const entries = [...bad.entries()].sort((a, b) => b[1] - a[1]);
console.log('recursos con error:', entries.length, '| peticiones fallidas:', entries.reduce((s, e) => s + e[1], 0));
const byPattern = new Map();
for (const [u, n] of entries) {
  const key = u.replace(/\/[^/]+$/, '/…');
  byPattern.set(key, (byPattern.get(key) || 0) + n);
}
for (const [k, n] of [...byPattern.entries()].sort((a, b) => b[1] - a[1]).slice(0, 20)) console.log(`  ${n}\t${k}`);
console.log('--- ejemplos ---');
for (const [u, n] of entries.slice(0, 15)) console.log(`  ${n}  ${u}`);
await browser.close();
