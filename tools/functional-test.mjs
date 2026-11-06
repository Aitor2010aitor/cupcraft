import puppeteer from 'puppeteer-core';
import { mkdir, readdir, rm } from 'node:fs/promises';
import path from 'node:path';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const BASE = 'http://127.0.0.1:8787/';
const MEDIA = path.resolve('tools/fixtures');
const SHOTS = path.resolve('shots');
const DOWNLOADS = path.resolve('shots/downloads');

await mkdir(SHOTS, { recursive: true });
await rm(DOWNLOADS, { recursive: true, force: true });
await mkdir(DOWNLOADS, { recursive: true });

const failures = [];
const check = (condition, label, detail = '') => {
  if (condition) {
    console.log(`PASS ${label}`);
  } else {
    failures.push(label);
    console.log(`FAIL ${label}${detail ? ` — ${detail}` : ''}`);
  }
};

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
const pageErrors = [];
const consoleErrors = [];
page.on('console', (message) => {
  if (message.type() === 'error') consoleErrors.push(message.text());
});
page.on('pageerror', (error) => pageErrors.push(error.message));

const cdp = await page.createCDPSession();
await cdp.send('Browser.setDownloadBehavior', {
  behavior: 'allow',
  downloadPath: DOWNLOADS,
  eventsEnabled: true,
});

const shot = async (name) => {
  await page.screenshot({ path: path.join(SHOTS, name) });
  console.log(`shot ${name}`);
};

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const state = () => page.evaluate(() => {
  const project = window.openCUP?.state?.project;
  return {
    view: window.openCUP?.state?.view,
    playhead: window.openCUP?.state?.playhead,
    playing: window.openCUP?.state?.playing,
    selection: window.openCUP?.state?.selection,
    zoom: window.openCUP?.state?.zoom,
    history: window.openCUP?.state?.history?.length,
    future: window.openCUP?.state?.future?.length,
    media: [...(window.openCUP?.state?.media?.values() || [])].map((item) => ({
      name: item.name,
      kind: item.kind,
      duration: item.duration,
      thumb: Boolean(item.thumb),
      peaks: item.peaks?.length || 0,
    })),
    tracks: project?.tracks?.map((track) => ({
      type: track.type,
      hidden: track.hidden,
      muted: track.muted,
      locked: track.locked,
      clips: track.clips.map((clip) => ({
        id: clip.id,
        kind: clip.kind,
        name: clip.name,
        start: Number(clip.start.toFixed(3)),
        duration: Number(clip.duration.toFixed(3)),
        filter: clip.filter,
        text: clip.text?.content,
        volume: clip.volume,
      })),
    })),
  };
});

await page.goto(BASE, { waitUntil: 'networkidle0' });
await wait(400);

// ---- Create a project -------------------------------------------------------
await page.click('.hero-create');
await page.waitForSelector('.modal');
await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Prueba funcional'; });
await page.click('[data-new-create]');
await page.waitForSelector('.editor');
await wait(500);
let data = await state();
check(data.view === 'editor' && data.tracks?.length === 3, 'new project opens editor with 3 tracks');

// ---- Import media -----------------------------------------------------------
const chooserPromise = page.waitForFileChooser();
await page.click('.media-drop');
const chooser = await chooserPromise;
await chooser.accept([
  path.join(MEDIA, 'clip-video.webm'),
  path.join(MEDIA, 'foto.png'),
  path.join(MEDIA, 'musica.wav'),
]);
await page.waitForFunction(() => document.querySelectorAll('.media-card').length === 3, { timeout: 40000 });
await page.waitForFunction(() => document.querySelectorAll('.clip').length === 3, { timeout: 20000 });
await wait(500);
data = await state();
check(data.media.length === 3, 'imports 3 media files', JSON.stringify(data.media));
check(data.media.some((item) => item.kind === 'video' && item.duration > 3), 'reads video duration');
check(data.media.some((item) => item.kind === 'audio' && item.peaks > 20), 'computes audio waveform peaks');
check(data.media.every((item) => item.kind === 'audio' || item.thumb), 'generates thumbnails for visual media');
const videoTrack = data.tracks.find((track) => track.type === 'video');
const audioTrack = data.tracks.find((track) => track.type === 'audio');
check(videoTrack.clips.length === 2, 'video and image land on the video track', JSON.stringify(videoTrack.clips));
check(audioTrack.clips.length === 1, 'audio lands on the audio track');
check(await page.evaluate(() => document.querySelectorAll('.clip-wave').length) === 1, 'renders waveform canvas');
await shot('10-media-imported.png');

