import puppeteer from 'puppeteer-core';

/* Humo end-to-end de Kokoro TTS: descarga el modelo (~92 MB) la primera vez,
   genera voz en español y la importa a la línea de tiempo.
   No forma parte de la suite rápida por su duración y tamaño. */

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const PROJECT_NAME = 'Prueba Kokoro';
const failures = [];
const check = (ok, label, detail = '') => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${label}${detail ? ` — ${detail}` : ''}`);
  if (!ok) failures.push(label);
};
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: 'new',
  args: [
    '--window-size=1440,900',
    '--force-device-scale-factor=1',
    '--hide-scrollbars',
    '--autoplay-policy=no-user-gesture-required',
  ],
  defaultViewport: { width: 1440, height: 900 },
});
const page = await browser.newPage();
const errors = [];
page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
page.on('pageerror', (error) => errors.push(error.message));

await page.goto('http://127.0.0.1:8787/', { waitUntil: 'networkidle0' });
await wait(400);

await page.evaluate((name) => {
  const card = [...document.querySelectorAll('.project-card')]
    .find((item) => item.querySelector('.project-name')?.textContent === name);
  if (card) card.querySelector('[data-project-delete]')?.click();
}, PROJECT_NAME);
if (await page.$('.modal [data-confirm]')) {
  await page.click('.modal [data-confirm]');
  await wait(400);
}

await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate((name) => { document.querySelector('[data-new-name]').value = name; }, PROJECT_NAME);
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(500);

await page.evaluate(() => { document.querySelector('[data-panel="text"]')?.click(); });
await wait(250);
await page.evaluate(() => { document.querySelector('[data-text-tab="tts"]')?.click(); });
await wait(300);
await page.evaluate(() => {
  const area = document.querySelector('[data-tts-text]');
  area.value = 'Hola, esta es la voz de cupcraft generada con Kokoro.';
  document.querySelector('[data-kokoro-voice]').value = 'ef_dora';
});
await page.evaluate(() => { document.querySelector('[data-tts-generate]')?.click(); });

console.log('Generando voz con Kokoro (la primera vez descarga el modelo)…');
let outcome = 'timeout';
try {
  await page.waitForFunction(() => {
    const audioClips = [...document.querySelectorAll('.clip.audio')].length;
    const toast = [...document.querySelectorAll('#toasts .toast, #toasts > div')]
      .map((node) => node.textContent || '').join(' ');
    if (audioClips > 0) return true;
    if (/Kokoro:/i.test(toast)) return true;
    if (/Voz generada/i.test(toast)) return true;
    return false;
  }, { timeout: 450000, polling: 500 });
  const audioClips = await page.evaluate(() => document.querySelectorAll('.clip.audio').length);
  const toast = await page.evaluate(() => [...document.querySelectorAll('#toasts > div')].map((n) => n.textContent).join(' | '));
  outcome = audioClips > 0 ? 'imported' : `toast: ${toast}`;
} catch {
  outcome = 'timeout';
}
console.log('resultado:', outcome);

const data = await page.evaluate(async () => {
  const store = await import('/src/store.js');
  const audioTrack = store.state.project.tracks.find((track) => track.type === 'audio');
  const media = [...store.state.media.values()].map((item) => ({
    kind: item.kind, name: item.name, duration: item.duration,
  }));
  return {
    clips: audioTrack?.clips.map((clip) => ({ kind: clip.kind, duration: clip.duration, name: clip.name })) || [],
    media,
    progressVisible: Boolean(document.querySelector('[data-tts-progress]') && !document.querySelector('[data-tts-progress]')?.classList.contains('hidden')),
    generateDisabled: Boolean(document.querySelector('[data-tts-generate]')?.disabled),
  };
});
console.log('estado', JSON.stringify(data));

check(outcome === 'imported', 'Kokoro genera el audio y lo importa al proyecto', outcome);
check(data.clips.some((clip) => clip.kind === 'audio' && clip.duration > 0.4), 'aparece un clip de audio en la pista', JSON.stringify(data.clips));
check(!data.progressVisible && !data.generateDisabled, 'la barra de progreso se oculta y el botón se reactiva');
check(errors.length === 0, 'sin errores de consola', JSON.stringify(errors.slice(0, 5)));

await browser.close();
if (failures.length) {
  console.log(`FALLOS: ${failures.length}`);
  failures.forEach((item) => console.log(` - ${item}`));
  process.exit(1);
}
console.log('KOKORO TTS TEST PASSED');
