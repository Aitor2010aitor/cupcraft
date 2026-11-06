import { FILTERS } from './store.js';
import { EFFECTS } from './library.js';
import { textAnimationById } from './text-catalog.js';
import { transformAt, keyframeValue } from './keyframes.js';

const clamp01 = (value) => Math.max(0, Math.min(1, value));

function waitEvent(target, event, timeout = 6000) {
  return new Promise((resolve) => {
    let timer = 0;
    const done = () => {
      target.removeEventListener(event, done);
      clearTimeout(timer);
      resolve();
    };
    timer = setTimeout(done, timeout);
    target.addEventListener(event, done, { once: true });
  });
}

export function activeLayers(project, time) {
  const layers = [];
  if (!project) return layers;
  for (const track of project.tracks || []) {
    if (track.hidden) continue;
    for (const clip of track.clips || []) {
      if (time >= clip.start - 0.0001 && time < clip.start + clip.duration) {
        layers.push({ clip, track, local: Math.max(0, time - clip.start) });
      }
    }
  }
  layers.sort((a, b) => {
    const rank = (kind) => (kind === 'text' ? 3 : kind === 'sticker' ? 2 : kind === 'audio' ? -1 : 1);
    return rank(a.clip.kind) - rank(b.clip.kind);
  });
  return layers;
}

export function clipAlpha(clip, local) {
  let alpha = keyframeValue(clip, 'opacity', (clip.start || 0) + local, clip.opacity ?? 1);
  const fadeIn = clip.fadeIn || 0;
  const hasTransition = clip.transition && clip.transition !== 'none';
  if (fadeIn > 0 && local < fadeIn && !hasTransition) alpha *= clamp01(local / fadeIn);
  const fadeOut = clip.fadeOut || 0;
  const remaining = clip.duration - local;
  if (fadeOut > 0 && remaining < fadeOut) alpha *= clamp01(remaining / fadeOut);
  return clamp01(alpha);
}

export function clipGain(clip, local) {
  let gain = clip.volume ?? 1;
  const fadeIn = clip.fadeIn || 0;
  if (fadeIn > 0 && local < fadeIn) gain *= clamp01(local / fadeIn);
  const fadeOut = clip.fadeOut || 0;
  const remaining = clip.duration - local;
  if (fadeOut > 0 && remaining < fadeOut) gain *= clamp01(remaining / fadeOut);
  return clamp01(gain);
}

function effectSpec(clip) {
  if (!clip?.effect || clip.effect === 'none') return null;
  return EFFECTS[clip.effect] || null;
}

/* ------------------------- Text layout / layer boxes ------------------------- */

const measureCanvas = document.createElement('canvas');
const measureCtx = measureCanvas.getContext('2d');

function textNumber(value, fallback = 0) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function applyTextCase(content, mode) {
  if (mode === 'upper') return content.toLocaleUpperCase('es');
  if (mode === 'lower') return content.toLocaleLowerCase('es');
  if (mode === 'capitalize') {
    return content.replace(/(^|[\s\n])(\p{L})/gu, (match, prefix, letter) => `${prefix}${letter.toLocaleUpperCase('es')}`);
  }
  return content;
}

function textFamily(value) {
  const name = String(value || 'Inter');
  const stack = name.includes(',') ? name : `"${name.replace(/"/g, '')}"`;
  return `${stack}, "Segoe UI", "Segoe UI Emoji", system-ui, sans-serif`;
}

function textFontSpec(value, fontSize, weight) {
  const italic = value.italic ? 'italic ' : '';
  const boldWeight = value.bold ? 900 : weight;
  return `${italic}${boldWeight} ${fontSize}px ${textFamily(value.font)}`;
}

function textLetterSpacing(text, fontSize) {
  return textNumber(text?.letterSpacing, 0) * fontSize;
}

function measureRun(ctx, run, spacing) {
  const width = ctx.measureText(run).width;
  return width + Math.max(0, Array.from(run).length - 1) * spacing;
}

function textPadding(text, fontSize) {
  return Math.max(0, textNumber(text?.backgroundPadding, 0.22)) * fontSize;
}

function textCurveExtra(text, fontSize) {
  return Math.abs(textNumber(text?.curve, 0)) / 100 * fontSize * 0.9;
}

export function textLayout(text, width, height, ctx = measureCtx) {
  const value = text || {};
  const fontSize = Math.max(6, textNumber(value.size, 72) * (height / 1080));
  const weight = Math.max(100, Math.min(900, textNumber(value.weight, 700)));
  const family = textFamily(value.font);
  const font = textFontSpec(value, fontSize, weight);
  const maxWidth = Math.max(40, width * 0.82);
  const content = applyTextCase(String(value.content ?? ''), value.textCase);
  const paragraphs = content.split('\n');
  const lines = [];
  const letterSpacing = textLetterSpacing(value, fontSize);
  ctx.font = font;
  for (const paragraph of paragraphs) {
    if (!paragraph) {
      lines.push('');
      continue;
    }
    const words = paragraph.split(/\s+/);
    let line = '';
    for (const word of words) {
      const candidate = line ? `${line} ${word}` : word;
      if (measureRun(ctx, candidate, letterSpacing) > maxWidth && line) {
        lines.push(line);
        line = word;
      } else {
        line = candidate;
      }
    }
    if (line) lines.push(line);
  }
  const lineSpacing = Math.max(0.6, Math.min(3, textNumber(value.lineSpacing, 1.18)));
  const lineHeight = fontSize * lineSpacing;
  const blockHeight = lines.length * lineHeight;
  const blockWidth = Math.max(...lines.map((line) => measureRun(ctx, line, letterSpacing)), fontSize);
  return {
    fontSize,
    weight,
    family,
    font,
    letterSpacing,
    maxWidth,
    lines,
    lineHeight,
    blockWidth,
    blockHeight,
    content,
  };
}

/* Devuelve la caja de una capa de texto/sticker en píxeles del canvas.
   Se usa tanto para dibujar los tiradores del preview como para el hit-testing. */
export function clipLayerBox(clip, width, height, time = null) {
  if (!clip) return null;
  const tr = transformAt(clip, time === null ? (clip.start || 0) : time);
  if (clip.kind === 'text') {
    const text = clip.text || {};
    const metrics = textLayout(text, width, height);
    if (!metrics.lines.length) return null;
    const hasBackground = text.background && text.background !== 'transparent';
    const pad = hasBackground ? textPadding(text, metrics.fontSize) : 0;
    const outline = textNumber(text.strokeWidth, 0) * metrics.fontSize * 0.6 +
      (text.glow ? textNumber(text.glowBlur, 0.35) * metrics.fontSize * 0.35 : 0);
    return {
      width: metrics.blockWidth + pad * 2 + outline * 2 + metrics.fontSize * 0.1,
      height: metrics.blockHeight + pad * 2 + outline * 2 +
        textCurveExtra(text, metrics.fontSize) + metrics.fontSize * 0.08,
      cx: width / 2 + ((tr.x + (text.x || 0)) / 100) * width,
      cy: height / 2 + ((tr.y + (text.y || 0)) / 100) * height,
      rotation: tr.rotate,
      scale: tr.scaleX,
      scaleX: tr.scaleX,
      scaleY: tr.scaleY,
      kind: 'text',
    };
  }
  if (clip.kind === 'sticker') {
    const sticker = clip.sticker || {};
    const fontSize = Math.max(8, (sticker.size ?? 220) * (height / 1080));
    const hasBackground = sticker.background && sticker.background !== 'transparent';
    const size = hasBackground ? fontSize * 1.45 : fontSize * 1.18;
    return {
      width: size,
      height: size,
      cx: width / 2 + ((tr.x + (sticker.x || 0)) / 100) * width,
      cy: height / 2 + ((tr.y + (sticker.y || 0)) / 100) * height,
      rotation: tr.rotate + (sticker.rotate || 0),
      scale: tr.scaleX,
      scaleX: tr.scaleX,
      scaleY: tr.scaleY,
      kind: 'sticker',
    };
  }
  return null;
}

export function adjustCss(adjust) {
  const a = adjust || {};
  const parts = [];
  if ((a.brightness ?? 100) !== 100) parts.push(`brightness(${a.brightness}%)`);
  if ((a.contrast ?? 100) !== 100) parts.push(`contrast(${a.contrast}%)`);
  if ((a.saturate ?? 100) !== 100) parts.push(`saturate(${a.saturate}%)`);
  if ((a.grayscale ?? 0) > 0) parts.push(`grayscale(${Math.min(100, a.grayscale)}%)`);
  if ((a.sepia ?? 0) > 0) parts.push(`sepia(${Math.min(100, a.sepia)}%)`);
  if ((a.hue ?? 0) !== 0) parts.push(`hue-rotate(${a.hue}deg)`);
  return parts.length ? parts.join(' ') : 'none';
}

export function clipFilter(clip) {
  const base = FILTERS[clip.filter]?.css || 'none';
  const parts = [];
  if (base !== 'none') parts.push(base);
  const adjusted = adjustCss(clip.adjust);
  if (adjusted !== 'none') parts.push(adjusted);
  const spec = effectSpec(clip);
  if (spec?.css && spec.css !== 'none') parts.push(spec.css);
  return parts.length ? parts.join(' ') : 'none';
}

function sourceTime(clip, local) {
  return Math.max(0, (clip.in || 0) + local * (clip.speed || 1));
}

export class MediaPool {
  constructor(mediaMap) {
    this.media = mediaMap;
    this.videos = new Map();
    this.audios = new Map();
    this.images = new Map();
    this.frames = new Map();
    this.audioContext = null;
    this.audioDest = null;
    this.muted = false;
    this.onReady = null;
  }

  mediaFor(mediaId) {
    return this.media?.get(mediaId) || null;
  }

  image(mediaId) {
    if (this.images.has(mediaId)) return this.images.get(mediaId);
    const media = this.mediaFor(mediaId);
    if (!media?.url) return null;
    const image = new Image();
    image.decoding = 'async';
    image.addEventListener('load', () => this.onReady?.());
    image.src = media.url;
    this.images.set(mediaId, image);
    return image;
  }

  video(clip) {
    if (this.videos.has(clip.id)) return this.videos.get(clip.id);
    const media = this.mediaFor(clip.mediaId);
    if (!media?.url) return null;
    const video = document.createElement('video');
    video.src = media.url;
    video.preload = 'auto';
    video.muted = true;
    video.playsInline = true;
    video.addEventListener('loadeddata', () => this.onReady?.());
    video.addEventListener('seeked', () => this.onReady?.());
    this.videos.set(clip.id, video);
    return video;
  }

  /* Guarda el último frame bueno de cada vídeo para que la vista previa
     no se ponga en negro mientras el navegador resuelve un seek. */
  captureFrame(clipId, video) {
    try {
      if (!video.videoWidth || !video.videoHeight) return;
      let snap = this.frames.get(clipId);
      if (!snap) {
        snap = document.createElement('canvas');
        this.frames.set(clipId, snap);
      }
      if (snap.width !== video.videoWidth || snap.height !== video.videoHeight) {
        snap.width = video.videoWidth;
        snap.height = video.videoHeight;
      }
      snap.getContext('2d').drawImage(video, 0, 0);
    } catch {
      /* frame aún no dibujable */
    }
  }