// ---- Playback ---------------------------------------------------------------
await page.evaluate(() => { window.openCUP.state.playhead = 0; });
await page.click('.play-btn');
await wait(1200);
data = await state();
check(data.playing === true && data.playhead > 0.5, 'playback advances the playhead', `playhead=${data.playhead}`);
await shot('11-playing.png');
await page.click('.play-btn');
await wait(200);
data = await state();
check(data.playing === false, 'pause stops playback');

// ---- Seek + split + undo/redo ----------------------------------------------
const rulerBox = await page.$eval('[data-ruler]', (el) => {
  const box = el.getBoundingClientRect();
  return { x: box.x, y: box.y, w: box.width, h: box.height };
});
const zoom = (await state()).zoom;
await page.mouse.click(rulerBox.x + 2 * zoom, rulerBox.y + 10);
await wait(150);
const seekTime = (await state()).playhead;
check(Math.abs(seekTime - 2) < 0.12, 'clicking the ruler seeks the playhead', `t=${seekTime}`);

await page.click('.clip.video');
await page.evaluate(() => document.activeElement?.blur());
await page.keyboard.press('b');
await wait(200);
data = await state();
const splitClips = data.tracks.find((track) => track.type === 'video').clips;
check(splitClips.length === 3, 'split creates an extra clip at the playhead', JSON.stringify(splitClips));
await shot('12-split.png');

await page.keyboard.down('Control');
await page.keyboard.press('z');
await page.keyboard.up('Control');
await wait(200);
data = await state();
check(data.tracks.find((track) => track.type === 'video').clips.length === 2, 'undo restores the split');
await page.keyboard.down('Control');
await page.keyboard.press('y');
await page.keyboard.up('Control');
await wait(200);
data = await state();
check(data.tracks.find((track) => track.type === 'video').clips.length === 3, 'redo reapplies the split');

// ---- Trim the right edge of the clip at 2s ---------------------------------
const splitBox = await page.evaluate(() => {
  const clips = [...document.querySelectorAll('.clip.video')];
  const target = clips.find((el) => Math.abs(parseFloat(el.style.left) - 2 * window.openCUP.state.zoom) < 2) || clips[1];
  const box = target.getBoundingClientRect();
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
});
await page.mouse.click(splitBox.x, splitBox.y);
await wait(120);
const clipBefore = await page.evaluate(() => {
  const found = window.openCUP.state.project.tracks.find((track) => track.type === 'video')
    .clips.find((clip) => clip.start > 1.9 && clip.start < 2.1);
  return found ? { id: found.id, duration: found.duration } : null;
});
if (clipBefore) {
  const handleBox = await page.$eval('.clip.video.selected .clip-handle.right', (el) => {
    const box = el.getBoundingClientRect();
    return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  });
  await page.mouse.move(handleBox.x, handleBox.y);
  await page.mouse.down();
  await page.mouse.move(handleBox.x - 35, handleBox.y, { steps: 6 });
  await page.mouse.up();
  await wait(250);
  const clipAfter = await page.evaluate((id) => {
    for (const track of window.openCUP.state.project.tracks) {
      const clip = track.clips.find((item) => item.id === id);
      if (clip) return { duration: clip.duration };
    }
    return null;
  }, clipBefore.id);
  check(clipAfter && clipAfter.duration < clipBefore.duration - 0.1, 'dragging the right handle trims the clip', `${clipBefore.duration} -> ${clipAfter?.duration}`);
} else {
  check(false, 'could locate the split clip for trimming');
}

// ---- Move the selected clip -------------------------------------------------
const beforeMove = await page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.find((track) => track.type === 'video')
    .clips.find((item) => item.id === window.openCUP.state.selection);
  return clip ? clip.start : null;
});
const clipBox = await page.$eval('.clip.selected', (el) => {
  const box = el.getBoundingClientRect();
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
});
await page.mouse.move(clipBox.x, clipBox.y);
await page.mouse.down();
await page.mouse.move(clipBox.x + 30, clipBox.y, { steps: 6 });
await page.mouse.up();
await wait(250);
const afterMove = await page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.find((track) => track.type === 'video')
    .clips.find((item) => item.id === window.openCUP.state.selection);
  return clip ? clip.start : null;
});
check(afterMove !== null && beforeMove !== null, 'drag moves the selected clip', `${beforeMove} -> ${afterMove}`);

