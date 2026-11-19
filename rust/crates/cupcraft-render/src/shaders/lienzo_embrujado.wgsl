// lienzo-embrujado (CapCut 7543902426269519157): trama de lienzo + ruido turbulento + tritone
struct PostUniforms { time: f32, amount: f32, res_x: f32, res_y: f32 }
@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}
fn luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
fn hash21(p: vec2<f32>) -> f32 {
    var pv = fract(p * vec2<f32>(123.34, 456.21)); pv += dot(pv, pv + 45.32); return fract(pv.x * pv.y);
}
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p); let f = fract(p);
    let u2 = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i), hash21(i+vec2<f32>(1.0,0.0)), u2.x),
               mix(hash21(i+vec2<f32>(0.0,1.0)), hash21(i+vec2<f32>(1.0,1.0)), u2.x), u2.y);
}
fn fbm(pin: vec2<f32>) -> f32 {
    var v = 0.0; var a = 0.5; var p = pin;
    for (var i = 0; i < 4; i++) { v += a * vnoise(p); p *= 2.03; a *= 0.5; }
    return v;
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let src = textureSample(u_tex, u_sampler, in.uv);
    let res = vec2<f32>(u.res_x, u.res_y);
    let px = in.uv * res;
    let weave = 0.93 + 0.07 * sin(px.x * 1.35) * sin(px.y * 1.35);
    var c = src.rgb * weave;
    c *= 0.82 + 0.36 * fbm(px / 7.0);
    let y = luma(c);
    c = mix(vec3<f32>(y), c, 0.4);
    c *= vec3<f32>(0.9, 0.98, 1.08);
    let l = luma(c);
    var tone = mix(vec3<f32>(0.02, 0.028, 0.034), vec3<f32>(0.3, 0.34, 0.35), clamp(l / 0.45, 0.0, 1.0));
    tone = mix(tone, vec3<f32>(0.86, 0.89, 0.88), clamp((l - 0.45) / 0.55, 0.0, 1.0));
    let vig = smoothstep(0.35, 0.95, length((in.uv - 0.5) * vec2<f32>(1.05, 1.0)));
    tone *= 1.0 - vig * 0.45;
    return vec4<f32>(mix(src.rgb, clamp(tone, vec3<f32>(0.0), vec3<f32>(1.0)), clamp(u.amount, 0.0, 1.0)), src.a);
}
