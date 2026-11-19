// vhs
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
fn hash_f(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let res = vec2<f32>(u.res_x, u.res_y);
    let line = sin(in.uv.y * res.y * 1.6) * 0.5 + 0.5;
    let wobble = sin(in.uv.y * 90.0 + u.time * 6.0) * 0.0018 * u.amount;
    let uv = vec2<f32>(in.uv.x + wobble, in.uv.y);
    let r = s(uv + vec2<f32>(0.0022 * u.amount, 0.0)).r;
    let g = s(uv).g;
    let b = s(uv - vec2<f32>(0.0022 * u.amount, 0.0)).b;
    var c = vec3<f32>(r, g, b) * (0.88 + line * 0.18);
    c += (hash_f(uv * res + u.time) - 0.5) * 0.06 * u.amount;
    return vec4<f32>(c, 1.0);
}
