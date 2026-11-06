// Renombra el nombre visible "openCUP" → "cupcraft" en todo el repositorio.
// Los identificadores técnicos (window.openCUP, openCUP-db, variables de
// entorno, token, %LOCALAPPDATA%\openCUP, etc.) quedan intactos.
// Uso: node tools/rename-cupcraft.mjs [--dry]
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

const root = process.cwd();
const dry = process.argv.includes('--dry');

// Fragmentos que NUNCA se tocan (se listan antes que "openCUP" en la
// alternancia para que la coincidencia más larga gane).
const KEEP = [
  'openCUP-ai-token',
  'openCUP_AI_',
  'openCUP_CAPCUT_ROOT',
  'openCUP-db',
  'openCUP-project',
  'openCUP-media',
  'openCUP-Token',
  'openCUP-token',
  'openCUP.autoEdit',
  'openCUP.adjust.presets',
  'openCUP-ai-client',
  'window.openCUP',
  'openCUPDesktopWindow',
  'openCUP.Desktop.SingleInstance',
  'openCUP_DESKTOP',
  'openCUP/1.0',
  'openCUP.log',
  'openCUP.rc',
  'openCUP.res',
  'openCUP.ico',
  'LOCALAPPDATA%\\openCUP', // %LOCALAPPDATA%\openCUP
  '\\\\openCUP',            // literal C++ L"\\openCUP"
  "app: 'openCUP'",
  'app: "openCUP"',
];

const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const rx = new RegExp(KEEP.map(esc).join('|') + '|openCUP', 'g');

const SKIP_EXT = new Set([
  '.png', '.jpg', '.jpeg', '.webp', '.gif', '.ico', '.svg', '.vf',
  '.wav', '.mp3', '.m4a', '.mp4', '.webm', '.mov', '.ttf', '.otf',
  '.woff', '.woff2', '.pdf', '.zip', '.exe', '.dll', '.bin', '.res',
  '.pdb', '.lnk', '.cur', '.ani', '.ico', '.DS_Store',
]);

const files = execSync('git ls-files -z', { cwd: root, maxBuffer: 128 * 1024 * 1024 })
  .toString('utf8').split('\0').filter(Boolean);

let changedFiles = 0;
let totalHits = 0;
const report = [];

for (const rel of files) {
  const ext = path.extname(rel).toLowerCase();
  if (SKIP_EXT.has(ext)) continue;
  const abs = path.join(root, rel);
  let text;
  try {
    const buf = fs.readFileSync(abs);
    // Heurística: binario si hay bytes nulos.
    if (buf.includes(0)) continue;
    text = buf.toString('utf8');
  } catch { continue; }
  if (!text.includes('openCUP')) continue;
  let hits = 0;
  const out = text.replace(rx, (m) => {
    if (m !== 'openCUP') return m; // protegido
    hits++;
    return 'cupcraft';
  });
  if (hits === 0) continue;
  changedFiles++;
  totalHits += hits;
  report.push(`  ${rel}: ${hits}`);
  if (!dry) fs.writeFileSync(abs, out, 'utf8');
}

console.log(`${dry ? '[dry] ' : ''}${totalHits} sustituciones en ${changedFiles} archivos:`);
console.log(report.join('\n'));
