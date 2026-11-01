import fs from 'node:fs';
import path from 'node:path';

const data = JSON.parse(fs.readFileSync('catalog/panels.json', 'utf8'));
const OUT = 'C:/Users/aitor/Documents/deepseek-harness/default-workspace/capcut-dl/covers-panels';
fs.mkdirSync(OUT, { recursive: true });

const jobs = [];
for (const [key, panel] of Object.entries(data.panels)) {
  for (const item of panel.items) jobs.push({ key, item });
}
let done = 0, failed = 0;
const queue = [...jobs];

const worker = async () => {
  while (queue.length) {
    const { key, item } = queue.shift();
    const dir = path.join(OUT, key);
    fs.mkdirSync(dir, { recursive: true });
    const dest = path.join(dir, `${item.id}.webp`);
    if (fs.existsSync(dest)) { done++; continue; }
    try {
      const res = await fetch(item.cover);
      if (!res.ok) throw new Error('HTTP ' + res.status);
      fs.writeFileSync(dest, Buffer.from(await res.arrayBuffer()));
      done++;
      if (done % 200 === 0) console.log(`  ${done}/${jobs.length}`);
    } catch { failed++; }
  }
};

await Promise.all(Array.from({ length: 10 }, worker));
let bytes = 0;
for (const k of Object.keys(data.panels)) {
  const dir = path.join(OUT, k);
  if (fs.existsSync(dir)) for (const f of fs.readdirSync(dir)) bytes += fs.statSync(path.join(dir, f)).size;
}
console.log(`portadas paneles: ${done}/${jobs.length} ok, ${failed} fallos, ${(bytes / 1048576).toFixed(0)} MB`);
