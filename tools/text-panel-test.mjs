import puppeteer from 'puppeteer-core';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const BASE = 'http://127.0.0.1:8787/';
const SHOTS = path.resolve('shots');
await mkdir(SHOTS, { recursive: true });

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
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
const errors = [];
page.on('console', (message) => {
  if (message.type() === 'error') errors.push(message.text());
});
page.on('pageerror', (error) => errors.push(error.message));

const shot = async (name) => {
  await page.evaluate(() => document.querySelectorAll('.toast').forEach((el) => el.remove()));
  await page.screenshot({ path: path.join(SHOTS, name), fullPage: false });
  console.log(`shot ${name}`);
};

const selectedText = () => page.evaluate(() => {
  const state = window.openCUP.state;
  const clip = state.project.tracks
    .flatMap((track) => track.clips)
    .find((item) => item.id === state.selection);
  return clip?.text || null;
});

const selectedClip = () => page.evaluate(() => {
  const state = window.openCUP.state;
  return state.project.tracks
    .flatMap((track) => track.clips)
    .find((item) => item.id === state.selection) || null;
});

const click = (selector) => page.evaluate((value) => {
  const element = document.querySelector(value);
  if (!element) throw new Error(`No está ${value}`);
  element.click();
}, selector);

const setRange = (key, value) => page.evaluate(({ key: field, value: next }) => {
  // Los controles de transformación/mezcla nuevos son sliders [data-prop]
  // o cajones numéricos [data-spin]; el resto siguen en [data-text-range].
  const input = document.querySelector(`[data-text-range="${field}"]`)
    || document.querySelector(`[data-prop="${field}"]`)
    || document.querySelector(`[data-prop="text.${field}"]`)
    || document.querySelector(`[data-spin="${field}"]`);
  if (!input) throw new Error(`No está el control ${field}`);
  input.value = String(next);
  input.dispatchEvent(new Event(input.matches('[data-spin]') ? 'change' : 'input', { bubbles: true }));
}, { key, value });

const setColor = (key, value) => page.evaluate(({ key: field, value: next }) => {
  const input = document.querySelector(`[data-text-paint="${field}"]`);
  if (!input) throw new Error(`No está el color ${field}`);
  input.value = next;
  input.dispatchEvent(new Event('input', { bubbles: true }));
}, { key, value });

const select = (selector, value) => page.evaluate(({ selector: query, value: next }) => {
  const input = document.querySelector(query);
  if (!input) throw new Error(`No está ${query}`);
  input.value = next;
  input.dispatchEvent(new Event('change', { bubbles: true }));
}, { selector, value });

const number = (selector, value) => page.evaluate(({ selector: query, value: next }) => {
  const input = document.querySelector(query);
  if (!input) throw new Error(`No está ${query}`);
  input.value = String(next);
  input.dispatchEvent(new Event('change', { bubbles: true }));
}, { selector, value });

