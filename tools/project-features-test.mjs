import path from 'node:path';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const MEDIA = path.resolve('tools/fixtures');
const PROJECT_NAME = 'Prueba capas y archivo';
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

// Proyecto limpio de ejecuciones anteriores.
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

const labels = () => page.evaluate(
  // La fila "Efectos" es fija del timeline, no una pista del proyecto.
  () => [...document.querySelectorAll('.track-label:not(.fx-label) .t-name')].map((el) => el.textContent),
);
const clickInPage = (selector) => page.evaluate((sel) => {
  const el = document.querySelector(sel);
  if (el) el.click();
  return Boolean(el);
}, selector);

/* ------------------------------ 1. Más capas ------------------------------ */
check((await labels()).length === 3, 'un proyecto nuevo tiene 3 pistas');
check(await clickInPage('[data-add-track-type="overlay"]'), 'el botón de añadir pista existe');
await clickInPage('[data-add-track-type="video"]');
await wait(350);
const afterTracks = await labels();
check(
  afterTracks.length === 5 && afterTracks.includes('Superposición') && afterTracks.includes('Vídeo 2'),
  'se añaden pistas nuevas desde la línea de tiempo',
  JSON.stringify(afterTracks),
);

const chooserPromise = page.waitForFileChooser();
await page.click('.media-drop');
(await chooserPromise).accept([path.join(MEDIA, 'clip-video.webm')]);
await page.waitForFunction(() => document.querySelectorAll('.clip').length === 1, { timeout: 40000 });
await wait(400);

const moved = await page.evaluate(async () => {
  const store = await import('/src/store.js');
  const target = store.state.project.tracks.find((track) => track.name === 'Vídeo 2');
  const clip = store.state.project.tracks.flatMap((track) => track.clips).find((item) => item.kind === 'video');
  if (!target || !clip) return { ok: false, reason: 'falta pista o clip' };
  store.moveClip(clip.id, { start: 0, trackId: target.id });
  const found = store.findClip(clip.id);
  return { ok: found?.track?.id === target.id, track: found?.track?.name, clips: target.clips.length };
});
check(moved.ok, 'un clip se arrastra a la pista de vídeo 2 (misma tipo)', JSON.stringify(moved));

/* --------------------------- 2. Animaciones de texto --------------------------- */
const textClip = await page.evaluate(async () => {
  const store = await import('/src/store.js');
  const clip = store.addTextClip('Hola animación', { start: 1 });
  return clip ? { id: clip.id, start: clip.start } : null;
});
check(Boolean(textClip), 'se crea un texto en la línea de tiempo');

await clickInPage('[data-panel="text"]');
await wait(250);
await clickInPage('[data-text-tab="animation"]');
await wait(250);
const inApplied = await page.evaluate(async (clipId) => {
  document.querySelector('[data-text-animation="in-slide-up"]')?.click();
  await new Promise((resolve) => setTimeout(resolve, 400));
  const store = await import('/src/store.js');
  const found = store.findClip(clipId);
  return {
    animation: found?.clip?.text?.animation,
    playhead: store.state.playhead,
    start: found?.clip?.start,
  };
}, textClip.id);
check(inApplied.animation === 'in-slide-up', 'la animación se aplica al texto', JSON.stringify(inApplied));
check(
  Math.abs(inApplied.playhead - inApplied.start) < 0.08,
  'aplicar una animación lleva el cursor al inicio del clip',
  `cursor=${inApplied.playhead?.toFixed(2)} inicio=${inApplied.start}`,
);

const loopApplied = await page.evaluate(async (clipId) => {
  document.querySelector('[data-text-animation="loop-wave"]')?.click();
  await new Promise((resolve) => setTimeout(resolve, 350));
  const store = await import('/src/store.js');
  return { animation: store.findClip(clipId)?.clip?.text?.animation, playing: store.state.playing };
}, textClip.id);
check(loopApplied.animation === 'loop-wave' && loopApplied.playing === false, 'la animación en bucle queda aplicada en pausa');

const frameA = await page.evaluate(() => document.querySelector('.stage-canvas')?.toDataURL('image/png') || '');
await wait(500);
const frameB = await page.evaluate(() => document.querySelector('.stage-canvas')?.toDataURL('image/png') || '');
check(
  Boolean(frameA) && frameA !== frameB,
  'la animación en bucle se previsualiza aunque el reproductor esté en pausa',
  `bytes=${frameA.length}`,
);