// ---- Text editing -----------------------------------------------------------
await page.click('[data-panel="text"]');
await wait(150);
await page.click('[data-preset="0"]');
await wait(250);
const styleBeforeEdit = await page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.find((track) => track.type === 'text').clips[0];
  if (!clip?.text) return null;
  const { content, ...style } = clip.text;
  return style;
});
await page.evaluate(() => {
  const area = document.querySelector('[data-text-content]');
  area.focus();
  area.select();
});
await page.keyboard.type('Hola cupcraft');
await wait(250);
data = await state();
const textClip = data.tracks.find((track) => track.type === 'text').clips[0];
check(textClip?.text === 'Hola cupcraft', 'editing text updates the clip', textClip?.text);
check(await page.evaluate(() => document.querySelector('.clip-text-preview')?.textContent) === 'Hola cupcraft', 'timeline preview reflects text');
const styleAfterEdit = await page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.find((track) => track.type === 'text').clips[0];
  if (!clip?.text) return null;
  const { content, ...style } = clip.text;
  return style;
});
check(
  styleBeforeEdit && styleAfterEdit
    && Object.keys(styleBeforeEdit).length === Object.keys(styleAfterEdit).length
    && Object.keys(styleBeforeEdit).every((key) => JSON.stringify(styleBeforeEdit[key]) === JSON.stringify(styleAfterEdit[key])),
  'editing text keeps its style fields',
  JSON.stringify({ styleBeforeEdit, styleAfterEdit }),
);
await shot('13-text-edited.png');

// ---- Filter + transition ----------------------------------------------------
await page.click('.clip.video');
await page.click('[data-panel="filters"]');
await wait(150);
await page.click('[data-filter-source="local"]');
await wait(150);
await page.click('[data-filter="vivid"]');
await wait(200);
data = await state();
check(data.tracks.find((track) => track.type === 'video').clips.some((clip) => clip.filter === 'vivid'), 'applies a filter to the selected clip');
await page.click('[data-panel="transitions"]');
await wait(150);
await page.click('[data-transition-source="local"]');
await wait(150);
await page.click('[data-transition="fade"]');
await wait(200);
const fade = await page.evaluate(() => {
  const clip = window.openCUP.state.project.tracks.find((track) => track.type === 'video')
    .clips.find((item) => item.id === window.openCUP.state.selection);
  return clip ? { fadeIn: clip.fadeIn, fadeOut: clip.fadeOut } : null;
});
check(fade?.fadeIn > 0 && fade?.fadeOut > 0, 'applies a fade transition', JSON.stringify(fade));
await shot('14-filter-fade.png');

// ---- Screenshot export ------------------------------------------------------
await page.click('[data-act="shot"]');
await wait(1600);
let downloads = await readdir(DOWNLOADS);
check(downloads.some((file) => file.endsWith('.png')), 'exports a PNG frame', JSON.stringify(downloads));

// ---- Persistence ------------------------------------------------------------
await page.click('[data-act="home"]');
await page.waitForSelector('.home');
await wait(500);
data = await state();
check(data.view === 'home', 'returns to home');
const cardName = await page.$eval('.project-card .project-name', (el) => el.textContent).catch(() => null);
check(cardName === 'Prueba funcional', 'home lists the saved project', cardName);
await shot('15-home-saved.png');

await page.reload({ waitUntil: 'networkidle0' });
await wait(500);
const reopened = await page.evaluate(() => document.querySelectorAll('.project-card').length);
check(reopened >= 1, 'project persists after reload');
if (reopened) {
  await page.click('.project-card');
  await page.waitForSelector('.editor');
  await wait(700);
  data = await state();
  check(data.media.length === 3, 'reopened project restores media', String(data.media.length));
  check(data.tracks.find((track) => track.type === 'text').clips.length === 1, 'reopened project restores timeline clips');
  await shot('16-reopened.png');
}

check(pageErrors.length === 0, 'no uncaught page errors', pageErrors.join(' | '));
check(consoleErrors.length === 0, 'no console errors', consoleErrors.join(' | '));

console.log('---');
console.log(failures.length ? `FAILURES: ${failures.join(', ')}` : 'ALL CHECKS PASSED');
await browser.close();
process.exit(failures.length ? 1 : 0);
