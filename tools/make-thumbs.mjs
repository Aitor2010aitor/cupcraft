// Genera miniaturas ligeras (320 px, WebP) de todo el catálogo CapCut para que
// el instalador pueda llevarlas: así la app instalada muestra las portadas de
// efectos/transiciones/stickers/texto también sin internet (los packs originales
// pesan +2 GB y no se empaquetan). 320 px se ven nítidas también en pantallas
// de alta densidad (2x), donde a 160 px salían borrosas.
//
// Se usa Chrome (puppeteer) porque decodifica webp y jpg por igual y reencodifica
// por canvas sin dependencias nativas.
//
// Uso: node tools/make-thumbs.mjs [tamaño] [calidad]
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import http from 'node:http';
import puppeteer from 'puppeteer-core';

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const ROOT = process.cwd();
const BASE = 'assets/capcut';
const OUT = path.join(ROOT, BASE, 'thumbs');
const SIZE = Number(process.argv[2]) || 320;
const QUALITY = Number(process.argv[3]) || 80;
const PORT = 8123;
const CHUNK = 120;

const isImage = (f) => /\.(jpe?g|png|webp|gif)$/i.test(f);

/* --- 1. Inventario de imágenes de origen. --- */
const sources = [];
const covers = path.join(ROOT, BASE, 'covers');
if (fs.existsSync(covers)) {
  for (const f of fs.readdirSync(covers)) if (isImage(f)) sources.push(`${BASE}/covers/${f}`);
}
const panels = path.join(ROOT, BASE, 'panels');
if (fs.existsSync(panels)) {
  for (const key of fs.readdirSync(panels)) {
    const dir = path.join(panels, key);
    if (!fs.statSync(dir).isDirectory()) continue;
    for (const f of fs.readdirSync(dir)) if (isImage(f)) sources.push(`${BASE}/panels/${key}/${f}`);
  }
}
if (!sources.length) {
  console.log('make-thumbs: no hay imágenes de origen en', BASE);
  process.exit(0);
}

const outPath = (src) => path.join(OUT, src.slice(BASE.length + 1).replace(/\.\w+$/, '.webp'));
let pending = sources.filter((src) => {
  const out = outPath(src);
  return !fs.existsSync(out) || fs.statSync(out).mtimeMs < fs.statSync(path.join(ROOT, src)).mtimeMs;
});
console.log(`make-thumbs: ${sources.length} imágenes, por generar: ${pending.length} (tamaño ${SIZE}px, calidad ${QUALITY})`);
if (!pending.length) { console.log('make-thumbs: nada que hacer'); process.exit(0); }

/* --- 2. Servidor estático mínimo (mismo origen que la página → canvas sin tinta). --- */
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  if (url.pathname === '/') {
    res.writeHead(200, { 'content-type': 'text/html' });
    res.end('<!doctype html><meta charset="utf-8"><title>thumbs</title>');
    return;
  }
  try {
    const file = path.join(ROOT, decodeURIComponent(url.pathname));
    if (!file.startsWith(ROOT)) { res.writeHead(403).end(); return; }
    const body = await fsp.readFile(file);
    res.writeHead(200, { 'content-type': 'image/*', 'cache-control': 'no-store' });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((resolve) => server.listen(PORT, '127.0.0.1', resolve));

/* --- 3. Chrome: decodifica, recorta al centro y reencodifica. --- */
const browser = await puppeteer.launch({ executablePath: CHROME, headless: true, args: ['--window-size=800,600'] });
const page = await browser.newPage();
await page.goto(`http://127.0.0.1:${PORT}/`, { waitUntil: 'domcontentloaded' });

let ok = 0;
let failed = 0;
const errors = [];
const started = Date.now();

for (let i = 0; i < pending.length; i += CHUNK) {
  const chunk = pending.slice(i, i + CHUNK);
  const results = await page.evaluate(async (items, size, quality) => {
    const out = [];
    for (const src of items) {
      try {
        const img = new Image();
        img.decoding = 'sync';
        await new Promise((resolve, reject) => {
          img.onload = resolve;
          img.onerror = () => reject(new Error('no se pudo cargar ' + src));
          img.src = src;
        });
        const w = img.naturalWidth;
        const h = img.naturalHeight;
        if (!w || !h) throw new Error('sin dimensiones');
        const side = Math.min(w, h);
        const canvas = document.createElement('canvas');
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext('2d');
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(img, (w - side) / 2, (h - side) / 2, side, side, 0, 0, size, size);
        out.push({ src, data: canvas.toDataURL('image/webp', quality) });
      } catch (e) {
        out.push({ src, error: String(e && e.message || e) });
      }
    }
    return out;
  }, chunk, SIZE, QUALITY);

  for (const item of results) {
    if (item.error) {
      failed++;
      if (errors.length < 8) errors.push(`${item.src}: ${item.error}`);
      continue;
    }
    const out = outPath(item.src);
    await fsp.mkdir(path.dirname(out), { recursive: true });
    await fsp.writeFile(out, Buffer.from(item.data.split(',')[1], 'base64'));
    ok++;
  }
  if ((i + CHUNK) % 600 < CHUNK) {
    console.log(`  ${Math.min(i + CHUNK, pending.length)}/${pending.length}…`);
  }
}

await browser.close();
server.close();

const total = fs.existsSync(OUT)
  ? [...(function* walk(dir) {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) yield* walk(full);
        else yield full;
      }
    })(OUT)].reduce((sum, f) => sum + fs.statSync(f).size, 0)
  : 0;
const seconds = ((Date.now() - started) / 1000).toFixed(1);
console.log(`make-thumbs: ${ok} miniaturas, ${failed} fallos, ${(total / 1024 / 1024).toFixed(1)} MB en ${seconds} s`);
if (errors.length) console.log('  fallos:', errors.join(' | '));
process.exit(failed > sources.length * 0.05 ? 1 : 0);