  lastFrame(clipId) {
    return this.frames.get(clipId) || null;
  }

  audio(clip) {
    if (this.audios.has(clip.id)) return this.audios.get(clip.id);
    const media = this.mediaFor(clip.mediaId);
    if (!media?.url) return null;
    const audio = document.createElement('audio');
    audio.src = media.url;
    audio.preload = 'auto';
    if (this.audioContext && this.audioDest) {
      try {
        const source = this.audioContext.createMediaElementSource(audio);
        source.connect(this.audioDest);
        audio.__source = source;
      } catch {
        /* el elemento ya estaba conectado */
      }
    }
    this.audios.set(clip.id, audio);
    return audio;
  }

  attachAudio(audioContext, destination) {
    this.audioContext = audioContext;
    this.audioDest = destination;
  }

  syncElement(element, target, playing, volume, muted) {
    if (!element) return;
    if (element.volume !== volume) element.volume = volume;
    if (element.muted !== muted) element.muted = muted;
    if (playing) {
      if (element.readyState === 0) {
        try { element.load(); } catch { /* ignorado */ }
      }
      if (Math.abs(element.currentTime - target) > 0.32) {
        try { element.currentTime = target; } catch { /* ignorado */ }
      }
      if (element.paused) element.play().catch(() => {});
    } else {
      if (!element.paused) element.pause();
      if (Math.abs(element.currentTime - target) > 0.04) {
        try { element.currentTime = target; } catch { /* ignorado */ }
      }
    }
  }

  syncPlayback(project, time, playing, muted = false) {
    const layers = activeLayers(project, time);
    const activeIds = new Set(layers.map((layer) => layer.clip.id));
    for (const layer of layers) {
      const { clip, track, local } = layer;
      const target = sourceTime(clip, local);
      const speed = Math.max(0.25, Math.min(4, clip.speed || 1));
      if (clip.kind === 'video') {
        const video = this.video(clip);
        if (video) {
          video.playbackRate = speed;
          this.syncElement(video, target, playing && time < clip.start + clip.duration, 0, true);
        }
      }
      if (clip.kind === 'video' || clip.kind === 'audio') {
        const audio = this.audio(clip);
        if (audio) {
          audio.playbackRate = speed;
          const gain = clipGain(clip, local) * (track.volume ?? 1);
          this.syncElement(audio, target, playing, gain, muted || !!track.muted);
        }
      }
    }
    for (const [id, element] of this.audios) {
      if (!activeIds.has(id) && !element.paused) element.pause();
    }
    for (const [id, element] of this.videos) {
      if (!activeIds.has(id) && !element.paused) element.pause();
    }
  }

  async prepare(project, time) {
    const layers = activeLayers(project, time);
    const jobs = [];
    for (const { clip, local } of layers) {
      if (clip.kind === 'image') {
        const image = this.image(clip.mediaId);
        if (image && !image.complete) jobs.push(waitEvent(image, 'load'));
      }
      if (clip.kind === 'video') {
        const video = this.video(clip);
        if (video) {
          jobs.push((async () => {
            if (video.readyState < 1) await waitEvent(video, 'loadedmetadata');
            if (video.readyState < 2) await waitEvent(video, 'loadeddata');
            const target = sourceTime(clip, local);
            if (Math.abs(video.currentTime - target) > 0.02) {
              const seeked = waitEvent(video, 'seeked', 5000);
              try { video.currentTime = target; } catch { /* ignorado */ }
              await seeked;
            }
          })());
        }
      }
    }
    await Promise.allSettled(jobs);
  }

  destroy() {
    for (const element of this.videos.values()) {
      element.pause();
      element.removeAttribute('src');
      try { element.load(); } catch { /* ignorado */ }
    }
    for (const element of this.audios.values()) {
      element.pause();
      element.removeAttribute('src');
      try { element.load(); } catch { /* ignorado */ }
    }
    this.videos.clear();
    this.audios.clear();
    this.images.clear();
    this.frames.clear();
    this.onReady = null;
  }
}

function roundedRect(ctx, x, y, width, height, radius) {
  const r = Math.min(radius, width / 2, height / 2);
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + width, y, x + width, y + height, r);
  ctx.arcTo(x + width, y + height, x, y + height, r);
  ctx.arcTo(x, y + height, x, y, r);
  ctx.arcTo(x, y, x + width, y, r);
  ctx.closePath();
}

function drawContain(ctx, source, sourceWidth, sourceHeight, width, height) {
  const scale = Math.min(width / sourceWidth, height / sourceHeight);
  const drawWidth = sourceWidth * scale;
  const drawHeight = sourceHeight * scale;
  ctx.drawImage(source, -drawWidth / 2, -drawHeight / 2, drawWidth, drawHeight);
}

function transitionState(clip, local) {
  if (!clip.transition || clip.transition === 'none') return null;
  const duration = Math.max(0.05, Math.min(Number(clip.transitionDuration) || 0.6, clip.duration * 0.62));
  if (local > duration + 0.0001) return null;
  return { type: clip.transition, p: clamp01(local / duration), duration };
}

function transitionAlpha(anim) {
  if (!anim) return 1;
  if (anim.type === 'flash') return clamp01(anim.p * 1.8);
  if (anim.type.startsWith('slide-') || anim.type.startsWith('wipe-')) return 1;
  if (anim.type === 'zoom-in' || anim.type === 'zoom-out' || anim.type === 'spin') return clamp01(0.25 + anim.p * 1.4);
  return anim.p;
}

function applyEntrance(ctx, anim, width, height) {
  if (!anim) return;
  const gap = 1 - anim.p;
  if (anim.type === 'slide-left') ctx.translate(width * gap, 0);
  else if (anim.type === 'slide-right') ctx.translate(-width * gap, 0);
  else if (anim.type === 'slide-up') ctx.translate(0, height * gap);
  else if (anim.type === 'slide-down') ctx.translate(0, -height * gap);
  else if (anim.type === 'zoom-in') ctx.scale(1 + gap * 0.34, 1 + gap * 0.34);
  else if (anim.type === 'zoom-out') ctx.scale(0.68 + anim.p * 0.32, 0.68 + anim.p * 0.32);
  else if (anim.type === 'spin') {
    ctx.rotate((gap * -14 * Math.PI) / 180);
    ctx.scale(0.9 + anim.p * 0.1, 0.9 + anim.p * 0.1);
  }
}

function wipeClip(ctx, anim, width, height) {
  if (!anim || !anim.type.startsWith('wipe-')) return false;
  const size = width * anim.p;
  ctx.beginPath();
  if (anim.type === 'wipe-right') ctx.rect(width / 2 - size, -height / 2, size, height);
  else ctx.rect(-width / 2, -height / 2, size, height);
  ctx.clip();
  return true;
}

function previousClipOnTrack(track, clip) {
  let best = null;
  for (const candidate of track.clips || []) {
    if (candidate.id === clip.id) continue;
    const end = candidate.start + candidate.duration;
    if (Math.abs(end - clip.start) <= 0.08) {
      if (!best || candidate.start > best.start) best = candidate;
    }
  }
  return best;
}

let grainTile = null;
function noiseTile() {
  if (grainTile) return grainTile;
  const canvas = document.createElement('canvas');
  canvas.width = 160;
  canvas.height = 160;
  const ctx = canvas.getContext('2d');
  const image = ctx.createImageData(canvas.width, canvas.height);
  for (let index = 0; index < image.data.length; index += 4) {
    const value = Math.random() * 255;
    image.data[index] = value;
    image.data[index + 1] = value;
    image.data[index + 2] = value;
    image.data[index + 3] = 26;
  }
  ctx.putImageData(image, 0, 0);
  grainTile = canvas;
  return canvas;
}

function hash01(value) {
  const x = Math.sin(value * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
}

const EMOJI_FONT = '"Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji", sans-serif';

const PARTICLE_GLYPHS = {
  heart: ['❤', '💖', '💕'],
  star: ['★', '✦', '✧'],
  spark: ['✦', '✧', '·'],
  bubble: ['○', '◯', '●'],
  dust: ['·'],
  emoji: ['✨'],
};

function particleGlyphs(spec) {
  if (Array.isArray(spec.glyphs) && spec.glyphs.length) return spec.glyphs;
  return PARTICLE_GLYPHS[spec.particles] || PARTICLE_GLYPHS.emoji;
}

function drawParticles(ctx, spec, local, width, height, alpha) {
  const kind = spec.particles;
  if (!kind) return;
  const count = Math.max(4, Math.min(80, spec.count || 24));
  const baseSize = Math.max(4, (spec.size || 30) * (height / 1080));
  const speed = spec.speed ?? 0.5;
  const colors = Array.isArray(spec.colors) && spec.colors.length ? spec.colors : ['#ffffff'];
  const glyphs = particleGlyphs(spec);
  const burst = kind === 'firework' || kind === 'explosion';
  const shapes = kind === 'confetti';
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let index = 0; index < count; index += 1) {
    const seed = hash01(index + 1);
    const seed2 = hash01(index + 17.3);
    const seed3 = hash01(index + 41.7);
    let x = 0;
    let y = 0;
    let life = 1;
    if (burst) {
      const cycle = 1.8;
      const phase = ((local * speed + seed) % cycle) / cycle;
      const angle = seed2 * Math.PI * 2;
      const radius = phase * Math.min(width, height) * (0.28 + seed3 * 0.34);
      x = width / 2 + Math.cos(angle) * radius;
      y = height / 2 + Math.sin(angle) * radius;
      life = 1 - phase;
    } else {
      const drift = (local * speed * 0.075 + seed2) % 1;
      const bubbles = kind === 'bubble';
      y = bubbles || spec.spawn === 'bottom' ? height * (1 - drift) : height * drift;
      x = seed * width + Math.sin(local * speed * 2.2 + index * 1.7) * width * 0.035;
      x = ((x % width) + width) % width;
      life = 0.55 + seed3 * 0.45;
    }
    const size = baseSize * (0.55 + seed3 * 0.75);
    if (x < -size || x > width + size || y < -size || y > height + size) continue;
    const color = colors[index % colors.length];
    if (shapes) {
      ctx.save();
      ctx.translate(x, y);
      ctx.rotate(local * (1.4 + seed) + seed2 * Math.PI);
      ctx.globalAlpha = alpha * life * 0.92;
      ctx.fillStyle = color;
      if (index % 3 === 0) {
        ctx.beginPath();
        ctx.arc(0, 0, size * 0.28, 0, Math.PI * 2);
        ctx.fill();
      } else {
        ctx.fillRect(-size * 0.24, -size * 0.5, size * 0.48, size * 0.9);
      }
      ctx.restore();
      continue;
    }
    if (kind === 'spark') {
      ctx.save();
      ctx.translate(x, y);
      ctx.rotate(seed * Math.PI + local * 1.8);
      ctx.globalAlpha = alpha * life;
      ctx.fillStyle = color;
      ctx.beginPath();
      const arm = size * 0.52;
      const thin = size * 0.1;
      ctx.moveTo(0, -arm);
      ctx.lineTo(thin, -thin);
      ctx.lineTo(arm, 0);
      ctx.lineTo(thin, thin);
      ctx.lineTo(0, arm);
      ctx.lineTo(-thin, thin);
      ctx.lineTo(-arm, 0);
      ctx.lineTo(-thin, -thin);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
      continue;
    }
    if (kind === 'bubble') {
      ctx.globalAlpha = alpha * life * 0.5;
      ctx.strokeStyle = color;
      ctx.lineWidth = Math.max(1, size * 0.06);
      ctx.beginPath();
      ctx.arc(x, y, size * 0.32, 0, Math.PI * 2);
      ctx.stroke();
      continue;
    }
    if (kind === 'dust') {
      ctx.globalAlpha = alpha * life * 0.7;
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(x, y, Math.max(0.7, size * 0.08), 0, Math.PI * 2);
      ctx.fill();
      continue;
    }
    const glyph = glyphs[index % glyphs.length];
    ctx.globalAlpha = alpha * life * 0.9;
    ctx.font = `${Math.max(6, size)}px ${EMOJI_FONT}`;
    ctx.fillStyle = color;
    ctx.fillText(glyph, x, y);
  }
  ctx.restore();
}

