// AUTO-GENERATED from legacy/src/gl-post.js — radial-blur
// Incluye los bindings de post_common.wgsl inline para ser un módulo WGSL autónomo.

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
fn rnd(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let toCenter = vec2<f32>(0.5) - uv;
    let offset = rnd(uv);
    var color = vec3<f32>(0.0);
    var total = 0.0f;
    for (var i = 0; i <= 25; i++) {
        let p = (f32(i) + offset) / 25.0;
        let w = 4.0 * (p - p * p);
        color += s(uv + toCenter * p * u.amount * 0.25).rgb * w;
        total += w;
    }
    return vec4<f32>(color / max(total, 0.001), 1.0);
}
