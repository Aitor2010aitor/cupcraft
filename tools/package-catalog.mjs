// Ajusta el catálogo para la app empaquetada: las miniaturas cuyo fichero no
// viaja en el instalador (efectos/transiciones/stickers/texto: +2 GB) se apuntan
// a su URL remota, que ya era el fallback del onerror. Así la app instalada no
// pide ficheros que no lleva y no llena la consola de 404.
//
// Uso: node tools/package-catalog.mjs [rutaDist]
import fs from 'node:fs';
import path from 'node:path';

const dist = process.argv[2] || 'dist/openCUP';
const file = path.join(dist, 'src', 'capcut-panels.js');
if (!fs.existsSync(file)) {
  console.log('package-catalog: no existe', file);
  process.exit(0);
}

const exists = (img) => fs.existsSync(path.join(dist, ...img.split('/')));
const swap = (m, img, url) => (img.startsWith('assets/') && !exists(img) ? `"img":"${url}","url":"${url}"` : m);

let text = fs.readFileSync(file, 'utf8');
let before = text;
text = text.replace(/"img":"([^"]+)","url":"([^"]+)"/g, swap);
text = text.replace(/"url":"([^"]+)","img":"([^"]+)"/g, (m, url, img) =>
  (img.startsWith('assets/') && !exists(img) ? `"url":"${url}","img":"${url}"` : m));

if (text !== before) fs.writeFileSync(file, text, 'utf8');

// Informe: cuántas siguen apuntando a un local que no está en el paquete.
const pending = [...text.matchAll(/"img":"(assets\/[^"]+)"/g)].map((m) => m[1]).filter((p) => !exists(p));
console.log(`package-catalog: ${dist} → remotas aplicadas=${text !== before ? 'sí' : 'no'}, locales aún ausentes=${pending.length}`);
if (pending.length) console.log('  ej:', pending.slice(0, 3).join(', '));