function drawFrameFx(ctx, spec, width, height, alpha) {
  const style = spec.frame;
  if (!style) return;
  const accent = spec.accent || '#ffffff';
  const edge = Math.max(6, Math.min(width, height) * 0.035);
  ctx.save();
  ctx.globalAlpha = alpha;
  if (style === 'polaroid') {
    const bottom = edge * 3.4;
    ctx.fillStyle = '#f7f2e6';
    ctx.fillRect(0, 0, width, edge);
    ctx.fillRect(0, height - bottom, width, bottom);
    ctx.fillRect(0, 0, edge, height);
    ctx.fillRect(width - edge, 0, edge, height);
    ctx.strokeStyle = 'rgba(0,0,0,0.18)';
    ctx.lineWidth = Math.max(1, edge * 0.08);
    ctx.strokeRect(edge * 0.35, edge * 0.35, width - edge * 0.7, height - edge * 0.7);
  } else if (style === 'film') {
    const strip = Math.max(10, height * 0.075);
    ctx.fillStyle = '#0b0b0d';
    ctx.fillRect(0, 0, width, strip);
    ctx.fillRect(0, height - strip, width, strip);
    const holeW = strip * 0.55;
    const holeH = strip * 0.32;
    ctx.fillStyle = 'rgba(245,242,226,0.9)';
    for (let x = strip * 0.35; x < width; x += holeW * 1.7) {
      roundedRect(ctx, x, strip * 0.28, holeW, holeH, holeH * 0.25);
      ctx.fill();
      roundedRect(ctx, x, height - strip * 0.6, holeW, holeH, holeH * 0.25);
      ctx.fill();
    }
  } else if (style === 'tape') {
    const tape = Math.min(width, height) * 0.13;
    ctx.fillStyle = 'rgba(255,238,166,0.72)';
    for (const corner of [[0, 0, -1, -1], [width, 0, 1, -1], [0, height, -1, 1], [width, height, 1, 1]]) {
      ctx.save();
      ctx.translate(corner[0], corner[1]);
      ctx.rotate(corner[2] * corner[3] * 0.75);
      ctx.fillRect(-tape * 0.14, -tape * 0.5, tape * 0.28, tape);
      ctx.restore();
    }
  } else if (style === 'neon') {
    ctx.strokeStyle = accent;
    ctx.lineWidth = Math.max(3, edge * 0.28);
    ctx.shadowColor = accent;
    ctx.shadowBlur = Math.max(10, edge * 1.4);
    roundedRect(ctx, edge * 0.7, edge * 0.7, width - edge * 1.4, height - edge * 1.4, edge * 1.2);
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.strokeStyle = 'rgba(255,255,255,0.7)';
    ctx.lineWidth = Math.max(1, edge * 0.08);
    ctx.stroke();
  } else if (style === 'floral') {
    const glyphs = ['🌸', '🌺', '🌼', '🍃'];
    for (let index = 0; index < 12; index += 1) {
      const side = index % 4;
      const t = (Math.floor(index / 4) + 0.5) / 3;
      const x = side === 0 || side === 2 ? t * width : side === 1 ? width - edge * 0.6 : edge * 0.6;
      const y = side === 0 ? edge * 0.6 : side === 2 ? height - edge * 0.6 : t * height;
      ctx.font = `${Math.max(12, edge * 0.95)}px ${EMOJI_FONT}`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(glyphs[index % glyphs.length], x, y);
    }
  } else if (style === 'torn') {
    ctx.fillStyle = '#f2ecdd';
    ctx.fillRect(0, 0, width, edge * 0.7);
    ctx.fillRect(0, height - edge * 0.7, width, edge * 0.7);
    ctx.fillRect(0, 0, edge * 0.7, height);
    ctx.fillRect(width - edge * 0.7, 0, edge * 0.7, height);
    ctx.fillStyle = 'rgba(0,0,0,0.18)';
    for (let index = 0; index < 22; index += 1) {
      const x = hash01(index + 4) * width;
      const h = edge * (0.15 + hash01(index + 9) * 0.5);
      ctx.beginPath();
      ctx.moveTo(x - h * 0.5, edge * 0.7);
      ctx.lineTo(x, edge * 0.7 + h);
      ctx.lineTo(x + h * 0.5, edge * 0.7);
      ctx.closePath();
      ctx.fill();
    }
  } else if (style === 'wood') {
    ctx.fillStyle = '#5a3823';
    ctx.fillRect(0, 0, width, edge);
    ctx.fillRect(0, height - edge, width, edge);
    ctx.fillRect(0, 0, edge, height);
    ctx.fillRect(width - edge, 0, edge, height);
    ctx.strokeStyle = 'rgba(35,18,8,0.55)';
    ctx.lineWidth = Math.max(1, edge * 0.08);
    for (let index = 0; index < 14; index += 1) {
      const y = (index / 13) * height;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.bezierCurveTo(width * 0.3, y + edge * 0.3, width * 0.7, y - edge * 0.3, width, y);
      ctx.stroke();
    }
  } else if (style === 'dots') {
    const step = Math.max(10, Math.min(width, height) * 0.055);
    ctx.fillStyle = accent;
    for (let x = step; x < width; x += step) {
      ctx.beginPath();
      ctx.arc(x, step * 0.7, step * 0.14, 0, Math.PI * 2);
      ctx.fill();
      ctx.beginPath();
      ctx.arc(x, height - step * 0.7, step * 0.14, 0, Math.PI * 2);
      ctx.fill();
    }
  } else if (style === 'rainbow') {
    const gradient = ctx.createLinearGradient(0, 0, width, height);
    ['#ff4d6d', '#ffd166', '#8ac926', '#4cc9f0', '#c77dff'].forEach((color, index, list) => {
      gradient.addColorStop(index / (list.length - 1), color);
    });
    ctx.strokeStyle = gradient;
    ctx.lineWidth = Math.max(6, edge * 0.5);
    ctx.strokeRect(edge * 0.5, edge * 0.5, width - edge, height - edge);
  } else if (style === 'web') {
    ctx.strokeStyle = 'rgba(235,240,250,0.72)';
    ctx.lineWidth = Math.max(1, edge * 0.08);
    for (const corner of [[0, 0, 0], [width, 0, Math.PI / 2], [0, height, -Math.PI / 2], [width, height, Math.PI]]) {
      for (let index = 0; index < 6; index += 1) {
        const angle = corner[2] + (index / 5) * (Math.PI / 2);
        ctx.beginPath();
        ctx.moveTo(corner[0], corner[1]);
        ctx.lineTo(corner[0] + Math.cos(angle) * edge * 4, corner[1] + Math.sin(angle) * edge * 4);
        ctx.stroke();
      }
      for (let radius = edge * 0.8; radius < edge * 3.4; radius += edge * 0.62) {
        ctx.beginPath();
        ctx.arc(corner[0], corner[1], radius, corner[2], corner[2] + Math.PI / 2);
        ctx.stroke();
      }
    }
  } else if (style === 'coupon') {
    ctx.strokeStyle = accent;
    ctx.lineWidth = Math.max(2, edge * 0.16);
    ctx.setLineDash([edge * 0.45, edge * 0.32]);
    roundedRect(ctx, edge * 0.8, edge * 0.8, width - edge * 1.6, height - edge * 1.6, edge * 0.9);
    ctx.stroke();
    ctx.setLineDash([]);
  } else if (style === 'comic') {
    ctx.strokeStyle = '#111111';
    ctx.lineWidth = Math.max(5, edge * 0.38);
    ctx.strokeRect(edge * 0.5, edge * 0.5, width - edge, height - edge);
    ctx.fillStyle = accent;
    ctx.beginPath();
    ctx.moveTo(width - edge * 3.4, 0);
    ctx.lineTo(width, 0);
    ctx.lineTo(width, edge * 3.4);
    ctx.closePath();
    ctx.fill();
  } else if (style === 'badge') {
    const radius = Math.max(16, edge * 1.35);
    ctx.fillStyle = accent;
    ctx.beginPath();
    ctx.arc(width - radius * 1.2, radius * 1.2, radius, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#071016';
    ctx.font = `800 ${Math.max(10, radius * 0.55)}px "Segoe UI", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('NUEVO', width - radius * 1.2, radius * 1.2);
  } else if (style === 'open') {
    const gradient = ctx.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, 'rgba(255,214,153,0.28)');
    gradient.addColorStop(0.45, 'rgba(255,255,255,0)');
    gradient.addColorStop(1, 'rgba(120,200,255,0.22)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
  }
  ctx.restore();
}

function drawOverlayFx(ctx, spec, local, width, height, alpha) {
  const overlay = spec.overlay;
  if (!overlay) return;
  const accent = spec.accent || '#ffffff';
  const unit = Math.min(width, height);
  ctx.save();
  ctx.globalAlpha = alpha;
  if (overlay === 'vignette' || overlay === 'silhouette' || overlay === 'slim' || overlay === 'shadow') {
    const gradient = ctx.createRadialGradient(width / 2, height / 2, unit * 0.2, width / 2, height / 2, Math.max(width, height) * 0.72);
    gradient.addColorStop(0, 'rgba(0,0,0,0)');
    gradient.addColorStop(1, overlay === 'slim' ? 'rgba(0,0,0,0.42)' : 'rgba(0,0,0,0.62)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
    if (overlay === 'slim') {
      ctx.fillStyle = 'rgba(0,0,0,0.3)';
      ctx.fillRect(0, 0, width * 0.09, height);
      ctx.fillRect(width * 0.91, 0, width * 0.09, height);
    }
  }
  if (overlay === 'grain' || overlay === 'oldFilm' || overlay === 'projector' || overlay === 'chalk' || overlay === 'print' || overlay === 'watercolor') {
    ctx.globalAlpha = alpha * 0.72;
    ctx.drawImage(noiseTile(), 0, 0, width, height);
    ctx.globalAlpha = alpha;
  }
  if (overlay === 'scanlines' || overlay === 'vhs' || overlay === 'vcr' || overlay === 'pixel' || overlay === 'thermal' || overlay === 'xray') {
    const step = Math.max(2, Math.round(height / 260));
    ctx.globalAlpha = alpha * 0.5;
    ctx.fillStyle = 'rgba(0,0,0,0.28)';
    for (let y = 0; y < height; y += step * 2) ctx.fillRect(0, y, width, step);
    ctx.globalAlpha = alpha;
  }
  if (overlay === 'oldFilm' || overlay === 'projector') {
    ctx.globalAlpha = alpha * (0.32 + Math.abs(Math.sin(local * 9)) * 0.12);
    ctx.strokeStyle = 'rgba(237,219,178,0.7)';
    ctx.lineWidth = Math.max(1, width / 900);
    for (let index = 0; index < 3; index += 1) {
      const x = ((local * 13 + index * 31.7) % 1) * width;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x + Math.sin(local * 2 + index) * 8, height);
      ctx.stroke();
    }
    if (overlay === 'projector') {
      ctx.fillStyle = 'rgba(25,15,6,0.18)';
      ctx.fillRect(0, 0, width, height);
    }
  }
  if (overlay === 'strobe' || overlay === 'flash') {
    const pulse = overlay === 'flash'
      ? Math.max(0, Math.sin(local * 8) - 0.72) * 2.8
      : (Math.sin(local * 18) > 0.55 ? 0.1 : 0);
    if (pulse > 0) {
      ctx.globalAlpha = alpha * Math.min(0.3, pulse);
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, width, height);
    }
  }
  if (overlay === 'glow' || overlay === 'dreamy' || overlay === 'aura' || overlay === 'bodyGlow' || overlay === 'magic' || overlay === 'fog') {
    const color = overlay === 'glow' || overlay === 'aura' || overlay === 'bodyGlow' ? accent : '#ffffff';
    const amount = overlay === 'dreamy' ? 0.035 : overlay === 'fog' ? 0.12 : 0.09;
    ctx.globalAlpha = alpha * amount;
    ctx.fillStyle = color;
    ctx.fillRect(0, 0, width, height);
    if (overlay === 'aura' || overlay === 'bodyGlow' || overlay === 'magic') {
      const gradient = ctx.createRadialGradient(width / 2, height / 2, unit * 0.08, width / 2, height / 2, unit * 0.58);
      gradient.addColorStop(0, `${accent}88`);
      gradient.addColorStop(1, 'rgba(0,0,0,0)');
      ctx.globalAlpha = alpha * 0.75;
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, height);
    }
  }
  if (overlay === 'chroma') {
    ctx.globalCompositeOperation = 'screen';
    const red = ctx.createLinearGradient(0, 0, width * 0.25, 0);
    red.addColorStop(0, 'rgba(255,0,60,0.28)');
    red.addColorStop(1, 'rgba(255,0,60,0)');
    ctx.fillStyle = red;
    ctx.fillRect(0, 0, width * 0.25, height);
    const cyan = ctx.createLinearGradient(width, 0, width * 0.75, 0);
    cyan.addColorStop(0, 'rgba(0,240,255,0.28)');
    cyan.addColorStop(1, 'rgba(0,240,255,0)');
    ctx.fillStyle = cyan;
    ctx.fillRect(width * 0.75, 0, width * 0.25, height);
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'vhs' || overlay === 'vcr' || overlay === 'tracking') {
    const bandY = ((local * 0.35) % 1.2) * height - height * 0.1;
    const band = ctx.createLinearGradient(0, bandY - height * 0.08, 0, bandY + height * 0.08);
    band.addColorStop(0, 'rgba(255,255,255,0)');
    band.addColorStop(0.5, 'rgba(255,255,255,0.16)');
    band.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = band;
    ctx.fillRect(0, bandY - height * 0.08, width, height * 0.16);
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    ctx.font = `600 ${Math.max(9, height * 0.032)}px "Consolas", monospace`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    const stamp = `PLAY ▶  ${String(Math.floor(local / 60)).padStart(2, '0')}:${String(Math.floor(local % 60)).padStart(2, '0')}:${String(Math.floor((local * 30) % 30)).padStart(2, '0')}`;
    ctx.fillText(stamp, width * 0.035, height * 0.04);
    if (overlay === 'vcr') {
      ctx.fillStyle = '#ff3b3b';
      ctx.beginPath();
      ctx.arc(width * 0.05, height * 0.9, Math.max(3, height * 0.012), 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = 'rgba(255,255,255,0.8)';
      ctx.fillText('REC', width * 0.065, height * 0.88);
    }
  }
  if (overlay === 'glitch' || overlay === 'datamosh' || overlay === 'tracking') {
    for (let index = 0; index < 10; index += 1) {
      const seed = hash01(index + Math.floor(local * 16));
      const y = seed * height;
      const h = Math.max(2, hash01(index + 3.3) * height * 0.045);
      const x = hash01(index + 8.8) * width * 0.28 - width * 0.14;
      ctx.globalAlpha = alpha * (0.1 + seed * 0.22);
      ctx.fillStyle = index % 3 === 0 ? '#ff2f5e' : index % 3 === 1 ? '#28e7ff' : '#ffffff';
      ctx.fillRect(x, y, width * (0.35 + seed * 0.5), h);
    }
    ctx.globalAlpha = alpha * 0.35;
    for (let index = 0; index < 4; index += 1) {
      const y = hash01(index + Math.floor(local * 8) + 2) * height;
      ctx.fillStyle = 'rgba(0,0,0,0.55)';
      ctx.fillRect(0, y, width, Math.max(1, height * 0.004));
    }
  }
  if (overlay === 'pixel') {
    const cols = 24;
    const rows = 14;
    ctx.globalAlpha = alpha * 0.18;
    ctx.fillStyle = 'rgba(255,255,255,0.5)';
    for (let x = 0; x < cols; x += 1) {
      for (let y = 0; y < rows; y += 1) {
        if (hash01(x * 17 + y * 31) > 0.82) ctx.fillRect((x / cols) * width, (y / rows) * height, width / cols * 0.8, height / rows * 0.8);
      }
    }
  }
  if (overlay === 'lightLeak') {
    const x = width * (0.75 + Math.sin(local * 0.6) * 0.18);
    const gradient = ctx.createRadialGradient(x, height * 0.08, 0, x, height * 0.08, unit * 0.95);
    gradient.addColorStop(0, 'rgba(255,213,128,0.65)');
    gradient.addColorStop(0.35, 'rgba(255,114,94,0.32)');
    gradient.addColorStop(1, 'rgba(255,60,90,0)');
    ctx.globalCompositeOperation = 'screen';
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'flare') {
    const x = width * (0.68 + Math.sin(local * 0.4) * 0.1);
    const y = height * (0.28 + Math.cos(local * 0.5) * 0.08);
    const gradient = ctx.createRadialGradient(x, y, 0, x, y, unit * 0.55);
    gradient.addColorStop(0, 'rgba(255,255,255,0.75)');
    gradient.addColorStop(0.15, 'rgba(255,238,180,0.35)');
    gradient.addColorStop(1, 'rgba(255,220,140,0)');
    ctx.globalCompositeOperation = 'screen';
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = 'rgba(255,245,210,0.6)';
    ctx.lineWidth = Math.max(1, unit * 0.003);
    for (let index = 0; index < 6; index += 1) {
      const angle = (index / 6) * Math.PI * 2 + local * 0.1;
      ctx.beginPath();
      ctx.moveTo(x + Math.cos(angle) * unit * 0.08, y + Math.sin(angle) * unit * 0.08);
      ctx.lineTo(x + Math.cos(angle) * unit * 0.46, y + Math.sin(angle) * unit * 0.46);
      ctx.stroke();
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'spotlight') {
    const x = width * (0.5 + Math.sin(local * 0.55) * 0.26);
    const y = height * (0.5 + Math.cos(local * 0.43) * 0.2);
    const gradient = ctx.createRadialGradient(x, y, unit * 0.05, x, y, unit * 0.62);
    gradient.addColorStop(0, 'rgba(0,0,0,0)');
    gradient.addColorStop(0.55, 'rgba(0,0,0,0.25)');
    gradient.addColorStop(1, 'rgba(0,0,0,0.78)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
  }
  if (overlay === 'bokeh') {
    ctx.globalCompositeOperation = 'screen';
    for (let index = 0; index < 22; index += 1) {
      const seed = hash01(index + 3);
      const x = ((seed * width + Math.sin(local * 0.3 + index) * 30) % width + width) % width;
      const y = ((hash01(index + 9) * height + Math.cos(local * 0.24 + index) * 24) % height + height) % height;
      const r = unit * (0.015 + hash01(index + 14) * 0.05);
      ctx.globalAlpha = alpha * (0.08 + hash01(index + 21) * 0.16);
      ctx.fillStyle = index % 2 ? 'rgba(255,235,180,1)' : 'rgba(180,235,255,1)';
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'rays') {
    ctx.globalCompositeOperation = 'screen';
    ctx.translate(width * 0.5, -height * 0.12);
    ctx.rotate(Math.sin(local * 0.25) * 0.12);
    ctx.fillStyle = 'rgba(255,238,180,0.16)';
    for (let index = 0; index < 9; index += 1) {
      const angle = (-Math.PI / 2) + (index - 4) * 0.17;
      const spread = 0.035;
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(Math.cos(angle - spread) * height * 1.25, Math.sin(angle - spread) * height * 1.25);
      ctx.lineTo(Math.cos(angle + spread) * height * 1.25, Math.sin(angle + spread) * height * 1.25);
      ctx.closePath();
      ctx.fill();
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'aurora') {
    ctx.globalCompositeOperation = 'screen';
    for (let band = 0; band < 3; band += 1) {
      const gradient = ctx.createLinearGradient(0, height * 0.2, width, height * 0.6);
      gradient.addColorStop(0, `rgba(${60 + band * 40},${220 - band * 30},${200 + band * 20},0)`);
      gradient.addColorStop(0.5, `rgba(${80 + band * 30},${230 - band * 20},${180 + band * 30},${0.18 - band * 0.04})`);
      gradient.addColorStop(1, 'rgba(80,180,255,0)');
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.moveTo(0, height * (0.28 + band * 0.09));
      for (let x = 0; x <= width; x += width / 12) {
        ctx.lineTo(x, height * (0.28 + band * 0.09) + Math.sin(x / width * 6 + local * 0.8 + band) * height * 0.07);
      }
      ctx.lineTo(width, height);
      ctx.lineTo(0, height);
      ctx.closePath();
      ctx.fill();
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'matrix') {
    const fontSize = Math.max(10, height * 0.032);
    const columns = 18;
    ctx.font = `600 ${fontSize}px "Consolas", monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    for (let column = 0; column < columns; column += 1) {
      const x = (column + 0.5) * (width / columns);
      const offset = (local * (0.35 + hash01(column + 2) * 0.3) + hash01(column + 5)) % 1;
      const glyphs = '01アイウエオカキクケコサシスセソ';
      for (let line = 0; line < 9; line += 1) {
        const y = ((offset + line / 9) % 1) * height;
        ctx.globalAlpha = alpha * (0.12 + (line / 9) * 0.3);
        ctx.fillStyle = line === 0 ? '#c8ffd4' : '#3dff7a';
        ctx.fillText(glyphs[(column + line) % glyphs.length], x, y);
      }
    }
  }
  if (overlay === 'halftone' || overlay === 'comic' || overlay === 'boom') {
    const step = Math.max(5, unit * 0.018);
    ctx.fillStyle = 'rgba(0,0,0,0.16)';
    for (let y = step; y < height; y += step * 1.5) {
      for (let x = step; x < width; x += step * 1.5) {
        ctx.beginPath();
        ctx.arc(x, y, step * 0.18, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    if (overlay === 'boom') {
      ctx.fillStyle = '#ffd166';
      ctx.strokeStyle = '#111111';
      ctx.lineWidth = Math.max(3, unit * 0.008);
      ctx.save();
      ctx.translate(width * 0.62, height * 0.36);
      ctx.rotate(-0.12);
      ctx.beginPath();
      for (let index = 0; index < 14; index += 1) {
        const angle = (index / 14) * Math.PI * 2;
        const radius = unit * (index % 2 ? 0.12 : 0.19);
        ctx.lineTo(Math.cos(angle) * radius, Math.sin(angle) * radius);
      }
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = '#111111';
      ctx.font = `900 ${Math.max(14, unit * 0.055)}px "Arial Black", sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('BOOM!', 0, 0);
      ctx.restore();
    }
  }
  if (overlay === 'thermal') {
    ctx.globalCompositeOperation = 'screen';
    const gradient = ctx.createLinearGradient(0, height, width, 0);
    gradient.addColorStop(0, 'rgba(20,20,255,0.42)');
    gradient.addColorStop(0.5, 'rgba(255,190,0,0.28)');
    gradient.addColorStop(1, 'rgba(255,20,20,0.38)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'xray') {
    ctx.globalCompositeOperation = 'screen';
    ctx.fillStyle = 'rgba(110,210,255,0.22)';
    ctx.fillRect(0, 0, width, height);
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'neon' || overlay === 'neonOutline' || overlay === 'outline' || overlay === 'doubleBorder' || overlay === 'extrude' || overlay === 'depth') {
    ctx.strokeStyle = overlay === 'neon' ? accent : 'rgba(255,255,255,0.8)';
    ctx.lineWidth = Math.max(2, unit * 0.006);
    if (overlay === 'neon' || overlay === 'neonOutline') {
      ctx.shadowColor = accent;
      ctx.shadowBlur = unit * 0.035;
    }
    roundedRect(ctx, unit * 0.04, unit * 0.04, width - unit * 0.08, height - unit * 0.08, unit * 0.03);
    ctx.stroke();
    if (overlay === 'doubleBorder') {
      ctx.shadowBlur = 0;
      ctx.strokeStyle = '#ffd166';
      roundedRect(ctx, unit * 0.075, unit * 0.075, width - unit * 0.15, height - unit * 0.15, unit * 0.02);
      ctx.stroke();
    }
    ctx.shadowBlur = 0;
  }
  if (overlay === 'disco' || overlay === 'party') {
    ctx.globalCompositeOperation = 'screen';
    for (let index = 0; index < 6; index += 1) {
      const angle = local * 0.5 + (index / 6) * Math.PI * 2;
      const x = width * 0.5 + Math.cos(angle) * width * 0.4;
      const y = height * 0.5 + Math.sin(angle) * height * 0.4;
      const gradient = ctx.createRadialGradient(x, y, 0, x, y, unit * 0.35);
      const colors = ['255,80,140', '80,220,255', '255,210,80', '150,100,255'];
      gradient.addColorStop(0, `rgba(${colors[index % 4]},0.22)`);
      gradient.addColorStop(1, `rgba(${colors[index % 4]},0)`);
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, height);
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'sale') {
    ctx.save();
    ctx.translate(width * 0.5, height * 0.18);
    ctx.rotate(-0.06);
    ctx.fillStyle = accent;
    ctx.fillRect(-width * 0.18, -height * 0.045, width * 0.36, height * 0.09);
    ctx.fillStyle = '#111111';
    ctx.font = `900 ${Math.max(12, height * 0.045)}px "Arial Black", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('OFERTA', 0, 0);
    ctx.restore();
  }
  if (overlay === 'action') {
    ctx.strokeStyle = 'rgba(255,255,255,0.55)';
    ctx.lineWidth = Math.max(1, unit * 0.003);
    for (let index = 0; index < 18; index += 1) {
      const y = hash01(index + 2) * height;
      const x = ((local * 900 + index * 91) % (width + 300)) - 150;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x - width * 0.16, y);
      ctx.stroke();
    }
  }
  if (overlay === 'mosaic') {
    const cols = 14;
    const rows = 8;
    ctx.globalAlpha = alpha * 0.16;
    for (let x = 0; x < cols; x += 1) {
      for (let y = 0; y < rows; y += 2) {
        ctx.fillStyle = (x + y) % 3 === 0 ? '#ffffff' : '#000000';
        ctx.fillRect((x / cols) * width, (y / rows) * height, width / cols, height / rows);
      }
    }
  }
  if (overlay === 'doodle') {
    ctx.strokeStyle = 'rgba(255,255,255,0.7)';
    ctx.lineWidth = Math.max(1.5, unit * 0.004);
    for (let index = 0; index < 8; index += 1) {
      const x = hash01(index + 3) * width;
      const y = hash01(index + 11) * height;
      ctx.beginPath();
      for (let step = 0; step < 5; step += 1) {
        ctx.lineTo(x + Math.sin(local * 0.8 + step + index) * unit * 0.05 + step * unit * 0.02, y + Math.cos(step * 2 + index) * unit * 0.03);
      }
      ctx.stroke();
    }
  }
  if (overlay === 'countdown') {
    const value = Math.max(1, Math.ceil(3 - (local % 4)));
    ctx.fillStyle = 'rgba(255,255,255,0.9)';
    ctx.font = `900 ${Math.max(36, unit * 0.22)}px "Arial Black", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(value), width / 2, height / 2);
  }
  if (overlay === 'curtain') {
    const amount = Math.max(0, Math.sin(local * 1.2)) * width * 0.18;
    const gradient = ctx.createLinearGradient(0, 0, amount + unit * 0.08, 0);
    gradient.addColorStop(0, 'rgba(90,0,20,0.9)');
    gradient.addColorStop(1, 'rgba(90,0,20,0)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, amount + unit * 0.08, height);
    ctx.save();
    ctx.translate(width, 0);
    ctx.scale(-1, 1);
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, amount + unit * 0.08, height);
    ctx.restore();
  }
  if (overlay === 'wipe') {
    const x = ((local * 0.8) % 1) * width;
    const gradient = ctx.createLinearGradient(x - width * 0.08, 0, x + width * 0.08, 0);
    gradient.addColorStop(0, 'rgba(255,255,255,0)');
    gradient.addColorStop(0.5, 'rgba(255,255,255,0.65)');
    gradient.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = gradient;
    ctx.fillRect(x - width * 0.08, 0, width * 0.16, height);
  }
  if (overlay === 'close') {
    const bar = height * 0.08;
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, width, bar);
    ctx.fillRect(0, height - bar, width, bar);
  }
  if (overlay === 'impact') {
    ctx.strokeStyle = accent;
    ctx.lineWidth = Math.max(2, unit * 0.006);
    for (let index = 0; index < 20; index += 1) {
      const angle = (index / 20) * Math.PI * 2;
      ctx.beginPath();
      ctx.moveTo(width / 2 + Math.cos(angle) * unit * 0.12, height / 2 + Math.sin(angle) * unit * 0.12);
      ctx.lineTo(width / 2 + Math.cos(angle) * unit * (0.2 + hash01(index + 2) * 0.22), height / 2 + Math.sin(angle) * unit * (0.2 + hash01(index + 2) * 0.22));
      ctx.stroke();
    }
  }
  if (overlay === 'fire') {
    const gradient = ctx.createLinearGradient(0, height, 0, height * 0.55);
    gradient.addColorStop(0, 'rgba(255,90,0,0.5)');
    gradient.addColorStop(1, 'rgba(255,180,0,0)');
    ctx.globalCompositeOperation = 'screen';
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'fog') {
    for (let index = 0; index < 5; index += 1) {
      const x = ((local * (12 + index * 3) + index * 300) % (width + 400)) - 200;
      const gradient = ctx.createRadialGradient(x, height * (0.55 + index * 0.06), 0, x, height * (0.55 + index * 0.06), unit * 0.32);
      gradient.addColorStop(0, 'rgba(220,230,240,0.14)');
      gradient.addColorStop(1, 'rgba(220,230,240,0)');
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, height);
    }
  }
  if (overlay === 'lightning') {
    if (Math.sin(local * 7) > 0.86) {
      ctx.strokeStyle = 'rgba(235,245,255,0.9)';
      ctx.lineWidth = Math.max(2, unit * 0.005);
      ctx.beginPath();
      let x = width * 0.58;
      ctx.moveTo(x, 0);
      for (let y = 0; y < height; y += height * 0.08) {
        x += (hash01(y + Math.floor(local * 10)) - 0.5) * unit * 0.08;
        ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
  }
  if (overlay === 'eyes') {
    ctx.fillStyle = 'rgba(0,0,0,0.55)';
    ctx.fillRect(0, 0, width, height);
    const glow = Math.max(0.25, Math.abs(Math.sin(local * 1.4)));
    ctx.shadowColor = '#ff3b3b';
    ctx.shadowBlur = unit * 0.04;
    ctx.fillStyle = `rgba(255,60,60,${glow})`;
    for (const offset of [-1, 1]) {
      ctx.beginPath();
      ctx.ellipse(width / 2 + offset * unit * 0.07, height * 0.46, unit * 0.022, unit * 0.014, 0, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.shadowBlur = 0;
  }
  if (overlay === 'blood') {
    ctx.fillStyle = 'rgba(150,0,18,0.55)';
    for (let index = 0; index < 9; index += 1) {
      const x = hash01(index + 4) * width;
      const length = height * (0.08 + hash01(index + 8) * 0.3);
      const w = unit * (0.006 + hash01(index + 12) * 0.018);
      ctx.beginPath();
      ctx.moveTo(x - w, 0);
      ctx.lineTo(x + w, 0);
      ctx.lineTo(x + w * 0.6, length);
      ctx.arc(x, length, w * 0.6, 0, Math.PI);
      ctx.closePath();
      ctx.fill();
    }
  }
  if (overlay === 'ripple') {
    ctx.strokeStyle = accent;
    ctx.lineWidth = Math.max(1, unit * 0.003);
    for (let index = 0; index < 5; index += 1) {
      const p = ((local * 0.35 + index / 5) % 1);
      ctx.globalAlpha = alpha * (1 - p) * 0.55;
      ctx.beginPath();
      ctx.arc(width / 2, height / 2, p * unit * 0.7, 0, Math.PI * 2);
      ctx.stroke();
    }
  }
  if (overlay === 'breeze') {
    ctx.strokeStyle = 'rgba(255,255,255,0.5)';
    ctx.lineWidth = Math.max(1, unit * 0.003);
    for (let index = 0; index < 7; index += 1) {
      const y = hash01(index + 6) * height;
      const x = ((local * 220 + index * 160) % (width + 300)) - 150;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.bezierCurveTo(x + 60, y - 30, x + 120, y + 30, x + 180, y);
      ctx.stroke();
    }
  }
  if (overlay === 'gold') {
    const gradient = ctx.createLinearGradient(0, 0, width, height);
    gradient.addColorStop(0, 'rgba(255,205,110,0.3)');
    gradient.addColorStop(0.5, 'rgba(255,240,190,0.12)');
    gradient.addColorStop(1, 'rgba(180,120,20,0.22)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
  }
  if (overlay === 'print') {
    ctx.fillStyle = 'rgba(245,235,214,0.1)';
    ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = 'rgba(60,45,30,0.3)';
    ctx.lineWidth = Math.max(1, unit * 0.002);
    for (let index = 0; index < 40; index += 1) {
      const y = (index / 40) * height;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y + (hash01(index + 3) - 0.5) * 6);
      ctx.stroke();
    }
  }
  if (overlay === 'glass') {
    ctx.globalCompositeOperation = 'screen';
    for (let index = 0; index < 4; index += 1) {
      const x = ((index * 0.3 + local * 0.04) % 1) * width;
      const gradient = ctx.createLinearGradient(x - width * 0.04, 0, x + width * 0.04, height);
      gradient.addColorStop(0, 'rgba(255,255,255,0)');
      gradient.addColorStop(0.5, 'rgba(255,255,255,0.16)');
      gradient.addColorStop(1, 'rgba(255,255,255,0)');
      ctx.fillStyle = gradient;
      ctx.fillRect(x - width * 0.04, 0, width * 0.08, height);
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'hologram') {
    ctx.globalCompositeOperation = 'screen';
    for (let index = 0; index < 7; index += 1) {
      const y = (index / 7) * height + ((local * 40) % (height / 7));
      const gradient = ctx.createLinearGradient(0, y, width, y);
      gradient.addColorStop(0, 'rgba(255,80,180,0.12)');
      gradient.addColorStop(0.5, 'rgba(80,255,230,0.1)');
      gradient.addColorStop(1, 'rgba(120,120,255,0.12)');
      ctx.fillStyle = gradient;
      ctx.fillRect(0, y, width, height / 28);
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'prism') {
    ctx.globalCompositeOperation = 'screen';
    for (let index = 0; index < 5; index += 1) {
      ctx.fillStyle = ['rgba(255,60,120,0.1)', 'rgba(255,200,60,0.1)', 'rgba(60,255,180,0.1)', 'rgba(60,180,255,0.1)', 'rgba(180,80,255,0.1)'][index];
      ctx.beginPath();
      ctx.moveTo(width * (0.1 + index * 0.16), height);
      ctx.lineTo(width * (0.45 + index * 0.1), 0);
      ctx.lineTo(width * (0.62 + index * 0.08), 0);
      ctx.closePath();
      ctx.fill();
    }
    ctx.globalCompositeOperation = 'source-over';
  }
  if (overlay === 'pulse') {
    const p = (local * 1.3) % 1;
    ctx.strokeStyle = accent;
    ctx.globalAlpha = alpha * (1 - p) * 0.6;
    ctx.lineWidth = Math.max(2, unit * 0.01);
    ctx.beginPath();
    ctx.arc(width / 2, height / 2, unit * (0.18 + p * 0.42), 0, Math.PI * 2);
    ctx.stroke();
  }
  if (overlay === 'magic') {
    ctx.globalCompositeOperation = 'screen';
    const gradient = ctx.createRadialGradient(width / 2, height / 2, 0, width / 2, height / 2, unit * 0.7);
    gradient.addColorStop(0, 'rgba(180,120,255,0.14)');
    gradient.addColorStop(1, 'rgba(80,180,255,0)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
    ctx.globalCompositeOperation = 'source-over';
  }
  ctx.restore();
}

function drawEffectOverlay(ctx, clip, local, width, height, alpha) {
  const spec = effectSpec(clip);
  if (!spec || alpha <= 0.001) return;
  ctx.save();
  ctx.globalAlpha = alpha;
  drawOverlayFx(ctx, spec, local, width, height, alpha);
  drawFrameFx(ctx, spec, width, height, alpha);
  drawParticles(ctx, spec, local, width, height, alpha);
  ctx.restore();
}

function drawMediaSource(ctx, clip, media, width, height, pool) {
  if (media.kind === 'video') {
    const video = pool.video(clip);
    const ready = Boolean(video && video.readyState >= 2 && video.videoWidth);
    // Mientras el navegador resuelve un seek el elemento puede devolver un
    // fotograma en negro: se dibuja la última imagen buena guardada.
    if (ready && !video.seeking) {
      pool.captureFrame(clip.id, video);
      drawContain(ctx, video, video.videoWidth, video.videoHeight, width, height);
      return true;
    }
    const snapshot = pool.lastFrame(clip.id);
    if (snapshot && snapshot.width) {
      drawContain(ctx, snapshot, snapshot.width, snapshot.height, width, height);
      return true;
    }
    if (ready) {
      drawContain(ctx, video, video.videoWidth, video.videoHeight, width, height);
      return true;
    }
  } else if (media.kind === 'image') {
    const image = pool.image(clip.mediaId);
    if (image && image.complete && image.naturalWidth) {
      drawContain(ctx, image, image.naturalWidth, image.naturalHeight, width, height);
      return true;
    }
  }
  return false;
}

function effectMotion(spec, local, width, height) {
  const motion = { tx: 0, ty: 0, rot: 0, scale: 1, sx: 1, sy: 1 };
  if (!spec?.move) return motion;
  const speed = spec.speed || 1;
  const amount = spec.amount || 1;
  switch (spec.move) {
    case 'zoom':
      motion.scale = 1 + Math.sin(local * 6.2 * speed) * 0.06;
      break;
    case 'heartbeat': {
      const beat = Math.max(0, Math.sin(local * 7.4 * speed));
      motion.scale = 1 + beat * beat * beat * 0.1;
      break;
    }
    case 'shake':
      motion.tx = Math.sin(local * 43) * width * 0.006 * amount;
      motion.ty = Math.cos(local * 37) * height * 0.006 * amount;
      break;
    case 'wave':
      motion.tx = Math.sin(local * 4.1 * speed) * width * 0.028;
      motion.ty = Math.cos(local * 4.6 * speed) * height * 0.018;
      motion.rot = Math.sin(local * 4.1 * speed) * 0.012;
      break;
    case 'spin':
      motion.rot = local * 0.55 * speed;
      break;
    case 'drift':
      motion.tx = Math.sin(local * 2.1 * speed) * width * 0.03;
      motion.ty = Math.cos(local * 1.7 * speed) * height * 0.02;
      break;
    case 'swing':
      motion.rot = Math.sin(local * 3.2 * speed) * 0.04;
      motion.tx = Math.sin(local * 3.2 * speed) * width * 0.01;
      break;
    case 'float':
      motion.ty = Math.sin(local * 2.7 * speed) * height * 0.02;
      motion.tx = Math.cos(local * 2.1 * speed) * width * 0.01;
      break;
    case 'pan':
      motion.tx = Math.sin(local * 1.8 * speed) * width * 0.04;
      break;
    case 'tilt':
      motion.ty = Math.sin(local * 1.8 * speed) * height * 0.04;
      break;
    case 'roll':
      motion.rot = Math.sin(local * 2.2 * speed) * 0.055;
      break;
    case 'orbit':
      motion.tx = Math.sin(local * 3.1 * speed) * width * 0.022;
      motion.ty = Math.cos(local * 3.1 * speed) * height * 0.022;
      break;
    case 'jelly':
      motion.sx = 1 + Math.sin(local * 6.4 * speed) * 0.035;
      motion.sy = 1 - Math.sin(local * 6.4 * speed) * 0.035;
      break;
    case 'rebound':
      motion.ty = -Math.abs(Math.sin(local * 5.2 * speed)) * height * 0.04;
      break;
    case 'parallax':
      motion.tx = Math.sin(local * 1.2 * speed) * width * 0.03;
      motion.ty = Math.cos(local * 1.2 * speed) * height * 0.016;
      motion.scale = 1.04;
      break;
    case 'flip':
      motion.sx = Math.max(0.12, Math.abs(Math.cos(local * 1.7 * speed)));
      break;
    case 'cube':
      motion.sx = Math.max(0.2, Math.abs(Math.cos(local * 1.9 * speed)));
      motion.tx = Math.sin(local * 1.9 * speed) * width * 0.035;
      break;
    case 'tilt3d':
      motion.sy = 0.84 + Math.sin(local * 2.1 * speed) * 0.08;
      motion.rot = Math.sin(local * 2.1 * speed) * 0.02;
      break;
    case 'zoomIn':
      motion.scale = 1 + Math.max(0, 1 - local / 1.4) * 0.32;
      break;
    case 'impact':
      motion.scale = 1 + Math.max(0, Math.sin(local * 9.4)) * 0.06;
      motion.tx = Math.sin(local * 51) * width * 0.003;
      motion.ty = Math.cos(local * 47) * height * 0.003;
      break;
    case 'flap':
      motion.sy = 1 + Math.sin(local * 8.2 * speed) * 0.035;
      motion.ty = Math.sin(local * 8.2 * speed) * height * 0.006;
      break;
    case 'glitch':
      motion.tx = (hash01(Math.floor(local * 18)) - 0.5) * width * 0.022;
      break;
    case 'cloneGhost':
      break;
    default:
      break;
  }
  return motion;
}

function drawMediaLayer(ctx, layer, width, height, pool, options = {}) {
  const { clip, local } = layer;
  const media = clip.mediaId ? pool.mediaFor(clip.mediaId) : null;
  if (!media) return;
  const time = (clip.start || 0) + local;
  const tr = transformAt(clip, time);
  const anim = options.ignoreTransition ? null : transitionState(clip, local);
  const alpha = (options.ignoreFades ? tr.opacity : clipAlpha(clip, local)) * transitionAlpha(anim);
  if (alpha <= 0.002) return;
  const filter = clipFilter(clip);
  const spec = effectSpec(clip);
  const motion = effectMotion(spec, local, width, height);
  ctx.save();
  ctx.globalAlpha = alpha;
  if (filter !== 'none') ctx.filter = filter;
  if (anim?.type === 'blur') ctx.filter = `${filter === 'none' ? '' : `${filter} `}blur(${(1 - anim.p) * 10}px)`;
  ctx.translate(width / 2 + (tr.x / 100) * width, height / 2 + (tr.y / 100) * height);
  if (tr.rotate) ctx.rotate((tr.rotate * Math.PI) / 180);
  applyEntrance(ctx, anim, width, height);
  ctx.translate(motion.tx, motion.ty);
  if (motion.rot) ctx.rotate(motion.rot);
  ctx.scale(tr.scaleX * motion.scale * motion.sx, tr.scaleY * motion.scale * motion.sy);
  wipeClip(ctx, anim, width, height);
  if (spec?.mirror) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(-width / 2, -height / 2, width / 2, height);
    ctx.clip();
    drawMediaSource(ctx, clip, media, width, height, pool);
    ctx.restore();
    ctx.save();
    ctx.scale(-1, 1);
    ctx.beginPath();
    ctx.rect(-width / 2, -height / 2, width / 2, height);
    ctx.clip();
    drawMediaSource(ctx, clip, media, width, height, pool);
    ctx.restore();
  } else {
    drawMediaSource(ctx, clip, media, width, height, pool);
    const ghosts = spec?.ghost || (spec?.move === 'cloneGhost' ? 2 : 0);
    for (let index = 0; index < ghosts; index += 1) {
      ctx.save();
      ctx.globalAlpha = alpha * (0.16 + index * 0.04);
      if (spec.move === 'cloneGhost') {
        const direction = index % 2 === 0 ? -1 : 1;
        ctx.translate(direction * width * (0.16 + index * 0.04), 0);
        ctx.scale(0.92, 0.92);
      } else {
        ctx.translate(width * 0.04 * (index + 1), height * 0.006 * (index + 1));
      }
      drawMediaSource(ctx, clip, media, width, height, pool);
      ctx.restore();
    }
  }
  ctx.restore();
  drawEffectOverlay(ctx, clip, local, width, height, alpha);
}

const stickerImageCache = new Map();

function stickerImage(src, fallback = null) {
  const key = src + '|' + (fallback || '');
  let img = stickerImageCache.get(key);
  if (!img) {
    img = new Image();
    img.onload = () => window.dispatchEvent(new CustomEvent('opencup:glfx'));
    img.onerror = () => {
      if (fallback && !img.dataset.retried) {
        img.dataset.retried = '1';
        img.src = `/api/img?u=${encodeURIComponent(fallback)}`;
        return;
      }
      console.warn('[cupcraft] no se pudo cargar el sticker', src);
    };
    img.src = src;
    stickerImageCache.set(key, img);
  }
  return img;
}

function drawStickerLayer(ctx, layer, width, height, options = {}) {
  const { clip, local } = layer;
  const sticker = clip.sticker || {};
  const time = (clip.start || 0) + local;
  const tr = transformAt(clip, time);
  const anim = options.ignoreTransition ? null : transitionState(clip, local);
  const alpha = (options.ignoreFades ? tr.opacity : clipAlpha(clip, local)) * transitionAlpha(anim);
  if (alpha <= 0.002) return;
  const fontSize = Math.max(8, (sticker.size ?? 220) * (height / 1080));
  const offsetX = (tr.x + (sticker.x || 0)) / 100 * width;
  const offsetY = (tr.y + (sticker.y || 0)) / 100 * height;
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.translate(width / 2 + offsetX, height / 2 + offsetY);
  ctx.rotate(((tr.rotate + (sticker.rotate || 0)) * Math.PI) / 180);
  applyEntrance(ctx, anim, width, height);
  let sx = tr.scaleX;
  let sy = tr.scaleY;
  if (clip.effect === 'zoomPulse') {
    const pulse = 1 + Math.sin(local * 6.2) * 0.055;
    sx *= pulse;
    sy *= pulse;
  }
  ctx.scale(sx, sy);
  wipeClip(ctx, anim, width, height);
  if (sticker.src) {
    const img = stickerImage(sticker.src, sticker.srcFallback);
    if (img.complete && img.naturalWidth) {
      const box = fontSize * 2.4;
      ctx.drawImage(img, -box / 2, -box / 2, box, box);
    } else {
      ctx.font = `${Math.max(10, fontSize * 0.35)}px sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillStyle = 'rgba(255,255,255,0.45)';
      ctx.fillText('cargando…', 0, 0);
    }
    ctx.restore();
    return;
  }
  const content = String(sticker.content || '✨');
  const family = '"Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji", sans-serif';
  ctx.font = `${fontSize}px ${family}`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  if (sticker.background && sticker.background !== 'transparent') {
    const radius = fontSize * 0.68;
    ctx.fillStyle = sticker.background;
    ctx.beginPath();
    if (sticker.shape === 'square') {
      roundedRect(ctx, -radius, -radius, radius * 2, radius * 2, radius * 0.28);
    } else {
      ctx.arc(0, 0, radius, 0, Math.PI * 2);
    }
    ctx.fill();
  }
  if (sticker.shadow !== false) {
    ctx.shadowColor = 'rgba(0,0,0,0.4)';
    ctx.shadowBlur = fontSize * 0.08;
    ctx.shadowOffsetY = fontSize * 0.03;
  }
  ctx.fillStyle = sticker.color || '#ffffff';
  ctx.fillText(content, 0, fontSize * 0.04);
  ctx.restore();
}

function colorWithAlpha(color, alpha = 1) {
  const value = String(color || '#ffffff');
  const short = value.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i);
  if (short) {
    return `rgba(${parseInt(short[1] + short[1], 16)}, ${parseInt(short[2] + short[2], 16)}, ${parseInt(short[3] + short[3], 16)}, ${alpha})`;
  }
  const long = value.match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})?$/i);
  if (long) {
    return `rgba(${parseInt(long[1], 16)}, ${parseInt(long[2], 16)}, ${parseInt(long[3], 16)}, ${alpha})`;
  }
  return value;
}

