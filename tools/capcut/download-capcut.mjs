import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const OUT = 'C:/Users/aitor/Documents/deepseek-harness/default-workspace/capcut-dl';
const catalog = JSON.parse(fs.readFileSync('catalog/capcut-catalog.json', 'utf8'));

const args = new Map(process.argv.slice(2).map((a) => a.split('=')));
const mode = args.get('--mode') ?? 'covers';
const typeFilter = args.get('--type') ? Number(args.get('--type')) : null;
const limit = args.get('--limit') ? Number(args.get('--limit')) : Infinity;
const concurrency = Number(args.get('--concurrency') ?? 8);

let items = catalog.effects.filter((e) => (typeFilter === null ? true : e.type === typeFilter));
if (mode === 'pkgs') items = items.filter((e) => e.packageUrl);
items = items.slice(0, limit);

fs.mkdirSync(OUT, { recursive: true });
const manifestPath = path.join(OUT, `${mode}${typeFilter ? '-t' + typeFilter : ''}.json`);
const manifest = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, 'utf8')) : { entries: {} };

const extOf = (url, ct) => {
  if (ct?.includes('png')) return '.png';
  if (ct?.includes('webp')) return '.webp';
  if (ct?.includes('jpeg')) return '.jpg';
  if (ct?.includes('gif')) return '.gif';
  if (ct?.includes('zip')) return '.zip';
  const m = /\.(png|jpe?g|webp|gif|zip)(\?|$)/i.exec(url);
  return m ? '.' + m[1].toLowerCase().replace('jpeg', 'jpg') : '.bin';
};

let done = 0, bytes = 0, failed = 0;
const queue = [...items];

async function worker() {
  while (queue.length) {
    const item = queue.shift();
    const url = mode === 'covers' ? item.coverUrl : item.packageUrl;
    if (!url) continue;
    const prev = manifest.entries[item.id];
    if (prev?.ok) { done++; continue; }
    try {
      const res = await fetch(url, { redirect: 'follow' });
      if (!res.ok) throw new Error('HTTP ' + res.status);
      const buf = Buffer.from(await res.arrayBuffer());
      const ext = extOf(url, res.headers.get('content-type'));
      const dir = path.join(OUT, mode, String(item.type));
      fs.mkdirSync(dir, { recursive: true });
      const file = path.join(dir, item.id + ext);
      fs.writeFileSync(file, buf);
      manifest.entries[item.id] = {
        ok: true, type: item.type, title: item.title, file: path.relative(OUT, file),
        bytes: buf.length, sha256: crypto.createHash('sha256').update(buf).digest('hex'),
      };
      done++; bytes += buf.length;
      if (done % 100 === 0) {
        fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 0));
        console.log(`  ${done}/${items.length}  ${(bytes / 1048576).toFixed(1)} MB`);
      }
    } catch (error) {
      failed++;
      manifest.entries[item.id] = { ok: false, type: item.type, title: item.title, error: error.message };
    }
  }
}

await Promise.all(Array.from({ length: concurrency }, worker));
fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 0));
console.log(`mode=${mode} type=${typeFilter ?? 'all'} items=${items.length} ok=${done} failed=${failed} size=${(bytes / 1048576).toFixed(1)} MB`);
console.log('manifest:', manifestPath);
