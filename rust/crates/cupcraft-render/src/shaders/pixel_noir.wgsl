// pixel-noir (CapCut 7680864395202727186)
// LumiHalftone: desenfoque + semitono rotado + capa. 
// Parámetros de main.scene: dotFreq 50, rotation -30°, colorMode Grayscale.
struct PostUniforms { time: f32, amount: f32, res_x: f32, res_y: f32 }
@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}
fn s(uv: vec2<f32>) -> vec4<f32> { return textureSample(u_tex, u_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))); }
fn luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
fn blur9(uv: vec2<f32>, sigma: f32) -> vec3<f32> {
    let res = max(vec2<f32>(u.res_x, u.res_y), vec2<f32>(1.0));
    let r = vec2<f32>(sigma) / res;
    var sc = s(uv).rgb * 0.2;
    sc += (s(uv+vec2<f32>(r.x,0.0)).rgb + s(uv-vec2<f32>(r.x,0.0)).rgb + s(uv+vec2<f32>(0.0,r.y)).rgb + s(uv-vec2<f32>(0.0,r.y)).rgb) * 0.12;
    sc += (s(uv+r).rgb + s(uv-r).rgb + s(uv+vec2<f32>(r.x,-r.y)).rgb + s(uv+vec2<f32>(-r.x,r.y)).rgb) * 0.08;
    return sc;
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let res = vec2<f32>(u.res_x, u.res_y);
    let src = s(in.uv);
    let side = min(res.x, res.y);
    let soft = blur9(in.uv, 0.0025 * side);
    let l = luma(soft);
    let ang = radians(-30.0);
    let p = in.uv * res;
    let q = vec2<f32>(p.x * cos(ang) - p.y * sin(ang), p.x * sin(ang) + p.y * cos(ang));
    let cell = max(side / 50.0, 2.0);
    let d = length(fract(q / cell) - 0.5);
    let radius = sqrt(clamp(1.0 - l, 0.0, 1.0)) * 0.68;
    let aa = max(0.03, 0.35 / cell);
    let ink = 1.0 - smoothstep(radius - aa, radius + aa, d);
    let noir = mix(vec3<f32>(0.955), vec3<f32>(0.035), ink);
    return vec4<f32>(mix(src.rgb, noir, clamp(u.amount, 0.0, 1.0)), src.a);
}