function easeOutCubic(value) {
  return 1 - ((1 - value) ** 3);
}

function easeOutBack(value) {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * ((value - 1) ** 3) + c1 * ((value - 1) ** 2);
}

function textAnimationState(text, local, duration) {
  const id = text?.animation;
  if (!id || id === 'none') return null;
  const spec = textAnimationById(id);
  if (!spec) return null;
  const phase = text.animationPhase || spec.phase || 'in';
  const animDuration = Math.max(0.1, textNumber(text.animationDuration, spec.duration || 0.6));
  let progress = 1;
  if (phase === 'in') progress = clamp01(local / animDuration);
  else if (phase === 'out') progress = clamp01((Math.max(0.001, duration) - local) / animDuration);
  const eased = easeOutCubic(progress);
  const state = {
    id, phase, progress, eased,
    alpha: 1, dx: 0, dy: 0, scale: 1, rotate: 0, blur: 0, reveal: 1,
  };
  const t = local;
  switch (id) {
    case 'in-fade':
      state.alpha = eased;
      break;
    case 'in-slide-up':
      state.alpha = eased;
      state.dy = (1 - eased) * 0.6;
      break;
    case 'in-slide-down':
      state.alpha = eased;
      state.dy = -(1 - eased) * 0.6;
      break;
    case 'in-slide-left':
      state.alpha = eased;
      state.dx = (1 - eased) * 0.7;
      break;
    case 'in-slide-right':
      state.alpha = eased;
      state.dx = -(1 - eased) * 0.7;
      break;
    case 'in-zoom':
      state.alpha = eased;
      state.scale = 0.7 + eased * 0.3;
      break;
    case 'in-pop':
      state.alpha = clamp01(eased * 3);
      state.scale = 0.3 + easeOutBack(eased) * 0.7;
      break;
    case 'in-typewriter':
      state.reveal = eased;
      break;
    case 'in-spin':
      state.alpha = eased;
      state.rotate = (1 - eased) * -22;
      break;
    case 'in-blur':
      state.alpha = clamp01(eased * 1.5);
      state.blur = (1 - eased) * 0.24;
      break;
    case 'out-fade':
      state.alpha = eased;
      break;
    case 'out-slide-up':
      state.alpha = eased;
      state.dy = -(1 - eased) * 0.45;
      break;
    case 'out-slide-down':
      state.alpha = eased;
      state.dy = (1 - eased) * 0.45;
      break;
    case 'out-zoom':
      state.alpha = eased;
      state.scale = 1 + (1 - eased) * 0.5;
      break;
    case 'loop-wave':
      state.dy = Math.sin(t * 3.2) * 0.08;
      break;
    case 'loop-pulse':
      state.scale = 1 + Math.sin(t * 4.4) * 0.035;
      break;
    case 'loop-shake':
      state.dx = Math.sin(t * 38) * 0.04 + Math.sin(t * 23) * 0.02;
      break;
    case 'loop-swing':
      state.rotate = Math.sin(t * 2.6) * 3.5;
      break;
    case 'loop-float':
      state.dy = Math.sin(t * 1.9) * 0.06;
      break;
    default:
      break;
  }
  return state;
}

