import { createReadStream } from 'node:fs';
import { access, readFile, readdir, stat } from 'node:fs/promises';
import { basename, extname, isAbsolute, join, normalize, resolve, sep } from 'node:path';
import { homedir } from 'node:os';

/* ------------------------------------------------------------------ */
/*  CapCut native draft reader                                         */
/*  Lee la carpeta de borradores de CapCut y la convierte al modelo    */
/*  interno de cupcraft. No modifica ningún archivo de CapCut.            */
/* ------------------------------------------------------------------ */

const local = process.env.LOCALAPPDATA || '';
const roaming = process.env.APPDATA || '';

const CANDIDATE_ROOTS = [
  process.env.openCUP_CAPCUT_ROOT,
  local && join(local, 'CapCut', 'User Data', 'Projects', 'com.lveditor.draft'),
  local && join(local, 'JianyingPro', 'User Data', 'Projects', 'com.lveditor.draft'),
  roaming && join(roaming, 'CapCut', 'User Data', 'Projects', 'com.lveditor.draft'),
  join(homedir(), 'Documents', 'CapCut', 'User Data', 'Projects', 'com.lveditor.draft'),
  join(homedir(), 'Documents', 'JianyingPro', 'User Data', 'Projects', 'com.lveditor.draft'),
].filter(Boolean);

export const capcutRoots = [...new Set(CANDIDATE_ROOTS.map((root) => resolve(root)))];

const MEDIA_EXTENSIONS = new Set([
  '.mp4', '.mov', '.m4v', '.mkv', '.webm', '.avi', '.wmv', '.flv',
  '.mp3', '.wav', '.m4a', '.aac', '.ogg', '.flac', '.opus',
  '.jpg', '.jpeg', '.png', '.webp', '.gif', '.bmp', '.heic', '.avif',
]);

const MIME = {
  '.mp4': 'video/mp4', '.mov': 'video/quicktime', '.m4v': 'video/x-m4v',
  '.mkv': 'video/x-matroska', '.webm': 'video/webm', '.avi': 'video/x-msvideo',
  '.wmv': 'video/x-ms-wmv', '.flv': 'video/x-flv',
  '.mp3': 'audio/mpeg', '.wav': 'audio/wav', '.m4a': 'audio/mp4',
  '.aac': 'audio/aac', '.ogg': 'audio/ogg', '.flac': 'audio/flac', '.opus': 'audio/ogg',
  '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.png': 'image/png',
  '.webp': 'image/webp', '.gif': 'image/gif', '.bmp': 'image/bmp',
  '.heic': 'image/heic', '.avif': 'image/avif',
};

const TEXT_EXT = new Set(['.json', '.txt']);
const allowedMedia = new Map();

export function encodePath(value) {
  return Buffer.from(String(value), 'utf8').toString('base64url');
}

export function decodePath(value) {
  try {
    return Buffer.from(String(value), 'base64url').toString('utf8');
  } catch {
    return '';
  }
}

function isInside(parent, child) {
  const root = parent.endsWith(sep) ? parent : parent + sep;
  return child === parent || child.startsWith(root);
}

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

