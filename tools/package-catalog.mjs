// Ajusta el catálogo para la app empaquetada: las miniaturas cuyo fichero no
// viaja en el instalador (efectos/transiciones/stickers/texto: +2 GB) se apuntan
// a su URL remota, que ya era el fallback del onerror. Así la app instalada no
// pide ficheros que no lleva y no llena la consola de 404.
//
// Uso: node tools/package-catalog.mjs [rutaDist]
import fs from 'node:fs';
import path from 'node:path';

const dist = process.argv[2] || 'dist/cupcraft';
const file = path.join(dist, 'src', 'capcut-panels.js');
if (!fs.existsSync(file)) {
  console.log('package-catalog: no existe', file);
  process.exit(0);
}

const exists = (img) => fs.existsSync(path.join(dist, ...img.split('/')));
// Las carátulas grandes no viajan en el paquete, pero sí su miniatura de 320 px
// (tools/make-thumbs.mjs): se prefiere la miniatura y solo se cae a la URL
// remota cuando no existe ni una ni otra.
const thumbOf = (img) => (img.startsWith('assets/capcut/')
  ? `assets/capcut/thumbs/${img.slice('assets/capcut/'.length).replace(/\.\w+$/, '.webp')}`
  : null);
const resolve = (img) => {
  if (!img.startsWith('assets/')) return img;
  if (exists(img)) return img;
  const thumb = thumbOf(img);
  return thumb && exists(thumb) ? thumb : null;
};

let text = fs.readFileSync(file, 'utf8');
let before = text;
text = text.replace(/"img":"([^"]+)","url":"([^"]+)"/g, (m, img, url) => {
  const next = resolve(img);
  if (next === img) return m;
  return next ? `"img":"${next}","url":"${url}"` : `"img":"${url}","url":"${url}"`;
});
text = text.replace(/"url":"([^"]+)","img":"([^"]+)"/g, (m, url, img) => {
  const next = resolve(img);
  if (next === img) return m;
  return next ? `"url":"${url}","img":"${next}"` : `"url":"${url}","img":"${url}"`;
});

if (text !== before) fs.writeFileSync(file, text, 'utf8');

// Informe: cuántas apuntan a miniatura local, a remoto o siguen ausentes.
const imgs = [...text.matchAll(/"img":"([^"]+)"/g)].map((m) => m[1]);
const local = imgs.filter((p) => p.startsWith('assets/') && exists(p));
const thumbs = local.filter((p) => p.includes('/thumbs/'));
const pending = imgs.filter((p) => p.startsWith('assets/') && !exists(p));
console.log(`package-catalog: ${dist} → cambiadas=${text !== before ? 'sí' : 'no'}, locales=${local.length} (miniaturas=${thumbs.length}), remotas=${imgs.length - local.length}, ausentes=${pending.length}`);
if (pending.length) console.log('  ej:', pending.slice(0, 3).join(', '));