/* Desfase de previsualización para las animaciones mientras el reproductor
   está pausado: los bucles avanzan y las entradas/salidas se repiten. */
function pausedAnimOffset(text, clock) {
  const spec = textAnimationById(text?.animation);
  if (!spec) return 0;
  const phase = text.animationPhase || spec.phase || 'in';
  if (phase === 'loop') return clock;
  const duration = Math.max(0.1, textNumber(text.animationDuration, spec.duration || 0.6));
  // Entrada: se reproduce una vez y deja el texto visible.
  if (phase === 'in') return Math.min(duration * 0.1 + clock, duration);
  // Salida: se repite para poder verla estando en pausa.
  return clock % (duration + 0.8);
}

function paintTextGlyph(ctx, run, x, y, text, metrics) {
  const strokeWidth = Math.max(0, textNumber(text.strokeWidth, 0)) *
    metrics.fontSize *
    (text.strokeMode === 'inside' ? 0.6 : text.strokeMode === 'center' ? 0.8 : 1);
  if (strokeWidth > 0.01) {
    ctx.lineWidth = strokeWidth;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    ctx.miterLimit = 2;
    ctx.strokeStyle = colorWithAlpha(text.strokeColor || '#000000', textNumber(text.strokeAlpha, 1));
    ctx.strokeText(run, x, y);
  }
  ctx.fillStyle = text.color || '#ffffff';
  ctx.fillText(run, x, y);
}

