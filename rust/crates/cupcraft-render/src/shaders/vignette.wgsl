// vignette
struct PostUniforms { time: f32, amount: f32, res_x: f32, res_y: f32 }
@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let c = textureSample(u_tex, u_sampler, in.uv);
    let d = distance(in.uv, vec2<f32>(0.5));
    let v = smoothstep(0.78, 0.32, d * (0.9 + u.amount * 0.5));
    return vec4<f32>(c.rgb * mix(0.45, 1.0, v), c.a);
}