function number(value, fallback = 0) {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

function secondsFromMicros(value) {
  return Math.max(0, number(value) / 1_000_000);
}

function normalizePath(raw, draftDir) {
  if (!raw || typeof raw !== 'string') return '';
  let value = raw.replace(/\\/g, '/').trim();
  if (!value) return '';
  if (/^file:/i.test(value)) {
    try { value = decodeURIComponent(new URL(value).pathname.replace(/^\/([A-Za-z]:)/, '$1')); } catch { /* ignore */ }
  }
  const normalized = value.replace(/\//g, sep);
  if (isAbsolute(normalized)) return normalize(normalized);
  return resolve(draftDir, normalized);
}

function safeMediaName(name, path) {
  const value = String(name || '').trim();
  if (value && !value.includes('\\') && !value.includes('/')) return value;
  return basename(path) || 'archivo';
}

function ratioFor(width, height) {
  if (!width || !height) return '16:9';
  const value = width / height;
  const options = [
    ['16:9', 16 / 9], ['9:16', 9 / 16], ['1:1', 1],
    ['4:3', 4 / 3], ['21:9', 21 / 9], ['4:5', 4 / 5],
  ];
  let best = options[0];
  for (const option of options) {
    if (Math.abs(option[1] - value) < Math.abs(best[1] - value)) best = option;
  }
  return best[0];
}

function parseTextColor(value, fallback) {
  if (Array.isArray(value) && value.length >= 3) {
    const scale = value.some((entry) => number(entry) > 1) ? 1 : 255;
    const [r, g, b] = value.slice(0, 3).map((entry) => Math.max(0, Math.min(255, Math.round(number(entry) * scale))));
    return `#${[r, g, b].map((entry) => entry.toString(16).padStart(2, '0')).join('')}`;
  }
  if (typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value)) return value;
  return fallback;
}

function parseTextMaterial(material, draftDir) {
  let parsed = {};
  try {
    parsed = JSON.parse(material.content || '{}');
  } catch {
    parsed = { text: material.content || '' };
  }
  const styles = Array.isArray(parsed.styles) ? parsed.styles : [];
  const first = styles[0] || {};
  const fill = first.fill?.content?.solid?.color;
  const color = parseTextColor(fill, parseTextColor(material.text_color, '#ffffff'));
  const size = Math.max(24, Math.min(240, number(first.size, number(material.font_size, 15)) * 6));
  const bold = number(material.bold_width) > 0 || number(first.bold) > 0;
  const italic = number(material.italic_degree) > 0;
  const weight = bold ? 700 : 500;
  const content = typeof parsed.text === 'string' ? parsed.text : String(material.content || '');
  let letterSpacing = number(material.letter_spacing, 0);
  if (Math.abs(letterSpacing) > 1) letterSpacing /= 100;
  letterSpacing = Math.max(-0.2, Math.min(0.6, letterSpacing));
  let rawLineSpacing = number(material.line_spacing, 0);
  if (Math.abs(rawLineSpacing) > 1) rawLineSpacing /= 100;
  const lineSpacing = Math.max(0.7, Math.min(3, 1 + rawLineSpacing));
  let curve = number(material.text_curve, 0);
  if (Math.abs(curve) <= 2) curve *= 100;
  curve = Math.max(-100, Math.min(100, curve));
  const borderColor = parseTextColor(material.border_color, '');
  const borderWidth = borderColor ? Math.max(0, Math.min(0.3, number(material.border_width, 0))) : 0;
  const backgroundStyle = number(material.background_style, 0);
  const backgroundColor = parseTextColor(material.background_color, '#000000');
  const hasBackground = Boolean(material.background_fill) || backgroundStyle > 0 ||
    (backgroundColor && backgroundColor.toLowerCase() !== '#000000');
  const backgroundWidth = number(material.background_width, 0.22);
  const backgroundHeight = number(material.background_height, 0.22);
  return {
    content: content.replace(/\\n/g, '\n'),
    size,
    weight,
    bold,
    italic,
    underline: Boolean(material.underline),
    color,
    align: material.alignment === 0 ? 'left' : material.alignment === 2 ? 'right' : 'center',
    font: 'Inter',
    x: 0,
    y: 0,
    letterSpacing,
    lineSpacing,
    textCase: 'none',
    opacity: Math.max(0, Math.min(1, number(material.global_alpha, 1) * number(material.text_alpha, 1))),
    shadow: Boolean(material.has_shadow),
    shadowColor: parseTextColor(material.shadow_color, '#000000'),
    shadowAlpha: Math.max(0, Math.min(1, number(material.shadow_alpha, 0.9))),
    shadowBlur: Math.max(0, Math.min(1, number(material.shadow_smoothing, 0.45))),
    shadowDistance: Math.max(0, Math.min(0.4, number(material.shadow_distance, 5) / 100)),
    shadowAngle: number(material.shadow_angle, 90),
    strokeColor: borderColor || '#000000',
    strokeWidth: borderWidth,
    strokeAlpha: Math.max(0, Math.min(1, number(material.border_alpha, 1))),
    strokeMode: material.border_mode === 2 ? 'inside' : material.border_mode === 1 ? 'center' : 'outer',
    background: hasBackground ? backgroundColor : 'transparent',
    backgroundAlpha: Math.max(0, Math.min(1, number(material.background_alpha, 1))),
    backgroundStyle: backgroundStyle === 2 ? 'pill' : backgroundStyle === 1 ? 'rounded' : 'square',
    backgroundRadius: Math.max(0, Math.min(0.5, number(material.background_round_radius, 0))),
    backgroundPadding: Math.max(0, Math.min(0.6, Math.max(backgroundWidth, backgroundHeight))),
    backgroundBorderColor: '#000000',
    backgroundBorderWidth: 0,
    glow: false,
    glowColor: '#00e5d0',
    glowBlur: 0.35,
    curve,
    flipX: Boolean(material.flip_x || material.flipX),
    flipY: Boolean(material.flip_y || material.flipY),
    animation: 'none',
    animationPhase: 'in',
    animationDuration: 0.6,
  };
}

function collectMaterials(content) {
  const buckets = content?.materials || {};
  const map = new Map();
  const ext = (bucket, kind) => {
    const list = Array.isArray(buckets[bucket]) ? buckets[bucket] : [];
    for (const item of list) {
      if (item?.id) map.set(item.id, { item, kind, bucket });
    }
  };
  ext('videos', 'video');
  ext('images', 'image');
  ext('audios', 'audio');
  ext('stickers', 'sticker');
  ext('texts', 'text');
  return map;
}

function materialToMedia(material, kind, draftDir, warnings) {
  const path = normalizePath(material.path, draftDir);
  if (!path) return null;
  const extension = extname(path).toLowerCase();
  if (!MEDIA_EXTENSIONS.has(extension)) {
    warnings.push(`Formato no soportado: ${basename(path)}`);
    return null;
  }
  const mediaKind = kind === 'sticker' ? 'image' : kind;
  return {
    capcutId: material.id,
    name: safeMediaName(material.material_name || material.name, path),
    kind: mediaKind,
    path,
    fileUrl: `/api/capcut/media?path=${encodePath(path)}`,
    duration: number(material.duration) / 1_000_000,
    width: number(material.width),
    height: number(material.height),
    size: number(material.size),
    mime: MIME[extension] || 'application/octet-stream',
    missing: false,
  };
}

function lookupRefs(segment, materials, content) {
  const refs = [];
  for (const id of segment.extra_material_refs || []) {
    const found = materials.get(id);
    if (found) refs.push(found);
    else {
      for (const list of Object.values(content?.materials || {})) {
        if (!Array.isArray(list)) continue;
        const item = list.find((entry) => entry?.id === id);
        if (item) { refs.push({ item, kind: 'ref', bucket: '' }); break; }
      }
    }
  }
  return refs;
}

function refNumber(refs, keys, fallback) {
  for (const { item } of refs) {
    for (const key of keys) {
      if (item && item[key] !== undefined && Number.isFinite(Number(item[key]))) return Number(item[key]);
    }
  }
  return fallback;
}

function normalizeColor(value) {
  if (!value || typeof value !== 'string') return '';
  if (value.startsWith('#')) return value;
  return '';
}

function normalizeImportedClip(segment, track, material, refs, mediaMap, draftDir, warnings) {
  const target = segment.target_timerange || {};
  const source = segment.source_timerange || {};
  const start = secondsFromMicros(target.start);
  let duration = secondsFromMicros(target.duration);
  if (!duration) duration = secondsFromMicros(source.duration);
  if (!duration) return null;
  const clip = {
    id: `cc_${segment.id || `${start}_${duration}`}`,
    kind: 'video',
    mediaId: null,
    name: '',
    start,
    duration,
    in: secondsFromMicros(source.start),
    speed: number(segment.speed, 1) || 1,
    volume: number(segment.volume, 1),
    opacity: Math.max(0, Math.min(1, number(segment.clip?.alpha, 1))),
    scale: Math.max(0.05, number(segment.clip?.scale?.x, 1) || 1),
    x: number(segment.clip?.transform?.x) * 50,
    y: number(segment.clip?.transform?.y) * 50,
    rotate: number(segment.clip?.rotation),
    filter: 'none',
    adjust: { brightness: 100, contrast: 100, saturate: 100 },
    fadeIn: 0,
    fadeOut: 0,
  };

  if (material?.kind === 'text') {
    clip.kind = 'text';
    clip.text = parseTextMaterial(material.item, draftDir);
    clip.name = (clip.text.content || 'Texto').replace(/\s+/g, ' ').slice(0, 24) || 'Texto';
  } else if (material) {
    const media = materialToMedia(material.item, material.kind, draftDir, warnings);
    if (!media) return null;
    clip.kind = media.kind;
    clip.mediaId = media.capcutId;
    clip.name = media.name;
    if (media.kind === 'audio') clip.name = media.name;
    if (!duration && media.duration) {
      clip.duration = media.duration;
    }
    if (media.kind === 'image' || media.kind === 'sticker') {
      clip.in = 0;
    }
  } else {
    // Pista sin material reconocible: se conserva solo si es audio/video con URL.
    return null;
  }

  const fadeIn = refNumber(refs, ['fade_in', 'fadeIn', 'audio_fade_in', 'video_fade_in'], 0);
  const fadeOut = refNumber(refs, ['fade_out', 'fadeOut', 'audio_fade_out', 'video_fade_out'], 0);
  if (fadeIn) clip.fadeIn = Math.min(clip.duration, secondsFromMicros(fadeIn));
  if (fadeOut) clip.fadeOut = Math.min(clip.duration, secondsFromMicros(fadeOut));

  const brightness = refNumber(refs, ['brightness', 'brightness_value'], null);
  const contrast = refNumber(refs, ['contrast', 'contrast_value'], null);
  const saturation = refNumber(refs, ['saturate', 'saturation', 'saturation_value'], null);
  if (brightness !== null) clip.adjust.brightness = brightness > 2 ? brightness : 100 + brightness * 100;
  if (contrast !== null) clip.adjust.contrast = contrast > 2 ? contrast : 100 + contrast * 100;
  if (saturation !== null) clip.adjust.saturate = saturation > 2 ? saturation : 100 + saturation * 100;

  for (const { item, bucket } of refs) {
    if (!item) continue;
    const filterName = String(item.name || item.filter_name || item.resource_name || '');
    if (bucket === 'filters' || item.type === 'filter' || item.filter_id) {
      const key = filterName.toLowerCase();
      const table = [
        ['mono', 'mono'], ['blanco y negro', 'mono'], ['noir', 'noir'],
        ['vintage', 'vintage'], ['retro', 'vintage'], ['vivid', 'vivid'],
        ['calido', 'warm'], ['cálido', 'warm'], ['warm', 'warm'],
        ['frio', 'cool'], ['frío', 'cool'], ['cool', 'cool'],
        ['fade', 'fade'], ['desvanecido', 'fade'],
      ];
      const match = table.find(([label]) => key.includes(label));
      if (match) clip.filter = match[1];
    }
  }

  const textColor = normalizeColor(clip.text?.color);
  if (clip.text && textColor) clip.text.color = textColor;
  return clip;
}

export async function scanCapCutProjects() {
  const projects = [];
  for (const root of capcutRoots) {
    if (!(await exists(root))) continue;
    let entries = [];
    try { entries = await readdir(root, { withFileTypes: true }); } catch { continue; }
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      const dir = join(root, entry.name);
      const contentPath = join(dir, 'draft_content.json');
      if (!(await exists(contentPath))) continue;
      let content = null;
      let meta = null;
      try { content = JSON.parse(await readFile(contentPath, 'utf8')); } catch { continue; }
      try { meta = JSON.parse(await readFile(join(dir, 'draft_meta_info.json'), 'utf8')); } catch { /* opcional */ }
      const info = await stat(contentPath).catch(() => null);
      const name = meta?.draft_name || entry.name;
      const width = number(content?.canvas_config?.width, 1920);
      const height = number(content?.canvas_config?.height, 1080);
      const coverCandidates = [
        meta?.draft_cover && normalizePath(meta.draft_cover, dir),
        join(dir, 'draft_cover.jpg'),
        join(dir, 'draft_cover.png'),
        join(dir, 'Resources', 'cover.jpg'),
      ].filter(Boolean);
      let cover = '';
      let coverUrl = '';
      for (const candidate of coverCandidates) {
        if (await exists(candidate)) {
          cover = candidate;
          coverUrl = `/api/capcut/cover?path=${encodePath(candidate)}`;
          allowedMedia.set(candidate, true);
          break;
        }
      }
      const counts = {
        video: Array.isArray(content?.materials?.videos) ? content.materials.videos.length : 0,
        image: Array.isArray(content?.materials?.images) ? content.materials.images.length : 0,
        audio: Array.isArray(content?.materials?.audios) ? content.materials.audios.length : 0,
        text: Array.isArray(content?.materials?.texts) ? content.materials.texts.length : 0,
      };
      const tracks = Array.isArray(content?.tracks) ? content.tracks : [];
      projects.push({
        id: encodePath(dir),
        folder: entry.name,
        name,
        duration: number(content?.duration) / 1_000_000,
        width,
        height,
        ratio: ratioFor(width, height),
        fps: number(content?.fps, 30),
        modified: info?.mtimeMs || 0,
        tracks: tracks.length,
        clips: tracks.reduce((sum, track) => sum + (track?.segments?.length || 0), 0),
        counts,
        cover,
        coverUrl,
        root,
      });
    }
  }
  projects.sort((a, b) => b.modified - a.modified);
  return projects;
}