function paintTextLine(ctx, line, x, y, text, metrics) {
  if (!line) return;
  const spacing = metrics.letterSpacing;
  if (Math.abs(spacing) > 0.01) {
    let cursor = x;
    for (const character of Array.from(line)) {
      paintTextGlyph(ctx, character, cursor, y, text, metrics);
      cursor += ctx.measureText(character).width + spacing;
    }
    return;
  }
  paintTextGlyph(ctx, line, x, y, text, metrics);
}

function paintCurvedLine(ctx, line, centerX, y, text, metrics) {
  const characters = Array.from(line);
  if (!characters.length) return;
  const spacing = metrics.letterSpacing;
  const widths = characters.map((character) => ctx.measureText(character).width);
  const total = widths.reduce((sum, width) => sum + width, 0) + spacing * Math.max(0, characters.length - 1);
  const curve = textNumber(text.curve, 0);
  const span = Math.max(0.02, Math.min(0.98, Math.abs(curve) / 100) * Math.PI);
  const radius = Math.max(total / span, metrics.fontSize * 0.2);
  const direction = curve >= 0 ? 1 : -1;
  let cursor = 0;
  ctx.save();
  ctx.textAlign = 'center';
  characters.forEach((character, index) => {
    const width = widths[index];
    const center = cursor + width / 2;
    const angle = direction * ((center - total / 2) / radius);
    const px = centerX + radius * Math.sin(angle);
    const py = y + (direction > 0 ? radius * (1 - Math.cos(angle)) : -radius * (1 - Math.cos(angle)));
    ctx.save();
    ctx.translate(px, py);
    ctx.rotate(direction > 0 ? angle : -angle);
    paintTextGlyph(ctx, character, 0, 0, text, metrics);
    ctx.restore();
    cursor += width + spacing;
  });
  ctx.restore();
  ctx.textAlign = 'left';
}