/* --------------------- 3. Guardar / abrir proyecto como archivo --------------------- */
const fileResult = await page.evaluate(async () => {
  const store = await import('/src/store.js');
  const originalId = store.state.project.id;
  const countTracks = () => store.state.project.tracks.length;
  const countClips = () => store.state.project.tracks.reduce((sum, track) => sum + track.clips.length, 0);
  const originalTracks = countTracks();
  const originalClips = countClips();

  let blob = null;
  let fileName = '';
  const originalClick = HTMLAnchorElement.prototype.click;
  const originalCreate = URL.createObjectURL;
  HTMLAnchorElement.prototype.click = function capture() { fileName = this.download; };
  URL.createObjectURL = (value) => { blob = value; return 'blob:opencup-test'; };
  let info = null;
  let exportError = null;
  try {
    info = await store.exportProjectFile();
  } catch (error) {
    exportError = error.message;
  } finally {
    HTMLAnchorElement.prototype.click = originalClick;
    URL.createObjectURL = originalCreate;
  }
  const json = blob ? await blob.text() : '';
  const parsed = json ? JSON.parse(json) : null;

  store.addTrack('audio');
  const mutatedTracks = countTracks();

  let imported = null;
  let importError = null;
  try {
    imported = await store.importProjectFile(
      new File([json], 'prueba.opencup.json', { type: 'application/json' }),
    );
  } catch (error) {
    importError = error.message;
  }

  return {
    exportError,
    importError,
    fileName,
    info,
    kind: parsed?.kind,
    embedded: Boolean(parsed?.media?.[0]?.data?.startsWith('data:')),
    mutatedTracks,
    originalTracks,
    originalClips,
    tracksAfter: countTracks(),
    clipsAfter: countClips(),
    idAfter: store.state.project?.id,
    originalId,
    imported,
  };
});
console.log('export/import', JSON.stringify(fileResult));
check(!fileResult.exportError && fileResult.kind === 'openCUP-project', 'el proyecto se exporta como archivo openCUP', fileResult.exportError || '');
check(/^Prueba capas y archivo.*\.opencup\.json$/.test(fileResult.fileName || ''), 'el archivo descargado lleva el nombre del proyecto', fileResult.fileName);
check(fileResult.embedded, 'los medios viajan incrustados dentro del archivo');
check(fileResult.info?.embedded >= 1, 'se incrusta al menos el vídeo importado', JSON.stringify(fileResult.info));
check(fileResult.mutatedTracks === fileResult.originalTracks + 1, 'la mutación intermedia añade una pista');
check(!fileResult.importError, 'importar el archivo no falla', fileResult.importError || '');
check(
  fileResult.idAfter === fileResult.originalId
    && fileResult.tracksAfter === fileResult.originalTracks
    && fileResult.clipsAfter === fileResult.originalClips,
  'importar restaura el proyecto tal y como se guardó',
  JSON.stringify({ idAfter: fileResult.idAfter, tracksAfter: fileResult.tracksAfter, clipsAfter: fileResult.clipsAfter }),
);

/* ------------------------------ 4. Kokoro (TTS) ------------------------------ */
await clickInPage('[data-panel="text"]');
await wait(250);
await clickInPage('[data-text-tab="tts"]');
await wait(250);
const ttsUi = await page.evaluate(() => {
  const select = document.querySelector('[data-kokoro-voice]');
  return {
    generate: Boolean(document.querySelector('[data-tts-generate]')),
    progress: Boolean(document.querySelector('[data-tts-progress]')),
    progressHidden: Boolean(document.querySelector('[data-tts-progress]')?.classList.contains('hidden')),
    voices: select ? [...select.options].map((option) => option.value) : [],
    preview: Boolean(document.querySelector('[data-tts-play]')) && Boolean(document.querySelector('[data-tts-stop]')),
  };
});
check(
  ttsUi.generate && ttsUi.progress && ttsUi.progressHidden && ttsUi.preview && ttsUi.voices.includes('ef_dora'),
  'la pestaña Texto a voz ofrece Kokoro con progreso y vista previa del sistema',
  JSON.stringify(ttsUi),
);

const kokoroLib = await page.evaluate(async () => {
  try {
    const kokoro = await import('https://cdn.jsdelivr.net/npm/kokoro-js@1.2.1/+esm');
    const local = await import('/src/kokoro-tts.js');
    const ipa = local.spanishToIpa('Hola, ¿cómo estás?');
    return {
      tts: typeof kokoro.KokoroTTS?.from_pretrained === 'function',
      ipa,
      voices: local.KOKORO_ES_VOICES,
    };
  } catch (error) {
    return { error: error.message };
  }
});
check(kokoroLib.tts, 'la biblioteca Kokoro se carga desde el CDN', kokoroLib.error || '');
check(
  Boolean(kokoroLib.ipa && kokoroLib.ipa.length > 5 && /ˈ/.test(kokoroLib.ipa)),
  'el conversor español produce fonemas IPA con acento',
  kokoroLib.ipa,
);
check(Array.isArray(kokoroLib.voices) && kokoroLib.voices.includes('ef_dora'), 'las voces españolas están declaradas', JSON.stringify(kokoroLib.voices));

check(errors.length === 0, 'sin errores de consola', JSON.stringify(errors.slice(0, 4)));

await browser.close();
if (failures.length) {
  console.log(`FALLOS: ${failures.length}`);
  failures.forEach((item) => console.log(` - ${item}`));
  process.exit(1);
}
console.log('PROJECT FEATURES TEST PASSED');
