// directional-blur
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

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let res = vec2<f32>(u.res_x, u.res_y);
    let step = vec2<f32>(1.0) / res * u.amount * 8.0;
    var c = vec3<f32>(0.0);
    for (var i = -7; i <= 7; i++) { c += s(in.uv + step * f32(i)).rgb; }
    return vec4<f32>(c / 15.0, 1.0);
}