function validateDraftId(id) {
  const dir = decodePath(id);
  if (!dir) throw new Error('Proyecto no válido');
  const resolved = resolve(dir);
  if (!capcutRoots.some((root) => isInside(root, resolved))) {
    throw new Error('La carpeta no pertenece a CapCut');
  }
  if (basename(resolved) !== resolved.split(sep).filter(Boolean).at(-1)) {
    throw new Error('Ruta no válida');
  }
  return resolved;
}

export async function readCapCutDraft(id) {
  const dir = validateDraftId(id);
  const contentPath = join(dir, 'draft_content.json');
  if (!(await exists(contentPath))) throw new Error('El borrador no contiene draft_content.json');
  const content = JSON.parse(await readFile(contentPath, 'utf8'));
  let meta = {};
  try { meta = JSON.parse(await readFile(join(dir, 'draft_meta_info.json'), 'utf8')); } catch { /* opcional */ }
  const warnings = [];
  const width = number(content?.canvas_config?.width, 1920);
  const height = number(content?.canvas_config?.height, 1080);
  const materials = collectMaterials(content);
  const mediaMap = new Map();
  const medias = [];

  for (const [materialId, entry] of materials) {
    if (!['video', 'image', 'audio', 'sticker'].includes(entry.kind)) continue;
    const media = materialToMedia(entry.item, entry.kind, dir, warnings);
    if (!media) continue;
    mediaMap.set(materialId, media);
    allowedMedia.set(media.path, true);
    medias.push(media);
  }

  const usedTrackNames = new Map();
  const tracks = [];
  for (const track of Array.isArray(content?.tracks) ? content.tracks : []) {
    const segments = Array.isArray(track.segments) ? track.segments : [];
    if (!segments.length) continue;
    const clips = [];
    for (const segment of segments) {
      const material = materials.get(segment.material_id) || null;
      const refs = lookupRefs(segment, materials, content);
      const clip = normalizeImportedClip(segment, track, material, refs, mediaMap, dir, warnings);
      if (!clip) continue;
      if (clip.mediaId) {
        const media = mediaMap.get(clip.mediaId);
        if (!media) continue;
        if (!(await exists(media.path))) {
          media.missing = true;
          warnings.push(`Archivo no encontrado: ${media.name}`);
        }
      }
      clips.push(clip);
    }
    if (!clips.length) continue;
    let type = 'video';
    if (clips.every((clip) => clip.kind === 'text')) type = 'text';
    else if (clips.every((clip) => clip.kind === 'audio')) type = 'audio';
    const baseName = type === 'text' ? 'Texto' : type === 'audio' ? 'Audio' : 'Video';
    const index = (usedTrackNames.get(type) || 0) + 1;
    usedTrackNames.set(type, index);
    tracks.push({
      id: `cap_track_${track.id || tracks.length + 1}`,
      type,
      name: `${baseName} ${index}`,
      clips,
    });
  }

  if (!tracks.some((track) => track.type === 'text')) {
    tracks.unshift({ id: 'cap_track_text', type: 'text', name: 'Texto 1', clips: [] });
  }
  if (!tracks.some((track) => track.type === 'video')) {
    tracks.push({ id: 'cap_track_video', type: 'video', name: 'Video 1', clips: [] });
  }
  if (!tracks.some((track) => track.type === 'audio')) {
    tracks.push({ id: 'cap_track_audio', type: 'audio', name: 'Audio 1', clips: [] });
  }

  const duration = number(content?.duration) / 1_000_000;
  const project = {
    name: meta?.draft_name || basename(dir),
    width,
    height,
    ratio: ratioFor(width, height),
    fps: number(content?.fps, 30),
    duration,
    createdAt: number(meta?.tm_draft_create) / 1000 || Date.now(),
    updatedAt: number(meta?.tm_draft_modified) / 1000 || Date.now(),
    tracks,
    media: medias,
    warnings: [...new Set(warnings)].slice(0, 40),
    source: { type: 'capcut', folder: dir, appVersion: content?.platform?.app_version || meta?.draft_new_version || '' },
  };
  return project;
}

