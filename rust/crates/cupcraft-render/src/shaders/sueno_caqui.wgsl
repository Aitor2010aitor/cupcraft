// sueno-caqui (CapCut 7582494191255702837): Tritone + Saturación + DeepGlow + TemperaturaTono
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
    sc += (s(uv+vec2<f32>(r.x,0.0)).rgb+s(uv-vec2<f32>(r.x,0.0)).rgb+s(uv+vec2<f32>(0.0,r.y)).rgb+s(uv-vec2<f32>(0.0,r.y)).rgb)*0.12;
    sc += (s(uv+r).rgb+s(uv-r).rgb+s(uv+vec2<f32>(r.x,-r.y)).rgb+s(uv+vec2<f32>(-r.x,r.y)).rgb)*0.08;
    return sc;
}
fn blur_wide(uv: vec2<f32>, sigma: f32) -> vec3<f32> { return mix(blur9(uv, sigma), blur9(uv, sigma * 2.4), 0.5); }

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let src = s(in.uv);
    let side = min(u.res_x, u.res_y);
    let soft = blur9(in.uv, 0.005 * side);
    let l = luma(soft);
    var tone = mix(vec3<f32>(0.23, 0.19, 0.17), vec3<f32>(0.62, 0.55, 0.47), clamp(l / 0.5, 0.0, 1.0));
    tone = mix(tone, vec3<f32>(1.0, 0.96, 0.89), clamp((l - 0.5) / 0.5, 0.0, 1.0));
    let y = luma(tone);
    tone = mix(vec3<f32>(y), tone, 1.06);
    tone *= vec3<f32>(1.05, 1.0, 0.95);
    let glow = blur_wide(in.uv, 0.02 * side);
    tone += max(glow - 0.62, vec3<f32>(0.0)) * 0.55 * vec3<f32>(1.0, 0.93, 0.82);
    return vec4<f32>(mix(src.rgb, clamp(tone, vec3<f32>(0.0), vec3<f32>(1.0)), clamp(u.amount, 0.0, 1.0)), src.a);
}