try {
  await page.goto(BASE, { waitUntil: 'networkidle0' });
  await page.evaluate(() => window.openCUP.command('newProject', { name: 'Panel de texto' }));
  await page.waitForSelector('.editor');
  await page.waitForFunction(() => window.openCUP?.state?.project?.id);
  await page.click('[data-panel="text"]');
  await sleep(250);

  const tabs = await page.$$eval('[data-text-tab]', (items) => items.map((item) => item.textContent.trim()));
  check(
    JSON.stringify(tabs.slice(0, 5)) === JSON.stringify(['Texto', 'Animación', 'Seguimiento', 'Texto a voz', 'Avatares de IA'])
      && tabs.length === 6
      && /^\d+ de CapCut$/.test(tabs[5]),
    'el panel Texto muestra las cinco pestañas de CapCut más el catálogo',
    JSON.stringify(tabs),
  );
  const subtabs = await page.$$eval('[data-text-subtab]', (items) => items.map((item) => item.textContent.trim()));
  check(JSON.stringify(subtabs) === JSON.stringify(['Básico', 'Burbuja', 'Efectos']), 'Básico, Burbuja y Efectos están presentes');
  const styleCount = await page.$$eval('[data-preset]', (items) => items.length);
  check(styleCount === 20, 'hay 20 estilos preestablecidos originales', String(styleCount));
  check(await page.$( '[data-preset="0"]' ) !== null, 'el selector [data-preset="0"] sigue disponible');
  await shot('text-panel-01-basico.png');

  await page.click('[data-preset="0"]');
  await sleep(250);
  let text = await selectedText();
  check(text?.content === 'Texto' && text?.size === 72 && text?.weight === 800, 'el estilo predeterminado crea un texto editable', JSON.stringify(text && { content: text.content, size: text.size, weight: text.weight }));
  const contentCount = await page.$$eval('[data-text-content]', (items) => items.length);
  check(contentCount === 1, 'solo hay un campo [data-text-content] en Detalles', String(contentCount));

  await click('[data-preset="3"]');
  await sleep(250);
  text = await selectedText();
  check(text?.color === '#b6fdff' && text?.glow === true && Math.abs(text.strokeWidth - 0.025) < 0.0001, 'un estilo del catálogo aplica color, brillo y trazo', JSON.stringify(text && { color: text.color, glow: text.glow, strokeWidth: text.strokeWidth }));
  const chosenId = await page.evaluate(() => window.openCUP.state.selection);

  await page.click('[data-text-subtab="bubble"]');
  await sleep(200);
  const bubbleCount = await page.$$eval('[data-text-bubble]', (items) => items.length);
  check(bubbleCount === 9, 'el catálogo de burbujas tiene 9 opciones', String(bubbleCount));
  await shot('text-panel-02-burbuja.png');
  await click('[data-text-bubble="pill"]');
  await sleep(200);
  text = await selectedText();
  check(text?.background === '#00e5d0' && text?.backgroundRadius === 0.5 && text?.color === '#032a2b', 'la burbuja Pastilla se aplica al texto seleccionado', JSON.stringify(text && { background: text.background, backgroundRadius: text.backgroundRadius }));

  await page.click('[data-text-subtab="effect"]');
  await sleep(200);
  const effectCount = await page.$$eval('[data-text-effect]', (items) => items.length);
  check(effectCount === 9, 'el catálogo de efectos de texto tiene 9 opciones', String(effectCount));
  await shot('text-panel-03-efectos.png');
  await click('[data-text-effect="neon"]');
  await sleep(200);
  text = await selectedText();
  check(text?.glow === true && Math.abs(text?.glowBlur - 0.55) < 0.0001 && text?.strokeColor === '#008f8f', 'el efecto Neón actualiza brillo y contorno', JSON.stringify(text && { glow: text.glow, glowBlur: text.glowBlur, strokeColor: text.strokeColor }));

  await page.click('[data-text-tab="animation"]');
  await sleep(200);
  const animationCount = await page.$$eval('[data-text-animation]', (items) => items.length);
  check(animationCount === 20, 'la pestaña Animación incluye 19 animaciones y el botón Quitar', String(animationCount));
  await shot('text-panel-04-animacion.png');
  await click('[data-text-animation="loop-wave"]');
  await sleep(200);
  text = await selectedText();
  check(text?.animation === 'loop-wave' && text?.animationPhase === 'loop' && text?.animationDuration === 2.4, 'la animación Onda se guarda con su fase y duración', JSON.stringify(text && { animation: text.animation, animationPhase: text.animationPhase, animationDuration: text.animationDuration }));
  check(await page.$eval('[data-text-animation="loop-wave"]', (el) => el.classList.contains('active')), 'la animación aplicada aparece como activa');

  await page.click('[data-text-tab="tracking"]');
  await sleep(200);
  const trackingText = await page.$eval('.text-track-hero', (el) => el.innerText);
  check(/Seguimiento/.test(trackingText) && /local/i.test(trackingText), 'Seguimiento explica su límite sin conexión');
  await shot('text-panel-05-seguimiento.png');

  await page.click('[data-text-tab="tts"]');
  await sleep(200);
  check(await page.$('[data-tts-play]') !== null && await page.$('[data-tts-stop]') !== null && await page.$('[data-tts-text]') !== null, 'Texto a voz ofrece vista previa local y parada');
  const ttsValue = await page.$eval('[data-tts-text]', (el) => el.value);
  check(ttsValue.length > 0, 'Texto a voz toma el texto seleccionado como muestra', ttsValue.slice(0, 40));
  await shot('text-panel-06-texto-a-voz.png');

  await page.click('[data-text-tab="avatar"]');
  await sleep(200);
  const avatarCount = await page.$$eval('[data-avatar]', (items) => items.length);
  check(avatarCount === 8, 'Avatares de IA mantiene los 8 avatares locales', String(avatarCount));
  await shot('text-panel-07-avatares.png');

  await page.click('[data-text-tab="text"]');
  await sleep(250);
  await page.evaluate((id) => window.openCUP.command('selectClip', { id }), chosenId);
  await sleep(250);
  check((await selectedClip())?.id === chosenId, 'el texto sigue seleccionado al volver a Texto');

  await setRange('letterSpacing', 20);
  await setRange('lineSpacing', 150);
  text = await selectedText();
  check(Math.abs(text.letterSpacing - 0.2) < 0.0001 && Math.abs(text.lineSpacing - 1.5) < 0.0001, 'espaciado de carácter y de línea actualizan el texto', JSON.stringify({ letterSpacing: text.letterSpacing, lineSpacing: text.lineSpacing }));

  await click('[data-text-align="right"]');
  await click('[data-text-toggle="bold"]');
  await click('[data-text-toggle="italic"]');
  await click('[data-text-toggle="underline"]');
  await click('[data-text-case="upper"]');
  text = await selectedText();
  check(text.align === 'right' && text.bold && text.italic && text.underline && text.textCase === 'upper', 'alineación, negrita, cursiva, subrayado y mayúsculas se guardan');

  await click('[data-text-style-apply="gold"]');
  await sleep(200);
  text = await selectedText();
  check(text?.color === '#ffd76a' && text?.strokeColor === '#7a4b00', 'un estilo de Detalles se aplica al clip seleccionado', JSON.stringify(text && { color: text.color, strokeColor: text.strokeColor }));

  await setRange('scale', 150);
  await setRange('x', 12);
  await setRange('y', -8);
  await setRange('rotate', 15);
  await click('[data-text-toggle="flipX"]');
  await click('[data-text-toggle="flipY"]');
  let clip = await selectedClip();
  check(
    Math.abs(clip.scale - 1.5) < 0.0001 && Math.abs(clip.x - 12) < 0.0001
      && Math.abs(clip.y + 8) < 0.0001 && Math.abs(clip.rotate - 15) < 0.0001
      && clip.text.flipX === true && clip.text.flipY === true,
    'transformación del texto actualiza escala, posición, rotación y volteo',
    JSON.stringify({ scale: clip.scale, x: clip.x, y: clip.y, rotate: clip.rotate, flipX: clip.text.flipX, flipY: clip.text.flipY }),
  );

  await setRange('opacity', 70);
  text = await selectedText();
  check(Math.abs(text.opacity - 0.7) < 0.0001, 'la opacidad del texto se guarda', String(text.opacity));

  await setColor('strokeColor', '#112233');
  await setRange('strokeWidth', 8);
  await select('[data-text-select="strokeMode"]', 'inside');
  text = await selectedText();
  check(text.strokeColor === '#112233' && Math.abs(text.strokeWidth - 0.08) < 0.0001 && text.strokeMode === 'inside', 'trazo: color, grosor y posición', JSON.stringify({ strokeColor: text.strokeColor, strokeWidth: text.strokeWidth, strokeMode: text.strokeMode }));

  await setColor('background', '#3355ff');
  await setRange('backgroundAlpha', 60);
  await setRange('backgroundRadius', 30);
  await setRange('backgroundPadding', 35);
  await select('[data-text-select="backgroundStyle"]', 'pill');
  await setColor('backgroundBorderColor', '#ffffff');
  await setRange('backgroundBorderWidth', 3);
  text = await selectedText();
  check(
    text.background === '#3355ff' && Math.abs(text.backgroundAlpha - 0.6) < 0.0001
      && Math.abs(text.backgroundRadius - 0.3) < 0.0001 && Math.abs(text.backgroundPadding - 0.35) < 0.0001
      && text.backgroundStyle === 'pill' && text.backgroundBorderColor === '#ffffff'
      && Math.abs(text.backgroundBorderWidth - 0.03) < 0.0001,
    'fondo: color, opacidad, radio, margen, estilo y borde',
    JSON.stringify({ background: text.background, alpha: text.backgroundAlpha, radius: text.backgroundRadius, padding: text.backgroundPadding, style: text.backgroundStyle }),
  );

  await click('[data-text-toggle="glow"]');
  text = await selectedText();
  check(text.glow === false, 'el resplandor se puede desactivar');
  await click('[data-text-toggle="glow"]');
  await setColor('glowColor', '#ff7a00');
  await setRange('glowBlur', 80);
  text = await selectedText();
  check(text.glow === true && text.glowColor === '#ff7a00' && Math.abs(text.glowBlur - 0.8) < 0.0001, 'brillo: activación, color e intensidad', JSON.stringify({ glow: text.glow, glowColor: text.glowColor, glowBlur: text.glowBlur }));

  await click('[data-text-toggle="shadow"]');
  text = await selectedText();
  check(text.shadow === false, 'la sombra se puede desactivar');
  await click('[data-text-toggle="shadow"]');
  await setColor('shadowColor', '#102030');
  await setRange('shadowDistance', 12);
  await setRange('shadowAngle', 45);
  await setRange('shadowBlur', 40);
  await setRange('shadowAlpha', 50);
  text = await selectedText();
  check(
    text.shadow === true && text.shadowColor === '#102030' && Math.abs(text.shadowDistance - 0.12) < 0.0001
      && Math.abs(text.shadowAngle - 45) < 0.0001 && Math.abs(text.shadowBlur - 0.4) < 0.0001
      && Math.abs(text.shadowAlpha - 0.5) < 0.0001,
    'sombra: color, distancia, ángulo, desenfoque y opacidad',
    JSON.stringify({ shadowColor: text.shadowColor, distance: text.shadowDistance, angle: text.shadowAngle, blur: text.shadowBlur, alpha: text.shadowAlpha }),
  );

  await setRange('curve', 40);
  await select('[data-text-select="font"]', 'Georgia');
  await select('[data-text-select="weight"]', '900');
  await number('[data-text-number="size"]', 120);
  text = await selectedText();
  check(text.curve === 40 && text.font === 'Georgia' && Number(text.weight) === 900 && text.size === 120, 'curva, fuente, grosor y tamaño actualizan el texto', JSON.stringify({ curve: text.curve, font: text.font, weight: text.weight, size: text.size }));

  await select('[data-text-animation-select]', 'loop-pulse');
  text = await selectedText();
  check(text.animation === 'loop-pulse' && text.animationPhase === 'loop' && text.animationDuration === 1.6, 'el selector de animación de Detalles guarda la animación', JSON.stringify({ animation: text.animation, phase: text.animationPhase, duration: text.animationDuration }));

  await shot('text-panel-08-detalles.png');

  await page.evaluate(() => {
    const details = document.querySelector('[data-details]');
    const group = [...details.querySelectorAll('.tx-group')].find((item) => item.querySelector('summary')?.textContent.trim() === 'Transformación');
    group?.scrollIntoView({ block: 'start' });
  });
  await sleep(150);
  await shot('text-panel-09-transformacion.png');

  await click('[data-text-reset]');
  await sleep(250);
  text = await selectedText();
  check(
    text.content === 'NEÓN' && text.size === 72 && text.color === '#ffffff' && text.background === 'transparent'
      && text.curve === 0 && text.font === 'Inter' && text.animation === 'none' && text.letterSpacing === 0,
    'Restablecer estilo conserva el contenido y vuelve a los valores iniciales',
    JSON.stringify({ content: text.content, size: text.size, color: text.color, background: text.background, curve: text.curve }),
  );

  const renderMetrics = await page.evaluate(async () => {
    const { renderFrame } = await import('/src/renderer.js');
    const canvas = document.createElement('canvas');
    canvas.width = 640;
    canvas.height = 360;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    const base = {
      id: 'clip_render',
      kind: 'text',
      start: 0,
      duration: 5,
      x: 0,
      y: 0,
      scale: 1,
      rotate: 0,
      opacity: 1,
      filter: 'none',
      text: {
        content: 'openCUP',
        size: 96,
        weight: 900,
        color: '#ffffff',
        font: 'Arial',
        align: 'center',
        x: 0,
        y: 0,
        letterSpacing: 0,
        lineSpacing: 1.18,
        textCase: 'none',
        opacity: 1,
        shadow: false,
        background: 'transparent',
        strokeWidth: 0,
        glow: false,
        curve: 0,
        animation: 'none',
      },
    };
    const render = (patch = {}, time = 2.5) => {
      const clip = {
        ...base,
        ...patch,
        text: { ...base.text, ...(patch.text || {}) },
      };
      const project = { tracks: [{ id: 'text-track', type: 'text', name: 'Texto', clips: [clip] }] };
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      renderFrame(ctx, canvas.width, canvas.height, project, new Map(), time, { background: '#000000' });
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      let pixels = 0;
      let minX = canvas.width;
      let minY = canvas.height;
      let maxX = 0;
      let maxY = 0;
      let red = 0;
      let blue = 0;
      let sumY = 0;
      for (let y = 0; y < canvas.height; y += 1) {
        for (let x = 0; x < canvas.width; x += 1) {
          const index = (y * canvas.width + x) * 4;
          const r = data[index];
          const g = data[index + 1];
          const b = data[index + 2];
          if (r + g + b < 24) continue;
          pixels += 1;
          sumY += y;
          if (x < minX) minX = x;
          if (y < minY) minY = y;
          if (x > maxX) maxX = x;
          if (y > maxY) maxY = y;
          if (r > 10 && r > g * 1.5 && r > b * 1.5) red += 1;
          if (b > 150 && b > r * 1.6 && b > g * 1.25) blue += 1;
        }
      }
      return {
        pixels,
        red,
        blue,
        width: maxX - minX,
        height: maxY - minY,
        centerY: pixels ? sumY / pixels : 0,
      };
    };
    return {
      plain: render(),
      bordered: render({ text: { background: '#3355ff', backgroundAlpha: 0.85, backgroundRadius: 0.3, backgroundPadding: 0.3 } }),
      glow: render({ text: { glow: true, glowColor: '#ff3300', glowBlur: 0.9, color: '#ffffff', size: 170 } }),
      stroked: render({ text: { strokeWidth: 0.12, strokeColor: '#ffffff', color: '#000000' } }),
      flat: render({ text: { curve: 0 } }),
      curved: render({ text: { curve: 70 } }),
      typeStart: render({ text: { animation: 'in-typewriter', animationPhase: 'in', animationDuration: 1.2 } }, 0.06),
      typeEnd: render({ text: { animation: 'in-typewriter', animationPhase: 'in', animationDuration: 1.2 } }, 1.25),
      waveA: render({ text: { animation: 'loop-wave', animationPhase: 'loop' } }, 2.5),
      waveB: render({ text: { animation: 'loop-wave', animationPhase: 'loop' } }, 3.0),
      scaled: render({ scale: 2 }),
      transparent: render({ text: { opacity: 0.2 } }),
    };
  });

  check(renderMetrics.plain.pixels > 1000, 'el renderizador dibuja el texto', String(renderMetrics.plain.pixels));
  check(renderMetrics.bordered.blue > 5000, 'el fondo de burbuja se dibuja en el render', String(renderMetrics.bordered.blue));
  check(renderMetrics.glow.red > 800, 'el resplandor se dibuja en el render', String(renderMetrics.glow.red));
  check(renderMetrics.stroked.width > renderMetrics.plain.width + 4, 'el trazo amplía el texto renderizado', `${renderMetrics.plain.width} -> ${renderMetrics.stroked.width}`);
  check(renderMetrics.curved.height > renderMetrics.flat.height + 20, 'la curva cambia la altura del texto renderizado', `${renderMetrics.flat.height} -> ${renderMetrics.curved.height}`);
  check(renderMetrics.typeStart.pixels < renderMetrics.typeEnd.pixels * 0.6, 'la animación de máquina de escribir revela el texto progresivamente', `${renderMetrics.typeStart.pixels} / ${renderMetrics.typeEnd.pixels}`);
  check(Math.abs(renderMetrics.waveA.centerY - renderMetrics.waveB.centerY) > 2, 'la animación de bucle mueve el texto con el tiempo', `${renderMetrics.waveA.centerY.toFixed(1)} / ${renderMetrics.waveB.centerY.toFixed(1)}`);
  check(renderMetrics.scaled.width > renderMetrics.plain.width * 1.5, 'la escala del clip afecta al texto renderizado', `${renderMetrics.plain.width} -> ${renderMetrics.scaled.width}`);
  check(Math.abs(renderMetrics.transparent.width - renderMetrics.plain.width) <= 2 && Math.abs(renderMetrics.transparent.height - renderMetrics.plain.height) <= 2, 'la opacidad del texto no altera la geometría renderizada');

  const previewPixels = await page.evaluate(() => {
    const canvas = document.querySelector('.stage canvas');
    if (!canvas) return null;
    const ctx = canvas.getContext('2d');
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    let lit = 0;
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] + data[i + 1] + data[i + 2] > 90) lit += 1;
    }
    return { width: canvas.width, height: canvas.height, lit };
  });
  check(previewPixels && previewPixels.lit > 100, 'el texto aparece en el canvas del preview', JSON.stringify(previewPixels));
  await shot('text-panel-10-preview.png');

  await page.setViewport({ width: 1000, height: 700 });
  await sleep(250);
  await page.click('[data-text-tab="text"]');
  await sleep(200);
  await page.click('[data-text-subtab="basic"]');
  await sleep(200);
  const narrow = await page.evaluate(() => ({
    overflow: document.documentElement.scrollWidth > innerWidth + 1,
    cards: document.querySelectorAll('[data-preset]').length,
    panelWidth: document.querySelector('.left-col')?.getBoundingClientRect().width,
    contentWidth: document.querySelector('.text-panel-content')?.getBoundingClientRect().width,
  }));
  check(!narrow.overflow && narrow.cards === 20 && narrow.contentWidth <= narrow.panelWidth + 1, 'el panel de texto no desborda a 1000x700', JSON.stringify(narrow));
  await shot('text-panel-11-1000.png');

  check(errors.length === 0, 'sin errores de consola', errors.join(' | '));
} finally {
  await browser.close();
}

console.log('---');
if (failures.length) {
  console.log(`FALLOS: ${failures.join(' | ')}`);
  process.exitCode = 1;
} else {
  console.log('TEXT PANEL TEST PASSED');
}
