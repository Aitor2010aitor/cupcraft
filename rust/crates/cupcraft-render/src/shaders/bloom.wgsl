// bloom
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
    let base = textureSample(u_tex, u_sampler, in.uv).rgb;
    var glow = vec3<f32>(0.0);
    for (var i = 0; i < 8; i++) {
        let a = f32(i) * 0.785398; // pi/4
        glow += textureSample(u_tex, u_sampler,
            clamp(in.uv + vec2<f32>(cos(a), sin(a)) * 0.008 * u.amount, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
    }
    glow /= 8.0;
    return vec4<f32>(base + max(glow - 0.55, vec3<f32>(0.0)) * 1.4 * u.amount, 1.0);
}