export function isAllowedMedia(path) {
  return allowedMedia.has(resolve(path));
}

export function allowMedia(path) {
  allowedMedia.set(resolve(path), true);
}

export async function serveLocalMedia(req, res, path) {
  const resolved = resolve(path);
  if (!isAllowedMedia(resolved)) {
    res.writeHead(403).end('Forbidden');
    return;
  }
  let info;
  try { info = await stat(resolved); } catch { res.writeHead(404).end('Not found'); return; }
  if (!info.isFile()) { res.writeHead(404).end('Not found'); return; }
  const extension = extname(resolved).toLowerCase();
  const type = MIME[extension] || 'application/octet-stream';
  const range = req.headers.range;
  if (range) {
    const match = /bytes=(\d*)-(\d*)/.exec(range);
    if (match) {
      let start = match[1] ? Number(match[1]) : 0;
      let end = match[2] ? Number(match[2]) : info.size - 1;
      start = Math.max(0, Math.min(start, info.size - 1));
      end = Math.max(start, Math.min(end, info.size - 1));
      res.writeHead(206, {
        'Content-Type': type,
        'Content-Range': `bytes ${start}-${end}/${info.size}`,
        'Accept-Ranges': 'bytes',
        'Content-Length': end - start + 1,
        'Cache-Control': 'no-store',
      });
      createReadStream(resolved, { start, end }).pipe(res);
      return;
    }
  }
  res.writeHead(200, {
    'Content-Type': type,
    'Content-Length': info.size,
    'Accept-Ranges': 'bytes',
    'Cache-Control': 'no-store',
  });
  createReadStream(resolved).pipe(res);
}

export function capcutMime(path) {
  return MIME[extname(path).toLowerCase()] || 'application/octet-stream';
}
