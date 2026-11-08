import puppeteer from 'puppeteer-core';
import { mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const BASE = 'http://127.0.0.1:8787';
const SHOTS = path.resolve('shots');
await mkdir(SHOTS, { recursive: true });

const failures = [];
const notes = [];
const check = (ok, message) => {
  if (ok) notes.push(`PASS ${message}`);
  else { failures.push(message); notes.push(`FAIL ${message}`); }
};

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: 'new',
  defaultViewport: { width: 1440, height: 900 },
  args: ['--no-sandbox', '--disable-gpu', '--autoplay-policy=no-user-gesture-required'],
});

// Pestaña anterior que permanece en Inicio: el puente debe ignorarla y
// controlar la pestaña más reciente, que es la que usa la prueba.
const distractor = await browser.newPage();
await distractor.goto(BASE, { waitUntil: 'networkidle0' });

const page = await browser.newPage();
const errors = [];
page.on('console', (msg) => {
  if (msg.type() === 'error') errors.push(`console: ${msg.text()}`);
});
page.on('pageerror', (error) => errors.push(`pageerror: ${error.message}`));

try {
  await page.goto(BASE, { waitUntil: 'networkidle0' });
  await page.waitForSelector('.hero-create');

  check(await page.$('.capcut-pill') !== null, 'home muestra acceso CapCut');
  check(await page.$('.side-ai') !== null, 'home muestra Puente IA');
  check((await page.$$('.showcase-card')).length === 0, 'se eliminó el centro de creación con IA de la portada');

  // Crear proyecto desde el hero.
  await page.click('.hero-create');
  await page.waitForSelector('[data-new-name]');
  await page.evaluate(() => { document.querySelector('[data-new-name]').value = 'Prueba nuevas herramientas'; });
  await page.click('[data-new-create]');
  await page.waitForSelector('.editor');
  await page.waitForFunction(() => window.openCUP?.state?.project?.id);

  const tabs = await page.$$eval('.tools-strip .tab', (els) => els.map((el) => el.dataset.panel || el.textContent.trim()));
  check(tabs.length === 12, `editor muestra 12 pestañas (${tabs.length})`);

  const clickTab = async (panel) => {
    await page.evaluate((name) => {
      const tabs = [...document.querySelectorAll('.tools-strip .tab')];
      tabs.find((tab) => (tab.dataset.panel || '') === name)?.click();
    }, panel);
    await page.waitForFunction((name) => window.openCUP.state.panel === name, {}, panel);
    await new Promise((resolve) => setTimeout(resolve, 120));
  };

  await clickTab('stickers');
  await page.click('[data-sticker-source="local"]');
  await new Promise((resolve) => setTimeout(resolve, 150));
  const stickerButtons = await page.$$('.sticker-grid .sticker-btn');
  const stickerChips = await page.$$('.category-chips .category-chip');
  check(stickerButtons.length >= 12, `panel stickers: ${stickerButtons.length} stickers en la categoría`);
  check(stickerChips.length === 19, `panel stickers: ${stickerChips.length} categorías`);
  await page.screenshot({ path: path.join(SHOTS, 'new-stickers.png') });
  await page.click('.sticker-btn');
  await new Promise((resolve) => setTimeout(resolve, 200));
  const stickerState = await page.evaluate(() => {
    const found = window.openCUP.state.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === window.openCUP.state.selection);
    return found ? { kind: found.kind, sticker: found.sticker } : null;
  });
  check(stickerState?.kind === 'sticker', 'sticker insertado como capa sticker');
  check(Boolean(stickerState?.sticker?.content), 'sticker conserva contenido');
  check(await page.$('[data-sticker-content]') !== null, 'detalles muestran controles de sticker');

  // Importar una imagen real para probar efectos y transiciones sobre medios.
  await clickTab('media');
  const chooserPromise = page.waitForFileChooser();
  await page.click('.media-drop');
  const chooser = await chooserPromise;
  await chooser.accept([path.resolve('tools/fixtures/foto.png')]);
  await page.waitForFunction(() => [...window.openCUP.state.media.values()].some((media) => media.kind === 'image'), { timeout: 30000 });
  await page.waitForFunction(() => window.openCUP.state.project.tracks.some((track) => track.clips.some((clip) => clip.kind === 'image')), { timeout: 20000 });
  const imageClipId = await page.evaluate(() => {
    const track = window.openCUP.state.project.tracks.find((item) => item.type === 'video');
    return track?.clips.find((clip) => clip.kind === 'image')?.id || null;
  });
  check(Boolean(imageClipId), 'imagen importada en la pista de video');
  if (imageClipId) await page.evaluate(async (id) => { await window.openCUP.command('selectClip', { id }); }, imageClipId);

  await clickTab('effects');
  await page.click('[data-effect-source="local"]');
  await new Promise((resolve) => setTimeout(resolve, 150));
  const effectCards = await page.$$('.effect-grid .effect-card');
  const effectChips = await page.$$('.category-chips .category-chip');
  check(effectCards.length >= 160, `panel efectos: ${effectCards.length} efectos`);
  check(effectChips.length === 20, `panel efectos: ${effectChips.length} categorías`);
  await page.click('[data-effect="oldFilm"]');
  await new Promise((resolve) => setTimeout(resolve, 250));
  const effectState = await page.evaluate(() => {
    const found = window.openCUP.state.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === window.openCUP.state.selection);
    return found ? { kind: found.kind, effect: found.effect } : null;
  });
  check(effectState?.kind === 'image' && effectState?.effect === 'oldFilm', 'efecto oldFilm aplicado a la imagen');
  await page.screenshot({ path: path.join(SHOTS, 'new-effects.png') });

  await clickTab('transitions');
  await page.click('[data-transition-source="local"]');
  await new Promise((resolve) => setTimeout(resolve, 150));
  const transitionCards = await page.$$('.transition-grid .transition-card');
  check(transitionCards.length === 16, `panel transiciones: ${transitionCards.length} transiciones`);
  await page.screenshot({ path: path.join(SHOTS, 'new-transitions.png') });
  await page.click('[data-transition="slide-left"]');
  await new Promise((resolve) => setTimeout(resolve, 200));
  check(await page.evaluate(() => {
    const found = window.openCUP.state.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === window.openCUP.state.selection);
    return found?.transition === 'slide-left' && found?.transitionDuration === 0.6;
  }), 'transición slide-left aplicada');

  await clickTab('captions');
  const captionStyles = await page.$$('.caption-styles .caption-style');
  check(captionStyles.length === 6, `panel subtítulos: ${captionStyles.length} estilos`);
  await page.click('[data-caption-style="yellow"]');
  await new Promise((resolve) => setTimeout(resolve, 150));
  await page.screenshot({ path: path.join(SHOTS, 'new-captions.png') });
  await page.click('[data-add-subtitle]');
  await new Promise((resolve) => setTimeout(resolve, 350));
  check(await page.evaluate(() => {
    const found = window.openCUP.state.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === window.openCUP.state.selection);
    return found?.kind === 'text' && found?.text?.color === '#ffe34d';
  }), 'subtítulo creado con estilo amarillo');
  await page.screenshot({ path: path.join(SHOTS, 'new-captions-added.png') });

  await clickTab('avatar');
  const avatarCards = await page.$$('.avatar-grid .avatar-card');
  check(avatarCards.length === 8, `panel avatar: ${avatarCards.length} avatares`);
  await page.screenshot({ path: path.join(SHOTS, 'new-avatar.png') });
  await page.click('[data-avatar="presenter"]');
  await new Promise((resolve) => setTimeout(resolve, 200));
  check(await page.evaluate(() => {
    const found = window.openCUP.state.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === window.openCUP.state.selection);
    return found?.kind === 'sticker' && found?.sticker?.background === '#1d6d78' && found?.sticker?.shape === 'circle';
  }), 'avatar insertado con fondo circular');

  await clickTab('filters');
  await new Promise((resolve) => setTimeout(resolve, 150));
  const filterCards = await page.$$('.filter-grid .filter-card');
  const filterChips = await page.$$('.category-chips .category-chip');
  check(filterCards.length === 83, `panel filtros: ${filterCards.length} filtros de la línea cupcraft`);
  check(filterChips.length === 13, `panel filtros: ${filterChips.length} categorías`);
  await page.screenshot({ path: path.join(SHOTS, 'new-filters.png') });
  await page.evaluate(() => {
    const chip = [...document.querySelectorAll('[data-filter-category]')].find((item) => item.dataset.filterCategory === 'Firma');
    chip?.click();
  });
  await new Promise((resolve) => setTimeout(resolve, 250));
  const creativeFilters = await page.$$('.filter-grid .filter-card');
  check(creativeFilters.length === 8, `categoría Firma filtra ${creativeFilters.length} filtros`);
  const activeFilterCategory = await page.$eval('.category-chips .category-chip.active', (el) => el.dataset.filterCategory);
  check(activeFilterCategory === 'Firma', `categoría activa visible: ${activeFilterCategory}`);
  const chipScroll = await page.$eval('.category-chips', (el) => Math.max(el.scrollTop, el.scrollLeft));
  check(chipScroll > 0, `el raíl de categorías se desplaza hasta la activa (${Math.round(chipScroll)} px)`);
  await page.screenshot({ path: path.join(SHOTS, 'new-filters-creative.png') });

  await clickTab('templates');
  const templateCards = await page.$$('.template-grid .template-card');
  check(templateCards.length === 10, `panel plantillas: ${templateCards.length} plantillas`);
  await page.screenshot({ path: path.join(SHOTS, 'new-templates.png') });

  // Diálogo del puente IA desde el propio API.
  await page.evaluate(() => { void window.openCUP.openBridge(); });
  await page.waitForSelector('.bridge-modal .bridge-hero, .bridge-modal .bridge-offline', { timeout: 8000 });
  const bridgeText = await page.$eval('.bridge-modal .modal-body', (el) => el.textContent);
  check(bridgeText.includes('cupcraft AI Bridge'), 'diálogo IA muestra el puente');
  check(bridgeText.includes('8790'), 'diálogo IA muestra el puerto 8790');
  check(await page.$('.bridge-actions .bridge-action') !== null, 'diálogo IA lista acciones');
  await page.screenshot({ path: path.join(SHOTS, 'new-bridge.png') });
  await page.evaluate(() => document.querySelector('[data-modal-close]')?.click());
  await new Promise((resolve) => setTimeout(resolve, 150));

  // CapCut desde inicio.
  await page.evaluate(() => window.openCUP.command('goHome'));
  await page.waitForSelector('.home');
  await page.click('[data-home-action="capcut"]');
  await page.waitForSelector('.capcut-modal .capcut-list, .capcut-modal .capcut-empty', { timeout: 10000 });
  const capcutCards = await page.$$('.capcut-modal .capcut-card');
  check(capcutCards.length >= 1, `modal CapCut lista ${capcutCards.length} borradores`);
  const capcutNames = await page.$$eval('.capcut-modal .capcut-info strong', (els) => els.map((el) => el.textContent.trim()));
  check(capcutNames.includes('1007'), 'modal CapCut incluye el borrador 1007');
  await page.screenshot({ path: path.join(SHOTS, 'new-capcut-list.png') });

  const clicked1007 = await page.evaluate(() => {
    const card = [...document.querySelectorAll('.capcut-modal .capcut-card')]
      .find((item) => item.querySelector('.capcut-info strong')?.textContent.trim() === '1007');
    if (!card) return false;
    card.querySelector('[data-capcut-open]')?.click();
    return true;
  });
  check(clicked1007, 'se puede abrir el borrador CapCut 1007');
  if (clicked1007) {
    await page.waitForFunction(() => window.openCUP.state.view === 'editor' && window.openCUP.state.project?.source?.type === 'capcut', { timeout: 20000 });
    const capcutState = await page.evaluate(() => {
      const project = window.openCUP.state.project;
      return {
        name: project.name,
        source: project.source,
        clips: project.tracks.flatMap((track) => track.clips).map((clip) => ({ kind: clip.kind, text: clip.text?.content })),
      };
    });
    check(capcutState.clips.some((clip) => clip.kind === 'text' && clip.text === 'Texto predeterminado'), 'import CapCut conserva el texto del borrador 1007');
    check(capcutState.source?.type === 'capcut', 'proyecto importado marcado como CapCut');
    await page.screenshot({ path: path.join(SHOTS, 'new-capcut-imported.png') });
  }

  // Puente IA real: estado y comando con token.
  const token = (await readFile('.openCUP-ai-token', 'utf8')).trim();
  const api = (route) => `http://127.0.0.1:8790/api/ai${route}`;
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
  const statusResponse = await fetch(api('/status'), { headers });
  const status = await statusResponse.json();
  check(statusResponse.ok && status.connected === true, 'puente IA reporta la app conectada');
  check(status.clients >= 2, `puente IA detecta varias pestañas (${status.clients})`);
  check(Boolean(status.target), 'puente IA identifica la pestaña de control');
  const unauthorized = await fetch(api('/status'));
  check(unauthorized.status === 401, 'el puente rechaza peticiones sin token');
  const actionsResponse = await fetch(api('/actions'), { headers });
  const actions = await actionsResponse.json();
  check(actions.actions?.length === 31, `el puente publica 31 acciones (${actions.actions?.length})`);
  check(actions.actions?.some((item) => item.action === 'applyTransition' && item.description.includes('16 tipos')), 'la documentación de acciones está actualizada');
  await new Promise((resolve) => setTimeout(resolve, 1000));
  const stateResponse = await fetch(api('/state'), { headers });
  const state = await stateResponse.json();
  check(state.state?.project?.source?.type === 'capcut', 'puente IA devuelve el estado del proyecto CapCut');

  const commandResponse = await fetch(api('/command'), {
    method: 'POST',
    headers,
    body: JSON.stringify({ action: 'addText', params: { content: 'Texto desde otra IA', start: 0, duration: 2 }, wait: true }),
  });
  const command = await commandResponse.json();
  check(commandResponse.ok && command.ok, `comando addText ejecutado (${command.error || 'ok'})`);
  const afterCommand = await page.evaluate(() => window.openCUP.state.project.tracks.flatMap((track) => track.clips).some((clip) => clip.text?.content === 'Texto desde otra IA'));
  check(afterCommand, 'el texto del comando aparece en el proyecto');

  const screenshotResponse = await fetch(api('/command'), {
    method: 'POST',
    headers,
    body: JSON.stringify({ action: 'screenshot', params: { width: 640 }, wait: true }),
  });
  const screenshot = await screenshotResponse.json();
  check(screenshotResponse.ok && String(screenshot.result?.dataUrl || '').startsWith('data:image/jpeg'), 'comando screenshot devuelve JPEG');

  // ---- Transformación estilo CapCut: escala, fotogramas clave y alineación ----
  await page.evaluate(() => window.openCUP.command('newProject', { name: 'Transformación CapCut' }));
  await page.waitForFunction(() => window.openCUP.state.view === 'editor');
  await new Promise((resolve) => setTimeout(resolve, 250));
  const transformClipId = await page.evaluate(async () => {
    const added = await window.openCUP.command('addText', { content: 'cupcraft', start: 0, duration: 5 });
    window.openCUP.command('selectClip', { id: added.clip.id });
    window.openCUP.command('seek', { time: 0.5 });
    return added.clip.id;
  });
  check(typeof transformClipId === 'string', 'clip de texto creado para probar transformación');

  const summaryInfo = await page.evaluate(() => {
    const group = [...document.querySelectorAll('[data-details] .tx-group')]
      .find((item) => item.querySelector('summary')?.textContent.trim() === 'Transformación');
    if (!group) return null;
    const reset = group.querySelector('.sec-reset');
    return {
      text: group.querySelector('summary').textContent.trim(),
      resetIconOnly: Boolean(reset) && reset.textContent.trim() === '',
      mixSection: Boolean(document.querySelector('[data-details] details[data-section="mix"]')),
    };
  });
  check(summaryInfo?.text === 'Transformación' && summaryInfo.resetIconOnly && summaryInfo.mixSection, 'resumen Transformación íntegro y restablecer solo icono', JSON.stringify(summaryInfo));

  let scaleRows = await page.evaluate(() => ({
    uniform: document.querySelectorAll('.dt-slider [data-prop="scale"]').length,
    x: document.querySelectorAll('[data-spin="scaleX"]').length,
    y: document.querySelectorAll('[data-spin="scaleY"]').length,
  }));
  check(scaleRows.uniform === 1 && scaleRows.x === 0 && scaleRows.y === 0, 'con escala uniforme activa hay una sola fila Escala', JSON.stringify(scaleRows));
  await page.click('[data-uniform]');
  await new Promise((resolve) => setTimeout(resolve, 150));
  scaleRows = await page.evaluate(() => ({
    uniform: document.querySelectorAll('.dt-slider [data-prop="scale"]').length,
    x: document.querySelectorAll('[data-spin="scaleX"]').length,
    y: document.querySelectorAll('[data-spin="scaleY"]').length,
  }));
  check(scaleRows.uniform === 0 && scaleRows.x === 1 && scaleRows.y === 1, 'al desactivar la escala uniforme aparecen Escala X e Y', JSON.stringify(scaleRows));
  await page.click('[data-uniform]');
  await new Promise((resolve) => setTimeout(resolve, 150));

  // Fotogramas clave de escala: 100% en t=0.5, 200% en t=2.5, interpolación en t=1.5.
  const keyframeFlow = await page.evaluate(async () => {
    const nap = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const spin = () => document.querySelector('[data-spin="scale"]');
    const setSpin = (value) => {
      const input = spin();
      input.value = String(value);
      input.dispatchEvent(new Event('change', { bubbles: true }));
    };
    window.openCUP.command('seek', { time: 0.5 });
    await nap(80);
    document.querySelector('[data-kf-toggle="scale"]').click(); // clave al valor efectivo (100%)
    await nap(80);
    window.openCUP.command('seek', { time: 2.5 });
    await nap(80);
    setSpin(200); // la propiedad ya tiene keyframes: escribe una clave con 200%
    await nap(80);
    window.openCUP.command('seek', { time: 1.5 });
    await nap(120);
    const clip = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((c) => c.id === window.openCUP.state.selection);
    const mid = Number(spin().value);
    document.querySelector('[data-kf-nav="1"]').click();
    await nap(120);
    return { count: clip.keyframes?.scale?.length || 0, mid, afterNext: window.openCUP.state.playhead };
  });
  check(keyframeFlow.count === 2, `escala con 2 fotogramas clave (${keyframeFlow.count})`);
  check(Math.abs(keyframeFlow.mid - 150) < 1, 'la escala se interpola entre fotogramas (150% a mitad de camino)', String(keyframeFlow.mid));
  check(Math.abs(keyframeFlow.afterNext - 2.5) < 0.01, 'el botón › salta al fotograma clave siguiente (t=2.5)', String(keyframeFlow.afterNext));

  // Los rombos de fotograma clave se pintan sobre el clip en la línea de tiempo.
  const diamonds = await page.evaluate(() => document.querySelectorAll('.clip .kf-diamond').length);
  check(diamonds === 2, `la línea de tiempo muestra 2 rombos de fotograma clave (${diamonds})`);

  // El cajón numérico escribe propiedades directamente (y sin keyframes).
  const spinWrite = await page.evaluate(async () => {
    const nap = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const input = document.querySelector('[data-spin="y"]');
    input.value = '-12';
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await nap(80);
    const clip = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((c) => c.id === window.openCUP.state.selection);
    return { y: clip.y };
  });
  check(Math.abs(spinWrite.y + 12) < 0.0001, 'el cajón numérico Y escribe la posición', JSON.stringify(spinWrite));

  // Alineación en el lienzo (proyecto 16:9 por defecto).
  const alignFlow = await page.evaluate(async () => {
    const nap = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const clipOf = () => window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((c) => c.id === window.openCUP.state.selection);
    document.querySelector('[data-align="left"]').click();
    await nap(80);
    const leftX = clipOf().x;
    document.querySelector('[data-align="center"]').click();
    await nap(80);
    const centerX = clipOf().x;
    document.querySelector('[data-align="right"]').click();
    await nap(80);
    const rightX = clipOf().x;
    document.querySelector('[data-align="bottom"]').click();
    await nap(80);
    const bottomY = clipOf().y;
    return { leftX, centerX, rightX, bottomY };
  });
  check(alignFlow.leftX < 0 && alignFlow.centerX === 0 && alignFlow.rightX > 0, 'alineación horizontal: izquierda < 0, centro = 0, derecha > 0', JSON.stringify(alignFlow));
  check(alignFlow.bottomY > 0, 'alineación inferior coloca y positiva', String(alignFlow.bottomY));

  // Restablecer transformación vuelve a los valores iniciales y borra claves.
  await page.click('[data-transform-reset]');
  await new Promise((resolve) => setTimeout(resolve, 150));
  const resetState = await page.evaluate(() => {
    const clip = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((c) => c.id === window.openCUP.state.selection);
    return { x: clip.x, y: clip.y, scale: clip.scale, rotate: clip.rotate, kf: Object.keys(clip.keyframes || {}).length };
  });
  check(resetState.x === 0 && resetState.y === 0 && resetState.scale === 1 && resetState.rotate === 0 && resetState.kf === 0, 'restablecer transformación limpia valores y fotogramas clave', JSON.stringify(resetState));

  // La Mezcla del texto usa el deslizador de opacidad con cajón y control de fotogramas.
  const mixRow = await page.evaluate(() => ({
    slider: document.querySelectorAll('[data-details] [data-prop="text.opacity"]').length,
    spin: document.querySelectorAll('[data-details] [data-spin="text.opacity"]').length,
    kf: document.querySelectorAll('[data-details] [data-kf-toggle="text.opacity"]').length,
  }));
  check(mixRow.slider === 1 && mixRow.spin === 1 && mixRow.kf === 1, 'la sección Mezcla del texto tiene deslizador, cajón y fotograma clave', JSON.stringify(mixRow));

  // El estado abierto/cerrado de las secciones sobrevive a un re-render.
  const sectionPersistence = await page.evaluate(async () => {
    const nap = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const group = document.querySelector('[data-details] details[data-section="transform"]');
    group.open = false;
    await nap(120); // el evento toggle se lanza de forma asíncrona
    window.openCUP.command('selectClip', { id: null });
    await nap(120);
    const textId = window.openCUP.state.project.tracks.flatMap((t) => t.clips).find((c) => c.kind === 'text').id;
    window.openCUP.command('selectClip', { id: textId });
    await nap(150);
    const after = document.querySelector('[data-details] details[data-section="transform"]');
    return after ? after.open : null;
  });
  check(sectionPersistence === false, 'la sección Transformación permanece cerrada tras re-renderizar', String(sectionPersistence));

  await page.screenshot({ path: path.join(SHOTS, 'new-transformacion.png') });

  check(errors.length === 0, `sin errores de consola (${errors.length})`);
  if (errors.length) notes.push(...errors.map((error) => `  ${error}`));
} catch (error) {
  failures.push(`Excepción: ${error?.message || error}`);
  notes.push(`FAIL Excepción: ${error?.stack || error}`);
  try { await page.screenshot({ path: path.join(SHOTS, 'new-failure.png') }); } catch { /* sin captura */ }
} finally {
  await browser.close();
}

console.log(notes.join('\n'));
if (failures.length) {
  console.error(`\n${failures.length} fallo(s):`);
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exit(1);
}
console.log('\nTodas las comprobaciones nuevas pasaron.');
