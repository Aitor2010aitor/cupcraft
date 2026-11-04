// Runner WebGL de openCUP: post-proceso por shader sobre el fotograma ya compuesto.
// Los .frag son portes de shaders de CapCut extraídos de sus paquetes (AmazingFeature/InfoSticker).

const VERT = `#version 300 es
in vec2 a_pos;
out vec2 v_uv;
void main() {
  v_uv = a_pos * 0.5 + 0.5;
  gl_Position = vec4(a_pos, 0.0, 1.0);
}`;

const HEAD = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 outColor;
uniform sampler2D u_tex;
uniform float u_time;
uniform vec2 u_res;
uniform float u_amount;
`;

// Auxiliares compartidos por los filtros "other" de CapCut.
const LUMA = `float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }`;

const BLUR9 = `vec3 blur9(vec2 uv, float sigma) {
  vec2 r = vec2(sigma) / max(u_res, vec2(1.0));
  vec3 s = texture(u_tex, uv).rgb * 0.2;
  s += (texture(u_tex, uv + vec2(r.x, 0.0)).rgb + texture(u_tex, uv - vec2(r.x, 0.0)).rgb
      + texture(u_tex, uv + vec2(0.0, r.y)).rgb + texture(u_tex, uv - vec2(0.0, r.y)).rgb) * 0.12;
  s += (texture(u_tex, uv + r).rgb + texture(u_tex, uv - r).rgb
      + texture(u_tex, uv + vec2(r.x, -r.y)).rgb + texture(u_tex, uv + vec2(-r.x, r.y)).rgb) * 0.08;
  return s;
}
vec3 blurWide(vec2 uv, float sigma) { return mix(blur9(uv, sigma), blur9(uv, sigma * 2.4), 0.5); }`;

const SHADERS = {
  'radial-blur': HEAD + `float rnd(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
void main() {
  vec2 toCenter = vec2(0.5) - v_uv;
  float offset = rnd(v_uv);
  vec3 color = vec3(0.0);
  float total = 0.0;
  const int num = 25;
  for (int i = 0; i <= num; i++) {
    float p = (float(i) + offset) / float(num);
    float w = 4.0 * (p - p * p);
    color += texture(u_tex, v_uv + toCenter * p * u_amount * 0.25).rgb * w;
    total += w;
  }
  outColor = vec4(color / total, 1.0);
}`,

  'directional-blur': HEAD + `void main() {
  vec2 step = vec2(1.0) / u_res * u_amount * 8.0;
  vec3 c = vec3(0.0);
  const int num = 7;
  for (int i = -num; i <= num; i++) c += texture(u_tex, v_uv + step * float(i)).rgb;
  outColor = vec4(c / float(2 * num + 1), 1.0);
}`,

  'rgb-split': HEAD + `float hash(float n) { return fract(sin(n) * 43758.5453); }
void main() {
  float band = floor(v_uv.y * 42.0);
  float shift = (hash(band + floor(u_time * 8.0)) - 0.5) * u_amount * 0.05;
  vec2 d = vec2(shift, 0.0);
  outColor = vec4(
    texture(u_tex, v_uv + d).r,
    texture(u_tex, v_uv).g,
    texture(u_tex, v_uv - d).b, 1.0);
}`,

  'chromatic': HEAD + `void main() {
  vec2 dir = v_uv - vec2(0.5);
  float d = length(dir);
  vec2 off = dir * d * u_amount * 0.08;
  outColor = vec4(
    texture(u_tex, v_uv + off).r,
    texture(u_tex, v_uv).g,
    texture(u_tex, v_uv - off).b, 1.0);
}`,

  'vhs': HEAD + `float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
void main() {
  float line = sin(v_uv.y * u_res.y * 1.6) * 0.5 + 0.5;
  float wobble = sin(v_uv.y * 90.0 + u_time * 6.0) * 0.0018 * u_amount;
  vec2 uv = vec2(v_uv.x + wobble, v_uv.y);
  float r = texture(u_tex, uv + vec2(0.0022 * u_amount, 0.0)).r;
  float g = texture(u_tex, uv).g;
  float b = texture(u_tex, uv - vec2(0.0022 * u_amount, 0.0)).b;
  vec3 c = vec3(r, g, b) * (0.88 + line * 0.18);
  c += (hash(uv * u_res.xy + u_time) - 0.5) * 0.06 * u_amount;
  outColor = vec4(c, 1.0);
}`,

  'zoom-pulse': HEAD + `void main() {
  float z = 1.0 + sin(u_time * 3.0) * 0.06 * u_amount;
  vec2 uv = (v_uv - 0.5) / z + 0.5;
  outColor = texture(u_tex, uv);
}`,

  'pixelate': HEAD + `void main() {
  float px = max(2.0, 2.0 + u_amount * 24.0);
  vec2 uv = floor(v_uv * u_res / px) * px / u_res + (px * 0.5) / u_res;
  outColor = texture(u_tex, uv);
}`,

  'mirror': HEAD + `void main() {
  vec2 uv = v_uv;
  if (uv.x > 0.5) uv.x = 1.0 - uv.x;
  outColor = texture(u_tex, uv);
}`,

  'vignette': HEAD + `void main() {
  vec4 c = texture(u_tex, v_uv);
  float d = distance(v_uv, vec2(0.5));
  float v = smoothstep(0.78, 0.32, d * (0.9 + u_amount * 0.5));
  outColor = vec4(c.rgb * mix(0.45, 1.0, v), c.a);
}`,

  'grain': HEAD + `float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
void main() {
  vec4 c = texture(u_tex, v_uv);
  float n = hash(v_uv * u_res + u_time) - 0.5;
  outColor = vec4(c.rgb + n * 0.22 * u_amount, c.a);
}`,

  'posterize': HEAD + `void main() {
  vec4 c = texture(u_tex, v_uv);
  float steps = max(2.0, 12.0 - u_amount * 9.0);
  outColor = vec4(floor(c.rgb * steps + 0.5) / steps, c.a);
}`,

  'wave': HEAD + `void main() {
  vec2 uv = v_uv;
  uv.x += sin(uv.y * 18.0 + u_time * 2.0) * 0.012 * u_amount;
  uv.y += cos(uv.x * 14.0 + u_time * 1.7) * 0.010 * u_amount;
  outColor = texture(u_tex, uv);
}`,

  'bloom': HEAD + `void main() {
  vec3 base = texture(u_tex, v_uv).rgb;
  vec3 glow = vec3(0.0);
  const int n = 8;
  for (int i = 0; i < n; i++) {
    float a = float(i) * 0.785;
    glow += texture(u_tex, v_uv + vec2(cos(a), sin(a)) * 0.008 * u_amount).rgb;
  }
  glow /= float(n);
  outColor = vec4(base + max(glow - 0.55, 0.0) * 1.4 * u_amount, 1.0);
}`,

  // LUT de CapCut: mismo algoritmo que pass6.frag de sus paquetes de filtro
  // (cubo 64x64x64 repartido en 8x8 celdas de una textura 512x512).
  'capcut-lut': HEAD + `uniform sampler2D u_lut;
vec4 takeFilter(sampler2D filterTex, vec4 inputColor, float alpha) {
  highp vec4 textureColor = inputColor;
  highp float blueColor = textureColor.b * 63.0;
  highp vec2 quad1;
  quad1.y = floor(floor(blueColor) / 8.0);
  quad1.x = floor(blueColor) - (quad1.y * 8.0);
  highp vec2 quad2;
  quad2.y = floor(ceil(blueColor) / 8.0);
  quad2.x = ceil(blueColor) - (quad2.y * 8.0);
  highp vec2 texPos1;
  texPos1.x = (quad1.x * 1.0 / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * textureColor.r);
  texPos1.y = (quad1.y * 1.0 / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * textureColor.g);
  highp vec2 texPos2;
  texPos2.x = (quad2.x * 1.0 / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * textureColor.r);
  texPos2.y = (quad2.y * 1.0 / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * textureColor.g);
  vec4 newColor1 = texture(filterTex, texPos1);
  vec4 newColor2 = texture(filterTex, texPos2);
  vec4 newColor = mix(newColor1, newColor2, fract(blueColor));
  return mix(textureColor, vec4(newColor.rgb, textureColor.w), alpha);
}
void main() {
  vec4 curColor = texture(u_tex, v_uv);
  vec4 resultColor = takeFilter(u_lut, curColor, u_amount);
  resultColor.rgb = mix(curColor.rgb, resultColor.rgb, curColor.a);
  outColor = vec4(resultColor.rgb, curColor.a);
}`,

  // Curva tonal (sigmoid) de los filtros "curvas" de CapCut.
  'curve': HEAD + `void main() {
  vec4 c = texture(u_tex, v_uv);
  float slope = 4.0 + u_amount * 6.0;
  vec3 s = 1.0 / (1.0 + exp(-slope * (c.rgb - 0.5)));
  outColor = vec4(mix(c.rgb, s, 0.6 + 0.4 * u_amount), c.a);
}`,

  // ------------------------------------------------------------------
  // Filtros de CapCut sin LUT (familia "other"). Tres son el porte
  // directo de su .frag; los otros cinco componen en una sola pasada la
  // cadena multipaso del paquete con los parámetros de su main.scene.
  // ------------------------------------------------------------------

  // Bruma borgoña (7696747334054448391): porte directo de su pass6.frag
  // (papel rojo + trama de puntos de tinta rotada + contorno de tinta).
  'bruma-borgona': HEAD + `
vec2 dims() { return max(u_res, vec2(1.0)); }
vec2 safeUV(vec2 p) { vec2 t = 0.5 / dims(); return clamp(p, t, 1.0 - t); }
float lum(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }
float toneCurve(float l) { return mix(l, clamp((l - 0.01176471) / 0.77228549, 0.0, 1.0), 0.68); }
float L(vec2 p) { return toneCurve(lum(texture(u_tex, safeUV(p)).rgb)); }
float screenL(vec2 p) { return 0.24 + 0.76 * pow(max(L(p), 0.0), 0.62); }
float blurL(vec2 p, float radius) {
  vec2 d = vec2(radius * 1.73205081) / dims();
  return L(p) * 0.44444444 +
    (L(p + vec2(d.x, 0.0)) + L(p - vec2(d.x, 0.0)) + L(p + vec2(0.0, d.y)) + L(p - vec2(0.0, d.y))) * 0.11111111 +
    (L(p + d) + L(p - d) + L(p + vec2(d.x, -d.y)) + L(p + vec2(-d.x, d.y))) * 0.02777778;
}
float blurScreen(vec2 p, float radius) {
  vec2 d = vec2(radius * 1.73205081) / dims();
  return screenL(p) * 0.44444444 +
    (screenL(p + vec2(d.x, 0.0)) + screenL(p - vec2(d.x, 0.0)) +
     screenL(p + vec2(0.0, d.y)) + screenL(p - vec2(0.0, d.y))) * 0.11111111 +
    (screenL(p + d) + screenL(p - d) + screenL(p + vec2(d.x, -d.y)) + screenL(p + vec2(-d.x, d.y))) * 0.02777778;
}
vec3 retainedPalette(float t) {
  vec3 a = vec3(0.018, 0.003, 0.010), b = vec3(0.66, 0.035, 0.073), c = vec3(0.99, 0.75, 0.65);
  return t < 0.5 ? mix(a, b, t * 2.0) : mix(b, c, (t - 0.5) * 2.0);
}
void main() {
  vec4 original = texture(u_tex, safeUV(v_uv));
  float sc = max(min(dims().x, dims().y) / 900.0, 0.35);
  float l = toneCurve(lum(original.rgb));
  float cell = 8.6 * sc;
  vec2 p = v_uv * dims() - 0.5;
  vec2 grid = vec2(p.x * 0.8660254 - p.y * 0.5, p.x * 0.5 + p.y * 0.8660254) / cell;
  float dist = length(fract(grid) - 0.5);
  float radius = sqrt(clamp(1.0 - blurScreen(v_uv, cell * 0.32), 0.0, 1.0)) * 0.70;
  float aa = max(0.028, 0.42 / cell);
  float dots = 1.0 - smoothstep(radius - aa, radius + aa, dist);
  vec2 px = 1.0 / dims();
  float gx = (blurL(v_uv + vec2(px.x, 0.0), 0.65 * sc) - blurL(v_uv - vec2(px.x, 0.0), 0.65 * sc)) * 0.5;
  float gy = (blurL(v_uv + vec2(0.0, px.y), 0.65 * sc) - blurL(v_uv - vec2(0.0, px.y), 0.65 * sc)) * 0.5;
  float outline = smoothstep(0.028, 0.115, length(vec2(gx, gy))) * 0.45;
  float ink = max(dots, outline);
  vec3 paper = mix(vec3(0.94, 0.045, 0.10), vec3(1.0, 0.91, 0.82), smoothstep(0.70, 0.98, l) * 0.80);
  vec3 full = mix(paper, vec3(0.025, 0.005, 0.016), ink) * 0.75 + retainedPalette(l) * 0.25;
  outColor = vec4(mix(original.rgb, clamp(full, 0.0, 1.0), clamp(u_amount, 0.0, 1.0)), original.a);
}`,

  // Atardecer de ensueño (7694537677269486868): porte directo de su pass6.frag
  // (grading cálido de sol con máscaras de tono y velo de sol arriba a la derecha).
  'atardecer-ensueno': HEAD + `
float luminance(vec3 color) { return dot(color, vec3(0.299, 0.587, 0.114)); }
float softHueMask(float value, float center, float width) {
  float d = abs(value - center);
  d = min(d, 1.0 - d);
  return 1.0 - smoothstep(width * 0.45, width, d);
}
vec3 rgbToHsv(vec3 c) {
  vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
  vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
  vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
  float d = q.x - min(q.w, q.y);
  float e = 1.0e-10;
  return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}
vec3 warmSunGrade(vec3 color, vec2 uv) {
  vec3 hsv = rgbToHsv(color);
  float sourceY = luminance(color);
  float greenMask = softHueMask(hsv.x, 0.325, 0.16) * smoothstep(0.10, 0.42, hsv.y);
  float yellowMask = softHueMask(hsv.x, 0.155, 0.11) * smoothstep(0.10, 0.38, hsv.y);
  float blueMask = softHueMask(hsv.x, 0.565, 0.16) * smoothstep(0.10, 0.40, hsv.y);
  float skinMask = softHueMask(hsv.x, 0.065, 0.12) * smoothstep(0.06, 0.28, hsv.y);
  vec3 graded = color;
  float greenStrength = greenMask * (0.55 + 0.45 * (1.0 - sourceY));
  graded.r += greenStrength * (0.145 + graded.g * 0.095);
  graded.g += greenStrength * 0.022;
  graded.b -= greenStrength * 0.108;
  graded.r += yellowMask * 0.062;
  graded.g += yellowMask * 0.012;
  graded.b -= yellowMask * 0.058;
  graded.r -= blueMask * 0.018;
  graded.g += blueMask * 0.016;
  graded.b += blueMask * 0.020;
  graded.r += skinMask * 0.036;
  graded.g += skinMask * 0.007;
  graded.b -= skinMask * 0.022;
  graded = clamp(graded, 0.0, 1.0);
  graded *= vec3(1.026, 0.998, 0.944);
  graded.r += sourceY * 0.008;
  graded += vec3(0.005, 0.005, 0.003);
  graded = clamp(graded, 0.0, 1.0);
  float y = luminance(graded);
  vec3 saturated = mix(vec3(y), graded, 1.22);
  saturated = mix(saturated, mix(vec3(y), graded, 1.50), max(greenMask, yellowMask) * 0.76);
  float targetY = clamp((y - 0.50) * 1.16 + 0.50, 0.0, 1.0);
  targetY = pow(targetY, 0.985);
  saturated *= targetY / max(luminance(saturated), 0.025);
  graded = clamp(saturated, 0.0, 1.0);
  float afterY = luminance(graded);
  float shadow = 1.0 - smoothstep(0.16, 0.54, afterY);
  graded = mix(graded, graded * vec3(0.965, 1.010, 1.005) + vec3(0.004, 0.009, 0.007), shadow * 0.23);
  vec3 cream = vec3(1.000, 0.820, 0.515);
  float shoulder = smoothstep(0.60, 0.97, afterY);
  graded = vec3(1.0) - (vec3(1.0) - graded) * (vec3(1.0) - cream * shoulder * 0.095);
  vec2 sunVector = (uv - vec2(0.84, 0.08)) * vec2(0.86, 1.0);
  float sun = 1.0 - smoothstep(0.08, 0.76, length(sunVector));
  sun *= 0.45 + 0.55 * smoothstep(0.32, 0.90, sourceY);
  graded = vec3(1.0) - (vec3(1.0) - graded) * (vec3(1.0) - vec3(1.000, 0.735, 0.360) * sun * 0.125);
  float veil = smoothstep(0.42, 0.94, afterY);
  graded = mix(graded, vec3(1.000, 0.920, 0.760), veil * 0.045);
  float orangeRange = smoothstep(0.06, 0.38, sourceY);
  orangeRange *= 1.0 - smoothstep(0.90, 1.0, sourceY);
  float orangeColor = max(greenMask, yellowMask);
  float orangeAmount = orangeRange * (0.018 + orangeColor * 0.012 + skinMask * 0.004);
  graded += orangeAmount * vec3(1.000, -0.260, -0.550);
  graded = clamp(graded, 0.0, 1.0);
  vec2 centered = (uv - 0.5) * vec2(1.0, 0.88);
  float vignette = smoothstep(0.42, 0.70, length(centered));
  graded *= 1.0 - vignette * 0.085;
  return clamp(graded, 0.0, 1.0);
}
void main() {
  vec4 original = texture(u_tex, v_uv);
  float amount = clamp(u_amount, 0.0, 1.0);
  if (amount <= 0.00001) { outColor = original; return; }
  vec3 color = clamp(original.rgb, 0.0, 1.0);
  vec3 graded = warmSunGrade(color, vec2(v_uv.x, 1.0 - v_uv.y));
  outColor = vec4(mix(color, graded, amount), original.a);
}`,

  // Brillo brumoso (7683823805017443602): porte directo de su skinseg.frag
  // (brillo frío difuminado + grading de escena, sin depender del modelo de piel).
  'brillo-brumoso': HEAD + LUMA + `
vec3 brightAt(vec2 uv) {
  vec3 c = texture(u_tex, clamp(uv, vec2(0.0), vec2(1.0))).rgb;
  float v = smoothstep(0.52, 0.94, luma(c));
  return mix(vec3(luma(c)), c, 0.18) * v;
}
vec3 glowRing(vec2 uv, vec2 radius) {
  vec3 b = brightAt(uv + radius * vec2(1.0, 0.0));
  b += brightAt(uv + radius * vec2(-1.0, 0.0));
  b += brightAt(uv + radius * vec2(0.0, 1.0));
  b += brightAt(uv + radius * vec2(0.0, -1.0));
  b += brightAt(uv + radius * vec2(0.7071, 0.7071));
  b += brightAt(uv + radius * vec2(-0.7071, 0.7071));
  b += brightAt(uv + radius * vec2(0.7071, -0.7071));
  b += brightAt(uv + radius * vec2(-0.7071, -0.7071));
  return b * 0.125;
}
vec3 sceneGrade(vec3 c) {
  float y = luma(c);
  float v = pow(max(y, 0.0), 0.80) * 0.96;
  vec3 outc = vec3(v) + (c - y) * 0.32;
  outc = outc * 0.91 + vec3(0.028, 0.128, 0.223);
  float hi = smoothstep(0.52, 0.94, y);
  return mix(outc, vec3(0.96, 0.991, 1.0), hi * 0.28);
}
vec3 softWarmGrade(vec3 c) {
  vec3 outc = vec3(0.08015, 0.15100, 0.26143);
  outc += c.r * vec3(0.82628, -0.67853, -1.04026);
  outc += c.g * vec3(0.86414, 2.20035, 1.31848);
  outc += c.b * vec3(0.07706, 0.12619, 1.22685);
  outc += c.r * c.r * vec3(0.24433, 0.53972, 0.59135);
  outc += c.g * c.g * vec3(-0.33383, -1.17251, -0.39698);
  outc += c.b * c.b * vec3(-1.26041, -0.76455, -1.29648);
  outc += c.r * c.g * vec3(-1.26740, -0.19806, -0.35591);
  outc += c.r * c.b * vec3(0.70729, 0.73092, 1.39186);
  outc += c.g * c.b * vec3(0.80338, -0.08116, -0.77414);
  return clamp(outc, 0.0, 1.0);
}
float warmMidtoneBlend(vec3 c) {
  float y = luma(c);
  float warmth = smoothstep(-0.06, 0.16, c.r - c.b);
  float brightness = smoothstep(0.22, 0.62, y);
  float chroma = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
  float skinColor = smoothstep(0.015, 0.10, c.r - c.g);
  skinColor *= 1.0 - smoothstep(0.20, 0.42, chroma);
  skinColor *= 1.0 - smoothstep(0.03, 0.18, c.b - c.g);
  float redAccent = smoothstep(0.025, 0.18, c.r - max(c.g, c.b));
  redAccent *= smoothstep(0.14, 0.36, y) * (1.0 - smoothstep(0.66, 0.90, y));
  return clamp(0.12 + brightness * (0.16 + warmth * 0.58)
               + skinColor * (0.70 - brightness * 0.20) + redAccent * 0.22, 0.12, 0.86);
}
void main() {
  vec4 source = texture(u_tex, v_uv);
  float amount = clamp(u_amount, 0.0, 1.0);
  if (amount <= 0.00001) { outColor = source; return; }
  vec3 c = clamp(source.rgb, 0.0, 1.0);
  float y = luma(c);
  float tonalBlend = warmMidtoneBlend(c);
  float bloomGain = mix(0.42, 0.10, smoothstep(0.30, 0.78, y));
  vec3 graded = mix(sceneGrade(c), softWarmGrade(c), tonalBlend);
  vec2 size = max(u_res, vec2(1.0));
  vec2 px = 1.0 / size;
  float shortSide = min(size.x, size.y);
  vec2 unit = px * shortSide;
  vec3 bloom = glowRing(v_uv, unit * 0.008) * 0.42;
  bloom += glowRing(v_uv, unit * 0.025) * 0.35;
  bloom += glowRing(v_uv, unit * 0.055) * 0.23;
  vec3 coolBloom = bloom * vec3(0.87, 0.96, 1.0);
  graded += (1.0 - clamp(graded, 0.0, 1.0)) * coolBloom * bloomGain;
  graded += coolBloom * bloomGain * 0.12;
  outColor = vec4(mix(c, clamp(graded, 0.0, 1.0), amount), source.a);
}`,

  // Píxel estilo noir (7680864395202727186): cadena LumiHalftone (desenfoque +
  // semitono rotado + capa). Parámetros de su main.scene: dotFreq 50,
  // rotationAngle -30°, dotsRelativeWidth 1, dotsSharpen 1, colorMode Grayscale.
  'pixel-noir': HEAD + LUMA + BLUR9 + `
void main() {
  vec4 src = texture(u_tex, v_uv);
  float side = min(u_res.x, u_res.y);
  vec3 soft = blur9(v_uv, 0.0025 * side);
  float l = luma(soft);
  float a = radians(-30.0);
  vec2 p = v_uv * u_res;
  vec2 q = vec2(p.x * cos(a) - p.y * sin(a), p.x * sin(a) + p.y * cos(a));
  float cell = max(side / 50.0, 2.0);
  float d = length(fract(q / cell) - 0.5);
  float radius = sqrt(clamp(1.0 - l, 0.0, 1.0)) * 0.68;
  float aa = max(0.03, 0.35 / cell);
  float ink = 1.0 - smoothstep(radius - aa, radius + aa, d);
  vec3 noir = mix(vec3(0.955), vec3(0.035), ink);
  outColor = vec4(mix(src.rgb, noir, clamp(u_amount, 0.0, 1.0)), src.a);
}`,

  // Cariño retro (7543902649372904765): GaussianBlur (blurIntensity 20) + Layer +
  // Tritone. Tritone por defecto de su LumiTritone.lua: sombras oscuras, medios
  // (0.5,0.4,0.3), luces crema, oriAlpha 0 (tritone completo).
  'carino-retro': HEAD + LUMA + BLUR9 + `
void main() {
  vec4 src = texture(u_tex, v_uv);
  float side = min(u_res.x, u_res.y);
  vec3 soft = blur9(v_uv, 0.006 * side);
  float l = luma(soft);
  vec3 tone = mix(vec3(0.04, 0.026, 0.02), vec3(0.5, 0.4, 0.3), clamp(l / 0.5, 0.0, 1.0));
  tone = mix(tone, vec3(1.0, 0.96, 0.9), clamp((l - 0.5) / 0.5, 0.0, 1.0));
  tone = 1.0 - (1.0 - tone) * (1.0 - vec3(0.12, 0.06, 0.03) * 0.45);
  outColor = vec4(mix(src.rgb, clamp(tone, 0.0, 1.0), clamp(u_amount, 0.0, 1.0)), src.a);
}`,

  // Sueño caqui (7582494191255702837): GaussianBlur (blurIntensity 20) + Tritone +
  // Saturation (1.0) + DeepGlow (glowIter 8) + TemperatureTone + Exposure (1) +
  // ShadowHighlight (blancos 1 / negros 0: sin recorte).
  'sueno-caqui': HEAD + LUMA + BLUR9 + `
void main() {
  vec4 src = texture(u_tex, v_uv);
  float side = min(u_res.x, u_res.y);
  vec3 soft = blur9(v_uv, 0.005 * side);
  float l = luma(soft);
  vec3 tone = mix(vec3(0.23, 0.19, 0.17), vec3(0.62, 0.55, 0.47), clamp(l / 0.5, 0.0, 1.0));
  tone = mix(tone, vec3(1.0, 0.96, 0.89), clamp((l - 0.5) / 0.5, 0.0, 1.0));
  float y = luma(tone);
  tone = mix(vec3(y), tone, 1.06);
  tone *= vec3(1.05, 1.0, 0.95);
  vec3 glow = blurWide(v_uv, 0.02 * side);
  tone += max(glow - 0.62, 0.0) * 0.55 * vec3(1.0, 0.93, 0.82);
  outColor = vec4(mix(src.rgb, clamp(tone, 0.0, 1.0), clamp(u_amount, 0.0, 1.0)), src.a);
}`,

  // Jengibre suave (7571029701003808005): GaussianBlur ×2 + DeepGlow (glowIter 8)
  // + Saturation + Hsl + Layer + LightSensation (lightIntensity 1, luz radial
  // superior). Los mapas lightSensation_min/max.png se sustituyen por esa luz
  // radial procedural.
  'jengibre-suave': HEAD + LUMA + BLUR9 + `
void main() {
  vec4 src = texture(u_tex, v_uv);
  float side = min(u_res.x, u_res.y);
  vec3 soft = mix(blur9(v_uv, 0.008 * side), blur9(v_uv, 0.02 * side), 0.5);
  float y = luma(soft);
  vec3 tone = mix(vec3(y), soft, 1.22);
  tone += vec3(0.10, 0.035, -0.05) * smoothstep(0.15, 0.75, y);
  tone = mix(tone, tone * vec3(1.12, 1.0, 0.86), 0.6);
  vec3 wide = blurWide(v_uv, 0.03 * side);
  tone += max(wide - 0.6, 0.0) * vec3(1.0, 0.72, 0.42) * 1.3;
  float light = 1.0 - smoothstep(0.1, 1.0, length((v_uv - vec2(0.5, 0.66)) * vec2(1.1, 1.45)));
  tone = 1.0 - (1.0 - tone) * (1.0 - vec3(1.0, 0.82, 0.6) * light * 0.16);
  outColor = vec4(mix(src.rgb, clamp(tone, 0.0, 1.0), clamp(u_amount, 0.0, 1.0)), src.a);
}`,

  // Lienzo embrujado (7543902426269519157): AnimSeqLoadAndCrop (Texture1.png) +
  // Hsl + Layer + Saturation + Tritone + TurbulentNoise (complexity 7). La trama
  // de lienzo y el ruido turbulento se generan por procedimiento: los bitmaps de
  // CapCut no se redistribuyen.
  'lienzo-embrujado': HEAD + LUMA + `
float hash21(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float vnoise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), u.x),
             mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), u.x), u.y);
}
float fbm(vec2 p) {
  float v = 0.0, a = 0.5;
  for (int i = 0; i < 4; i++) { v += a * vnoise(p); p *= 2.03; a *= 0.5; }
  return v;
}
void main() {
  vec4 src = texture(u_tex, v_uv);
  vec2 px = v_uv * u_res;
  float weave = 0.93 + 0.07 * sin(px.x * 1.35) * sin(px.y * 1.35);
  vec3 c = src.rgb * weave;
  c *= 0.82 + 0.36 * fbm(px / 7.0);
  float y = luma(c);
  c = mix(vec3(y), c, 0.4);
  c *= vec3(0.9, 0.98, 1.08);
  float l = luma(c);
  vec3 tone = mix(vec3(0.02, 0.028, 0.034), vec3(0.3, 0.34, 0.35), clamp(l / 0.45, 0.0, 1.0));
  tone = mix(tone, vec3(0.86, 0.89, 0.88), clamp((l - 0.45) / 0.55, 0.0, 1.0));
  float vig = smoothstep(0.35, 0.95, length((v_uv - 0.5) * vec2(1.05, 1.0)));
  tone *= 1.0 - vig * 0.45;
  outColor = vec4(mix(src.rgb, clamp(tone, 0.0, 1.0), clamp(u_amount, 0.0, 1.0)), src.a);
}`,
};

// Filtros CapCut sin LUT ni familia propia → shader portado (clave del clip glFilter).
export const CAPCUT_SHADER_MAP = {
  '7696747334054448391': 'bruma-borgona',
  '7694537677269486868': 'atardecer-ensueno',
  '7683823805017443602': 'brillo-brumoso',
  '7680864395202727186': 'pixel-noir',
  '7543902649372904765': 'carino-retro',
  '7582494191255702837': 'sueno-caqui',
  '7571029701003808005': 'jengibre-suave',
  '7543902426269519157': 'lienzo-embrujado',
};

function compile(gl, type, src) {
  const sh = gl.createShader(type);
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(sh) || 'shader compile failed');
  }
  return sh;
}

class GLPost {
  constructor() {
    this.canvas = document.createElement('canvas');
    this.gl = this.canvas.getContext('webgl2', { premultipliedAlpha: false, preserveDrawingBuffer: true });
    this.programs = new Map();
    this.failed = !this.gl;
    if (this.failed) return;
    const gl = this.gl;
    this.quad = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    this.tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  }

  program(id) {
    if (this.programs.has(id)) return this.programs.get(id);
    if (!SHADERS[id]) return null;
    const gl = this.gl;
    const prog = gl.createProgram();
    gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VERT));
    gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, SHADERS[id]));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog) || 'link failed');
    const posLoc = gl.getAttribLocation(prog, 'a_pos');
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);
    this.programs.set(id, prog);
    return prog;
  }

  apply(sourceCanvas, effectId, time = 0, amount = 1) {
    if (this.failed) {
      console.warn('[openCUP][gl] runner deshabilitado tras un fallo previo; efecto ignorado:', effectId);
      return false;
    }
    if (!SHADERS[effectId]) {
      console.warn('[openCUP][gl] no hay shader portado para:', effectId);
      return false;
    }
    const gl = this.gl;
    try {
      if (this.canvas.width !== sourceCanvas.width || this.canvas.height !== sourceCanvas.height) {
        this.canvas.width = sourceCanvas.width;
        this.canvas.height = sourceCanvas.height;
      }
      gl.viewport(0, 0, this.canvas.width, this.canvas.height);
      gl.bindTexture(gl.TEXTURE_2D, this.tex);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, sourceCanvas);
      const prog = this.program(effectId);
      if (!prog) return false;
      gl.useProgram(prog);
      gl.uniform1i(gl.getUniformLocation(prog, 'u_tex'), 0);
      const t = gl.getUniformLocation(prog, 'u_time');
      if (t) gl.uniform1f(t, time);
      const r = gl.getUniformLocation(prog, 'u_res');
      if (r) gl.uniform2f(r, this.canvas.width, this.canvas.height);
      const a = gl.getUniformLocation(prog, 'u_amount');
      if (a) gl.uniform1f(a, amount);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      return true;
    } catch (error) {
      console.error('[openCUP][gl] fallo el shader ' + effectId + ': ' + (error && error.message ? error.message : error));
      this.failed = true;
      return false;
    }
  }

  /** Aplica un LUT de CapCut (imagen 512x512 con cubo 64^3) al fotograma. */
  applyLut(sourceCanvas, lutUrl, amount = 1) {
    if (this.failed || !lutUrl) return false;
    const img = lutImage(lutUrl);
    if (!img.complete || !img.naturalWidth) return false;
    const gl = this.gl;
    try {
      if (this.canvas.width !== sourceCanvas.width || this.canvas.height !== sourceCanvas.height) {
        this.canvas.width = sourceCanvas.width;
        this.canvas.height = sourceCanvas.height;
      }
      gl.viewport(0, 0, this.canvas.width, this.canvas.height);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.tex);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, sourceCanvas);

      if (!this.lutTex) {
        this.lutTex = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, this.lutTex);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      }
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, this.lutTex);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);

      const prog = this.program('capcut-lut');
      if (!prog) return false;
      gl.useProgram(prog);
      gl.uniform1i(gl.getUniformLocation(prog, 'u_tex'), 0);
      gl.uniform1i(gl.getUniformLocation(prog, 'u_lut'), 1);
      const a = gl.getUniformLocation(prog, 'u_amount');
      if (a) gl.uniform1f(a, amount);
      const r = gl.getUniformLocation(prog, 'u_res');
      if (r) gl.uniform2f(r, this.canvas.width, this.canvas.height);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      gl.activeTexture(gl.TEXTURE0);
      return true;
    } catch (error) {
      console.error('[openCUP][gl] LUT: ' + (error && error.message ? error.message : error));
      return false;
    }
  }

  /** Canvas WebGL con el fotograma ya procesado. */
  get canvasEl() {
    return this.canvas;
  }
}

const lutCache = new Map();

function lutImage(url) {
  let img = lutCache.get(url);
  if (!img) {
    img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => window.dispatchEvent(new CustomEvent('opencup:glfx'));
    img.onerror = () => console.warn('[openCUP][gl] no se pudo cargar el LUT', url);
    img.src = url;
    lutCache.set(url, img);
  }
  return img;
}

let instance = null;
export function getGLPost() {  if (!instance) instance = new GLPost();
  return instance;
}

export const GL_EFFECTS = Object.keys(SHADERS);

const NAME_MAP = [
  [/desenfoque|blur|bokeh|difumin/i, 'radial-blur'],
  [/movimiento|motion|barrid/i, 'directional-blur'],
  [/vhs|cinta|casete|analóg|analog/i, 'vhs'],
  [/glitch|falla|error|interferen|señal|senal|distorsi/i, 'rgb-split'],
  [/crom[aá]tic|aberraci|rgb/i, 'chromatic'],
  [/zoom|pulso|latido|acerc|alej/i, 'zoom-pulse'],
  [/pixel|mosaico|mosaic|bloque/i, 'pixelate'],
  [/espejo|mirror|reflej/i, 'mirror'],
  [/vi[nñ]eta|vignette/i, 'vignette'],
  [/grano|grain|ruido|noise/i, 'grain'],
  [/poster|umbral|threshold|reduce/i, 'posterize'],
  [/onda|wave|ripple|agua|líquid|liquid/i, 'wave'],
  [/brillo|glow|ne[oó]n|bloom|flash|luz/i, 'bloom'],
  [/explosi|fuego|llama|chispa|part[ií]cul|energ[ií]a|rayo|el[eé]ctric/i, 'bloom'],
  [/humo|niebla|fog|smoke|vapor/i, 'wave'],
];

export function capcutEffectToGL(name) {
  const hit = NAME_MAP.find(([re]) => re.test(String(name || '')));
  return hit ? hit[1] : null;
}
