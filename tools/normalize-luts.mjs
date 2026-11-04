// Normaliza los LUT de assets/luts a PNG 512x512 (los JPEG y los atlas grandes
// se reescalen; el shader capcut-lut asume rejilla 8x8 sobre 512x512).
import fs from 'node:fs';
import path from 'node:path';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const DIR = path.resolve('assets/luts');

const targets = [];
for (const f of fs.readdirSync(DIR)) {
  const p = path.join(DIR, f);
  const b = fs.readFileSync(p);
  const isPng = b[0] === 0x89 && b[1] === 0x50;
  const w = isPng ? b.readUInt32BE(16) : 0;
  const h = isPng ? b.readUInt32BE(20) : 0;
  if (!isPng || w !== 512 || h !== 512) targets.push({ file: f, isPng, w, h });
}
console.log('a normalizar:', targets.length);

const browser = await puppeteer.launch({ executablePath: CHROME, headless: true });
const page = await browser.newPage();
await page.goto('http://127.0.0.1:8787/', { waitUntil: 'domcontentloaded' });

let ok = 0;
const fails = [];
for (const t of targets) {
  const p = path.join(DIR, t.file);
  const dataUrl = `data:image/${t.isPng ? 'png' : 'jpeg'};base64,${fs.readFileSync(p).toString('base64')}`;
  try {
    const out = await page.evaluate(async (src) => {
      const img = new Image();
      await new Promise((res, rej) => { img.onload = res; img.onerror = rej; img.src = src; });
      const c = document.createElement('canvas');
      c.width = 512; c.height = 512;
      const ctx = c.getContext('2d');
      ctx.imageSmoothingEnabled = true;
      ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(img, 0, 0, 512, 512);
      return c.toDataURL('image/png');
    }, dataUrl);
    if (!out.startsWith('data:image/png;base64,')) { fails.push(t.file + ' sin salida'); continue; }
    fs.writeFileSync(p, Buffer.from(out.split(',')[1], 'base64'));
    ok++;
  } catch (e) {
    fails.push(t.file + ': ' + e.message);
  }
}
await browser.close();
console.log('normalizados:', ok, 'fallo:', fails.length);
fails.forEach((f) => console.log('   ', f));

// revalidar todo
let bad = 0, other = 0;
for (const f of fs.readdirSync(DIR)) {
  const b = fs.readFileSync(path.join(DIR, f));
  const isPng = b[0] === 0x89 && b[1] === 0x50;
  if (!isPng) { other++; continue; }
  const t = b.subarray(b.length - 8);
  const iend = t[0] === 0x49 && t[1] === 0x45 && t[2] === 0x4e && t[3] === 0x44;
  if (!iend || b.readUInt32BE(16) !== 512 || b.readUInt32BE(20) !== 512) bad++;
}
console.log(`validacion: PNG 512x512=${fs.readdirSync(DIR).length - bad - other} malos=${bad} otros=${other}`);
