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

const families = await page.evaluate(() => {
  const cards = [...document.querySelectorAll('[data-capcut-filter]')];
  return cards.length;
});
console.log('tarjetas de filtro CapCut:', families);

const run = async (match, label) => {
  const before = await sample();
  const clicked = await page.evaluate((name) => {
    const cards = [...document.querySelectorAll('[data-capcut-filter]')];
    const card = cards.find((c) => c.dataset.capcutName === name);
    if (!card) return null;
    card.click();
    return card.dataset.capcutName;
  }, match);
  await wait(700);
  const after = await sample();
  const clip = await page.evaluate(() => {
    const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
    return c ? { lut: c.lut ?? null, glFilter: c.glFilter ?? null, filter: c.filter } : null;
  });
  console.log(`${label}: tarjeta="${clicked}" clip=${JSON.stringify(clip)} deltaPixeles=${diff(before, after)}`);
  // limpiar
  await page.evaluate(() => {
    const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
    if (c) { c.filter = 'none'; c.lut = null; c.glFilter = null; c.glFilterName = null; }
    window.dispatchEvent(new CustomEvent('opencup:glfx'));
  });
  await wait(500);
};

await run('Fiesta metálica', 'curve ');
await run('Espejismo somnoliento', 'grain ');
await run('Calabaza', 'LUT   ');

// Los 8 filtros "other" sin LUT: ahora con su propio shader portado.
const OTHERS = [
  ['Lienzo embrujado', 'lienzo'],
  ['Cariño retro', 'retro '],
  ['Sueño caqui', 'caqui '],
  ['Jengibre suave', 'jengi '],
  ['Bruma borgoña', 'bruma '],
  ['Atardecer de ensueño', 'atard '],
  ['Brillo brumoso', 'brillo'],
  ['Píxel estilo noir', 'noir  '],
];
const bad = [];
for (const [name, label] of OTHERS) {
  const before = errors.length;
  const res = await (async () => {
    const b = await sample();
    const clicked = await page.evaluate((n) => {
      const card = [...document.querySelectorAll('[data-capcut-filter]')].find((c) => c.dataset.capcutName === n);
      if (!card) return null;
      card.click();
      return card.dataset.capcutName;
    }, name);
    await wait(700);
    const a = await sample();
    const clip = await page.evaluate(() => {
      const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
      return c ? { glFilter: c.glFilter ?? null, glFilterName: c.glFilterName ?? null, lut: c.lut ?? null } : null;
    });
    await page.evaluate(() => {
      const c = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((x) => x.id === window.openCUP.state.selection);
      if (c) { c.filter = 'none'; c.lut = null; c.glFilter = null; c.glFilterName = null; }
      window.dispatchEvent(new CustomEvent('opencup:glfx'));
    });
    await wait(400);
    return { clicked, clip, d: diff(b, a), err: errors.length - before };
  })();
  const ok = res.clicked && res.clip && res.clip.glFilter && res.d > 0 && res.err === 0;
  if (!ok) bad.push(name);
  console.log(`${label}: tarjeta="${res.clicked}" glFilter=${res.clip && res.clip.glFilter} deltaPixeles=${res.d} erroresNuevos=${res.err} ${ok ? 'OK' : 'FALLO'}`);
}

console.log('otros 8 →', bad.length ? 'FALLO: ' + bad.join(', ') : 'OK (8/8)');

console.log('errores:', errors.length ? errors.slice(0, 8) : []);
await browser.close();