function paintTextBlock(ctx, text, metrics, blockWidth, startY, revealInfo = {}) {
  const curve = textNumber(text.curve, 0);
  const align = text.align === 'left' ? 'left' : text.align === 'right' ? 'right' : 'center';
  const typewriter = Boolean(revealInfo.typewriter);
  const reveal = typewriter ? Math.max(0, Math.min(1, textNumber(revealInfo.reveal, 1))) : 1;
  const totalChars = metrics.lines.reduce((sum, line) => sum + Array.from(line).length + 1, 0);
  let remaining = Math.max(0, Math.ceil(totalChars * reveal));
  metrics.lines.forEach((line, index) => {
    let shown = line;
    if (typewriter) {
      const characters = Array.from(line);
      const count = Math.max(0, Math.min(characters.length, remaining));
      shown = characters.slice(0, count).join('');
      remaining -= characters.length + 1;
    }
    if (!shown) return;
    const lineWidth = measureRun(ctx, line, metrics.letterSpacing);
    const startX = align === 'left'
      ? -blockWidth / 2
      : align === 'right'
        ? blockWidth / 2 - lineWidth
        : -lineWidth / 2;
    const baselineY = startY + index * metrics.lineHeight;
    if (Math.abs(curve) > 1) paintCurvedLine(ctx, shown, startX + lineWidth / 2, baselineY, text, metrics);
    else paintTextLine(ctx, shown, startX, baselineY, text, metrics);
    if (text.underline && Math.abs(curve) <= 1) {
      ctx.beginPath();
      ctx.moveTo(startX, baselineY + metrics.fontSize * 0.42);
      ctx.lineTo(startX + lineWidth, baselineY + metrics.fontSize * 0.42);
      ctx.strokeStyle = text.color || '#ffffff';
      ctx.lineWidth = Math.max(1, metrics.fontSize * 0.045);
      ctx.stroke();
    }
  });
}

