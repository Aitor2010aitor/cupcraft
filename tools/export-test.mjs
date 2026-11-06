import puppeteer from 'puppeteer-core';
import { mkdir, readdir, rm, stat } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';

const exec = promisify(execFile);
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const DOWNLOADS = path.resolve('shots/export-test');
await rm(DOWNLOADS, { recursive: true, force: true });
await mkdir(DOWNLOADS, { recursive: true });

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
const cdp = await page.createCDPSession();
await cdp.send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: DOWNLOADS, eventsEnabled: true });
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

await page.goto('http://127.0.0.1:8787/', { waitUntil: 'networkidle0' });
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Export test'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(400);

// A 3-second text card with a vivid background layer is enough to exercise
// canvas rendering, frame pacing and audio-less WebM muxing.
await page.click('[data-panel="text"]');
await page.click('[data-preset="0"]');
await wait(300);
await page.evaluate(() => {
  const area = document.querySelector('[data-text-content]');
  area.focus();
  area.select();
});
await page.keyboard.type('Exportación cupcraft');
await page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.find((track) => track.type === 'text').clips[0];
  clip.duration = 3;
  window.openCUP.state.project.duration = 3;
});
await wait(200);

await page.click('[data-act="export"]');
await page.waitForSelector('.modal');
await page.select('[data-export-resolution]', '480p');
await page.select('[data-export-quality]', 'alta');
await page.evaluate(() => { document.querySelector('[data-export-name]').value = 'cupcraft-export-test'; });
await page.click('[data-export-start]');
console.log('export started');

let finished = false;
let label = '';
const startedAt = Date.now();
while (Date.now() - startedAt < 30000) {
  label = await page.evaluate(() => document.querySelector('[data-export-label]')?.textContent || '');
  if (/Listo|Error|No se pudo|error/i.test(label)) { finished = true; break; }
  await wait(400);
}
console.log('label', label, 'elapsed', Date.now() - startedAt);
await wait(1000);
const files = await readdir(DOWNLOADS);
console.log('files', files);
const webm = files.find((file) => file.endsWith('.webm'));
if (!webm) {
  console.error('FAIL no webm downloaded');
  console.error('errors', errors);
  await browser.close();
  process.exit(1);
}
const filePath = path.join(DOWNLOADS, webm);
const info = await stat(filePath);
console.log('size', info.size);
const { stdout } = await exec('ffprobe', [
  '-v', 'error',
  '-show_entries', 'stream=width,height,codec_name,duration',
  '-show_entries', 'format=duration',
  '-of', 'json', filePath,
]);
console.log('ffprobe', stdout);
const parsed = JSON.parse(stdout);
const stream = parsed.streams?.[0];
const duration = Number(parsed.format?.duration || stream?.duration || 0);
const ok = info.size > 10000
  && stream?.width === 854
  && stream?.height === 480
  && Math.abs(duration - 3) < 0.9
  && errors.length === 0;
console.log(ok ? 'EXPORT TEST PASSED' : 'EXPORT TEST FAILED');
if (!ok) console.log('errors', errors);
await browser.close();
process.exit(ok ? 0 : 1);
