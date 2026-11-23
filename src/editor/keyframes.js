/* Fotogramas clave de cupcraft.
   Cada clip puede guardar ``clip.keyframes = { x: [{ t, v }], ... }`` donde
   ``t`` es el tiempo absoluto en el proyecto y ``v`` el valor de la propiedad.
   La interpolación es lineal y la evalúa el renderer en cada fotograma. */

export const KEYFRAME_PROPS = ['x', 'y', 'scale', 'scaleX', 'scaleY', 'rotate', 'opacity', 'text.opacity'];

const EPSILON = 1 / 120;

export function keyframesOf(clip, prop) {
  const list = clip?.keyframes?.[prop];
  return Array.isArray(list) && list.length ? list : null;
}

export function hasKeyframes(clip, prop) {
  return Boolean(keyframesOf(clip, prop));
}

export function clipHasKeyframes(clip) {
  const store = clip?.keyframes;
  if (!store) return false;
  return Object.values(store).some((list) => Array.isArray(list) && list.length);
}

export function keyframeCount(clip, prop) {
  return keyframesOf(clip, prop)?.length || 0;
}

function interpolate(list, time) {
  if (!list.length) return null;
  if (time <= list[0].t) return list[0].v;
  const last = list[list.length - 1];
  if (time >= last.t) return last.v;
  for (let index = 1; index < list.length; index += 1) {
    const next = list[index];
    if (time <= next.t) {
      const prev = list[index - 1];
      const span = next.t - prev.t;
      const progress = span > 0 ? (time - prev.t) / span : 1;
      return prev.v + (next.v - prev.v) * progress;
    }
  }
  return last.v;
}

export function keyframeValue(clip, prop, time, fallback) {
  const list = keyframesOf(clip, prop);
  if (!list) return fallback;
  const value = interpolate(list, time);
  return value === null ? fallback : value;
}

export function keyframeIndexAt(clip, prop, time) {
  const list = keyframesOf(clip, prop);
  if (!list) return -1;
  return list.findIndex((point) => Math.abs(point.t - time) <= EPSILON);
}

/* Devuelve una NUEVA tienda de fotogramas con el punto insertado/actualizado.
   No muta el clip: así el historial de deshacer ve el antes y el después. */
export function nextKeyframeStore(clip, prop, time, value) {
  const store = { ...(clip.keyframes || {}) };
  const list = [...(store[prop] || [])];
  const existing = list.findIndex((point) => Math.abs(point.t - time) <= EPSILON);
  if (existing >= 0) {
    list[existing] = { t: time, v: value };
  } else {
    list.push({ t: time, v: value });
  }
  list.sort((a, b) => a.t - b.t);
  store[prop] = list;
  return store;
}

export function withoutKeyframe(clip, prop, time) {
  const store = { ...(clip.keyframes || {}) };
  const list = (store[prop] || []).filter((point) => Math.abs(point.t - time) > EPSILON);
  if (list.length) store[prop] = list;
  else delete store[prop];
  return store;
}

/* Variantes mutantes, para el arrastre del preview (se restauran al soltar). */
export function setKeyframe(clip, prop, time, value) {
  clip.keyframes = nextKeyframeStore(clip, prop, time, value);
  return clip.keyframes;
}

export function clearKeyframe(clip, prop, time) {
  clip.keyframes = withoutKeyframe(clip, prop, time);
  return clip.keyframes;
}

export function clearAllKeyframes(clip) {
  clip.keyframes = {};
}

/* Vecino anterior/siguiente respecto al cabezal, para navegar con ‹ ›. */
export function neighborKeyframe(clip, prop, time, direction) {
  const list = keyframesOf(clip, prop);
  if (!list) return null;
  if (direction > 0) return list.find((point) => point.t > time + EPSILON) || null;
  for (let index = list.length - 1; index >= 0; index -= 1) {
    if (list[index].t < time - EPSILON) return list[index];
  }
  return null;
}

/* Transformación completa de un clip en el instante ``time``.
   Respeta la escala uniforme: si está activada, X eY comparten valor. */
export function transformAt(clip, time) {
  const uniform = clip.uniform !== false;
  const baseScale = clip.scale ?? 1;
  const scaleX = uniform
    ? keyframeValue(clip, 'scale', time, baseScale)
    : keyframeValue(clip, 'scaleX', time, clip.scaleX ?? baseScale);
  const scaleY = uniform
    ? keyframeValue(clip, 'scale', time, baseScale)
    : keyframeValue(clip, 'scaleY', time, clip.scaleY ?? baseScale);
  return {
    x: keyframeValue(clip, 'x', time, clip.x || 0),
    y: keyframeValue(clip, 'y', time, clip.y || 0),
    rotate: keyframeValue(clip, 'rotate', time, clip.rotate || 0),
    scaleX,
    scaleY,
    opacity: keyframeValue(clip, 'opacity', time, clip.opacity ?? 1),
  };
}

/* Escribe una propiedad respetando los fotogramas clave existentes:
   si la propiedad ya tiene keyframes, se crea/actualiza uno en el cabezal. */
export function writeProp(clip, prop, value, time) {
  if (hasKeyframes(clip, prop)) {
    setKeyframe(clip, prop, time, value);
    return true;
  }
  const path = prop.split('.');
  if (path.length === 2) {
    const [head, tail] = path;
    clip[head] = { ...(clip[head] || {}), [tail]: value };
  } else {
    clip[prop] = value;
  }
  return false;
}
