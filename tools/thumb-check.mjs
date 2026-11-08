// Comprueba que las miniaturas del catálogo CapCut se cargan de verdad en la
// app empaquetada (servida por cupcraft.exe en 8787): ninguna imagen rota y
// portadas distintas entre sí en cada panel.
//
// Uso: node tools/thumb-check.mjs
import path from 'node:path';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:8787/';
const MEDIA = path.resolve('tools/fixtures');

const browser = await puppeteer.launch({ executablePath: CHROME, headless: true, args: ['--window-size=1440,900'] });
const page = await browser.newPage();
await page.setViewport({ width: 1440, height: 900 });

const wait = (ms) => new Promise((r) => setTimeout(r, ms));
let pass = 0;
let fail = 0;
const check = (ok, label, extra = '') => {
  if (ok) { pass++; console.log('  ok  ' + label); }
  else { fail++; console.log('  FALLA ' + label + (extra ? ' → ' + extra : '')); }
};

await page.goto(URL, { waitUntil: 'networkidle2' });
await wait(600);

// Proyecto nuevo (mismo camino que el resto de tests).
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Thumb Check'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(600);

// Un clip de imagen, por si algún panel necesita selección.
const chooser = page.waitForFileChooser();
await page.click('.media-drop');
(await chooser).accept([path.join(MEDIA, 'foto.png')]);
await wait(1200);
const hasClip = await page.evaluate(() => !!document.querySelector('.clip'));
check(hasClip, 'hay un clip en la línea de tiempo');

// Panel → pestaña de origen CapCut → rejilla. (El panel Filtros ya no usa el
// catálogo CapCut: su línea propia es CSS puro, sin miniaturas.)
const panels = [
  { id: 'effects', tab: '[data-effect-source="capcut"]', attr: 'data-capcut-effect' },
  { id: 'transitions', tab: '[data-transition-source="capcut"]', attr: 'data-capcut-transition' },
  { id: 'stickers', tab: '[data-sticker-source="capcut"]', attr: 'data-capcut-sticker' },
  { id: 'text', tab: '[data-text-tab="capcut"]', attr: 'data-capcut-text' },
];

// Desplaza el contenedor scrolleable de la rejilla para disparar las
// miniaturas perezosas (loading="lazy") y espera a que terminen de cargar.
async function scrollAndLoad(attr) {
  for (let round = 0; round < 8; round++) {
    const done = await page.evaluate((selector) => {
      const grid = document.querySelector(`${selector}`);
      if (!grid) return true;
      let node = grid;
      while (node && node !== document.body) {
        if (node.scrollHeight > node.clientHeight + 4) {
          node.scrollTop = Math.min(node.scrollTop + node.clientHeight, node.scrollHeight);
          break;
        }
        node = node.parentElement;
      }
      return false;
    }, `[${attr}]`);
    await wait(450);
    if (done) break;
  }
  // Espera a que no quede ninguna imagen a medias.
  for (let i = 0; i < 20; i++) {
    const pending = await page.evaluate((selector) =>
      [...document.querySelectorAll(`${selector} img`)].filter((i) => !i.complete).length, `[${attr}]`);
    if (!pending) break;
    await wait(400);
  }
}

for (const { id, tab, attr } of panels) {
  const opened = await page.evaluate((panelId) => {
    const btn = document.querySelector(`[data-panel="${panelId}"]`);
    if (!btn) return false;
    btn.click();
    return true;
  }, id);
  await wait(600);
  if (!opened) { check(false, `${id}: se abre el panel`); continue; }

  const tabOk = await page.evaluate((selector) => {
    const btn = document.querySelector(selector);
    if (!btn) return false;
    btn.click();
    return true;
  }, tab);
  await wait(600);
  check(tabOk, `${id}: pestaña CapCut seleccionada`);

  await scrollAndLoad(attr);

  const stats = await page.evaluate((selector) => {
    const imgs = [...document.querySelectorAll(`${selector} img`)];
    const loaded = imgs.filter((i) => i.complete && i.naturalWidth > 0);
    const broken = imgs.filter((i) => i.complete && i.naturalWidth === 0);
    const pending = imgs.filter((i) => !i.complete);
    const srcs = loaded.map((i) => i.currentSrc || i.getAttribute('src'));
    return {
      total: imgs.length,
      loaded: loaded.length,
      broken: broken.length,
      pending: pending.length,
      distinct: new Set(srcs).size,
      sampleBroken: broken.slice(0, 3).map((i) => i.getAttribute('src')),
    };
  }, `[${attr}]`);

  check(stats.total > 0, `${id}: hay miniaturas en la rejilla`, JSON.stringify(stats));
  check(stats.broken === 0, `${id}: ninguna imagen rota`, JSON.stringify(stats));
  check(stats.loaded > 0 && stats.distinct > 1, `${id}: las portadas se cargan y son distintas`, JSON.stringify(stats));

  // Muestra repartida de srcs: el servidor debe responder 200 con imagen.
  const probe = await page.evaluate(async (selector, size) => {
    const srcs = [...new Set([...document.querySelectorAll(`${selector} img`)].map((i) => i.getAttribute('src')))];
    const step = Math.max(1, Math.floor(srcs.length / size));
    const sample = srcs.filter((_, i) => i % step === 0).slice(0, size);
    const bad = [];
    for (const src of sample) {
      try {
        const res = await fetch(src, { method: 'GET' });
        const type = res.headers.get('content-type') || '';
        if (res.status !== 200 || !type.startsWith('image/')) bad.push(`${src} → ${res.status} ${type}`);
      } catch (e) {
        bad.push(`${src} → ${e.message}`);
      }
    }
    return { sampled: sample.length, bad };
  }, `[${attr}]`, 40);
  check(probe.bad.length === 0, `${id}: ${probe.sampled} srcs de muestra responden 200 imagen`,
    JSON.stringify(probe.bad.slice(0, 3)));
}

console.log(`\nthumb-check: ${pass} pass / ${fail} fail`);
await browser.close();
process.exit(fail ? 1 : 0);