function clearShadow(ctx) {
  ctx.shadowColor = 'transparent';
  ctx.shadowBlur = 0;
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;
}

function drawTextLayer(ctx, layer, width, height, options = {}) {
  const { clip, local } = layer;
  const text = clip.text || {};
  const time = (clip.start || 0) + local;
  const tr = transformAt(clip, time);
  const anim = options.ignoreTransition ? null : transitionState(clip, local);
  const animClock = Number.isFinite(options.animClock) ? options.animClock : 0;
  const textAnim = textAnimationState(text, animClock ? local + pausedAnimOffset(text, animClock) : local, clip.duration || 1);
  const alpha = (options.ignoreFades ? tr.opacity : clipAlpha(clip, local)) *
    transitionAlpha(anim) *
    keyframeValue(clip, 'text.opacity', time, textNumber(text.opacity, 1)) *
    (textAnim?.alpha ?? 1);
  if (alpha <= 0.002) return;
  const metrics = textLayout(text, width, height);
  if (!metrics.lines.length) return;
  const { fontSize, lineHeight, blockHeight, blockWidth } = metrics;
  const offsetX = (tr.x + (text.x || 0)) / 100 * width;
  const offsetY = (tr.y + (text.y || 0)) / 100 * height;
  const centerX = width / 2 + offsetX;
  const centerY = height / 2 + offsetY;
  const revealInfo = {
    typewriter: text.animation === 'in-typewriter' && (text.animationPhase || 'in') === 'in',
    reveal: textAnim?.reveal ?? 1,
  };

  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.translate(centerX, centerY);
  if (tr.rotate) ctx.rotate((tr.rotate * Math.PI) / 180);
  applyEntrance(ctx, anim, width, height);
  const animScale = textAnim?.scale ?? 1;
  ctx.scale(
    tr.scaleX * (text.flipX ? -1 : 1) * animScale,
    tr.scaleY * (text.flipY ? -1 : 1) * animScale,
  );
  if (textAnim) {
    ctx.translate((textAnim.dx || 0) * fontSize, (textAnim.dy || 0) * fontSize);
    if (textAnim.rotate) ctx.rotate((textAnim.rotate * Math.PI) / 180);
    if (textAnim.blur > 0) ctx.filter = `blur(${textAnim.blur * fontSize}px)`;
  }
  wipeClip(ctx, anim, width, height);
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.font = metrics.font;
  if ('letterSpacing' in ctx) ctx.letterSpacing = '0px';

  if (text.background && text.background !== 'transparent') {
    const pad = textPadding(text, fontSize);
    const bgWidth = blockWidth + pad * 2;
    const bgHeight = blockHeight + pad * 2;
    let radius = Math.max(0, Math.min(0.5, textNumber(text.backgroundRadius, 0.16))) * Math.min(bgWidth, bgHeight);
    if (text.backgroundStyle === 'pill') radius = Math.min(bgWidth, bgHeight) / 2;
    else if (text.backgroundStyle === 'square') radius = 0;
    ctx.save();
    ctx.globalAlpha = alpha * Math.max(0, Math.min(1, textNumber(text.backgroundAlpha, 1)));
    ctx.fillStyle = text.background;
    roundedRect(ctx, -bgWidth / 2, -bgHeight / 2, bgWidth, bgHeight, radius);
    ctx.fill();
    const borderWidth = Math.max(0, textNumber(text.backgroundBorderWidth, 0)) * fontSize;
    if (borderWidth > 0.01) {
      ctx.lineWidth = borderWidth;
      ctx.strokeStyle = colorWithAlpha(text.backgroundBorderColor || '#000000', 1);
      ctx.stroke();
    }
    ctx.restore();
  }

  const startY = -blockHeight / 2 + lineHeight / 2;

  if (text.glow) {
    ctx.shadowColor = colorWithAlpha(text.glowColor || '#00e5d0', 1);
    ctx.shadowBlur = Math.max(0, textNumber(text.glowBlur, 0.35)) * fontSize;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 0;
    paintTextBlock(ctx, text, metrics, blockWidth, startY, revealInfo);
    clearShadow(ctx);
  }

  if (text.shadow !== false) {
    const angle = textNumber(text.shadowAngle, 90) * (Math.PI / 180);
    const distance = Math.max(0, textNumber(text.shadowDistance, 0.05)) * fontSize;
    ctx.shadowColor = colorWithAlpha(text.shadowColor || '#000000', textNumber(text.shadowAlpha, 0.58));
    ctx.shadowBlur = Math.max(0, textNumber(text.shadowBlur, 0.2)) * fontSize;
    ctx.shadowOffsetX = Math.cos(angle) * distance;
    ctx.shadowOffsetY = Math.sin(angle) * distance;
    paintTextBlock(ctx, text, metrics, blockWidth, startY, revealInfo);
    clearShadow(ctx);
  }

  paintTextBlock(ctx, text, metrics, blockWidth, startY, revealInfo);

  // Karaoke: resalta en amarillo la palabra activa si hay tiempos por palabra.
  if (Array.isArray(text.words) && text.words.length) {
    const sourceTime = (clip.in || 0) + local * (clip.speed || 1);
    const active = text.words.find((w) => sourceTime >= (w.start || 0) && sourceTime < (w.end || w.start + 0.5));
    if (active && active.word) {
      const target = String(active.word);
      let lineIndex = -1;
      for (let i = 0; i < metrics.lines.length; i++) {
        if (metrics.lines[i].toLowerCase().includes(target.toLowerCase())) { lineIndex = i; break; }
      }
      if (lineIndex >= 0) {
        const line = metrics.lines[lineIndex];
        const wordsInLine = line.split(/\s+/);
        const idx = wordsInLine.findIndex((w) => w.toLowerCase() === target.toLowerCase() || w.toLowerCase().includes(target.toLowerCase()));
        if (idx >= 0) {
          const prefix = wordsInLine.slice(0, idx).join(' ');
          const prefixWidth = prefix ? measureRun(ctx, prefix + ' ', metrics.letterSpacing) : 0;
          const wordWidth = measureRun(ctx, wordsInLine[idx], metrics.letterSpacing);
          const lineWidth = measureRun(ctx, line, metrics.letterSpacing);
          const align = text.align === 'left' ? 'left' : text.align === 'right' ? 'right' : 'center';
          const startX = align === 'left' ? -blockWidth / 2 : align === 'right' ? blockWidth / 2 - lineWidth : -lineWidth / 2;
          const baselineY = startY + lineIndex * metrics.lineHeight;
          ctx.save();
          ctx.fillStyle = '#ffeb3b';
          ctx.shadowColor = 'rgba(0,0,0,0.6)';
          ctx.shadowBlur = metrics.fontSize * 0.12;
          ctx.fillText(wordsInLine[idx], startX + prefixWidth, baselineY);
          ctx.restore();
        }
      }
    }
  }

  ctx.restore();
}

function drawVisualLayer(ctx, layer, width, height, pool, options = {}) {
  if (layer.clip.kind === 'text') drawTextLayer(ctx, layer, width, height, options);
  else if (layer.clip.kind === 'sticker') drawStickerLayer(ctx, layer, width, height, options);
  else if (layer.clip.kind === 'video' || layer.clip.kind === 'image') {
    drawMediaLayer(ctx, layer, width, height, pool, options);
  }
}

export function renderFrame(ctx, width, height, project, pool, time, { background = '#000000', animClock = 0 } = {}) {
  ctx.save();
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = background;
  ctx.fillRect(0, 0, width, height);
  const layers = activeLayers(project, time);
  let visual = 0;

  const whiteFlash = layers.find((layer) => {
    const anim = transitionState(layer.clip, layer.local);
    return anim?.type === 'white';
  });
  if (whiteFlash) {
    const anim = transitionState(whiteFlash.clip, whiteFlash.local);
    ctx.save();
    ctx.globalAlpha = 1 - anim.p;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, width, height);
    ctx.restore();
  }

  for (const layer of layers) {
    if (layer.clip.kind === 'audio') continue;
    const anim = transitionState(layer.clip, layer.local);
    if (anim && !['black', 'white', 'fade'].includes(anim.type)) {
      const previous = previousClipOnTrack(layer.track, layer.clip);
      if (previous) {
        const under = {
          ...previous,
          opacity: 1,
          fadeIn: 0,
          fadeOut: 0,
          transition: null,
          effect: previous.effect || 'none',
        };
        drawVisualLayer(ctx, { clip: under, track: layer.track, local: Math.max(0, previous.duration - 0.001) }, width, height, pool, { ignoreFades: true, ignoreTransition: true });
      }
    }
    drawVisualLayer(ctx, layer, width, height, pool, animClock ? { animClock } : {});
    visual += 1;
  }

  const blackFlash = layers.find((layer) => transitionState(layer.clip, layer.local)?.type === 'black');
  if (blackFlash) {
    const anim = transitionState(blackFlash.clip, blackFlash.local);
    ctx.save();
    ctx.globalAlpha = (1 - anim.p) * 0.55;
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, width, height);
    ctx.restore();
  }
  const flash = layers.find((layer) => transitionState(layer.clip, layer.local)?.type === 'flash');
  if (flash) {
    const anim = transitionState(flash.clip, flash.local);
    ctx.save();
    ctx.globalAlpha = (1 - anim.p) * 0.7;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, width, height);
    ctx.restore();
  }

  ctx.restore();
  return visual;
}

export async function renderStill(project, mediaMap, time, width, height) {
  const pool = new MediaPool(mediaMap);
  try {
    await pool.prepare(project, time);
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(2, Math.round(width));
    canvas.height = Math.max(2, Math.round(height));
    const ctx = canvas.getContext('2d');
    renderFrame(ctx, canvas.width, canvas.height, project, pool, time);
    return canvas;
  } finally {
    pool.destroy();
  }
}
