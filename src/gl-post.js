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
    if (this.failed || !SHADERS[effectId]) return false;
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
    } catch {
      this.failed = true;
      return false;
    }
  }
}

let instance = null;
export function getGLPost() {
  if (!instance) instance = new GLPost();
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
