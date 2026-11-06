import puppeteer from 'puppeteer-core';
import { mkdir, readdir, rm, stat } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';

const exec = promisify(execFile);
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const MEDIA = path.resolve('tools/fixtures');
const DOWNLOADS = path.resolve('shots/audio-export-test');
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
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Export con audio'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(400);

const chooserPromise = page.waitForFileChooser();
await page.click('.media-drop');
const chooser = await chooserPromise;
await chooser.accept([path.join(MEDIA, 'clip-video.webm'), path.join(MEDIA, 'musica.wav')]);
await page.waitForFunction(() => document.querySelectorAll('.clip').length === 2, { timeout: 40000 });
await wait(600);

const tracks = await page.evaluate(() => window.openCUP.state.project.tracks.map((track) => ({
  type: track.type,
  clips: track.clips.map((clip) => ({ kind: clip.kind, duration: clip.duration, mediaId: clip.mediaId })),
})));
console.log('tracks', JSON.stringify(tracks));

// Recorta ambos clips a 3 s para que la exportación en tiempo real sea rápida.
await page.evaluate(() => {
  for (const track of window.openCUP.state.project.tracks) {
    for (const clip of track.clips) {
      if (clip.kind === 'video' || clip.kind === 'audio') clip.duration = 3;
    }
  }
});
await wait(200);

await page.click('[data-act="export"]');
await page.waitForSelector('.modal');
await page.select('[data-export-resolution]', '480p');
await page.select('[data-export-quality]', 'media');
await page.evaluate(() => { document.querySelector('[data-export-name]').value = 'cupcraft-audio-test'; });
await page.click('[data-export-start]');
console.log('export started');

let label = '';
const startedAt = Date.now();
while (Date.now() - startedAt < 30000) {
  label = await page.evaluate(() => document.querySelector('[data-export-label]')?.textContent || '');
  if (/Listo|Error|No se pudo|error/i.test(label)) break;
  await wait(400);
}
console.log('label', label, 'elapsed', Date.now() - startedAt);
await wait(1200);
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
  '-show_entries', 'stream=index,codec_type,codec_name,width,height,channels,sample_rate,duration',
  '-show_entries', 'format=duration',
  '-of', 'json', filePath,
]);
console.log('ffprobe', stdout);
const parsed = JSON.parse(stdout);
const video = parsed.streams?.find((stream) => stream.codec_type === 'video');
const audio = parsed.streams?.find((stream) => stream.codec_type === 'audio');
const duration = Number(parsed.format?.duration || video?.duration || 0);

let meanVolume = null;
try {
  const result = await exec('ffmpeg', ['-v', 'info', '-i', filePath, '-map', '0:a:0', '-af', 'volumedetect', '-f', 'null', '-']);
  const match = /mean_volume:\s*(-?[\d.]+) dB/.exec(`${result.stderr || ''}${result.stdout || ''}`);
  meanVolume = match ? Number(match[1]) : null;
} catch (error) {
  console.log('volumedetect failed', error.message);
}
console.log('mean_volume', meanVolume);

const checks = [
  ['WebM downloaded', Boolean(webm)],
  ['file size > 20 KB', info.size > 20000],
  ['video stream VP9 854x480', video?.codec_name === 'vp9' && video?.width === 854 && video?.height === 480],
  ['audio stream Opus', audio?.codec_name === 'opus'],
  ['duration ~3 s', Math.abs(duration - 3) < 0.9],
  ['audio not silent', meanVolume !== null && meanVolume > -50],
  ['no console errors', errors.length === 0],
];
let failed = 0;
for (const [name, ok] of checks) {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
  if (!ok) failed += 1;
}
if (errors.length) console.log('errors', errors);
console.log(failed ? `AUDIO EXPORT FAILED (${failed})` : 'AUDIO EXPORT TEST PASSED');
await browser.close();
process.exit(failed ? 1 : 0);
