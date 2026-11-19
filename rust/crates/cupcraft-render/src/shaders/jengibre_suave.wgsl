// jengibre-suave (CapCut 7571029701003808005): Desenfoque doble + DeepGlow + tono cálido + luz radial
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
    let soft = mix(blur9(in.uv, 0.008 * side), blur9(in.uv, 0.02 * side), 0.5);
    let y = luma(soft);
    var tone = mix(vec3<f32>(y), soft, 1.22);
    tone += vec3<f32>(0.10, 0.035, -0.05) * smoothstep(0.15, 0.75, y);
    tone = mix(tone, tone * vec3<f32>(1.12, 1.0, 0.86), 0.6);
    let wide = blur_wide(in.uv, 0.03 * side);
    tone += max(wide - 0.6, vec3<f32>(0.0)) * vec3<f32>(1.0, 0.72, 0.42) * 1.3;
    let light = 1.0 - smoothstep(0.1, 1.0, length((in.uv - vec2<f32>(0.5, 0.66)) * vec2<f32>(1.1, 1.45)));
    tone = 1.0 - (1.0 - tone) * (1.0 - vec3<f32>(1.0, 0.82, 0.6) * light * 0.16);
    return vec4<f32>(mix(src.rgb, clamp(tone, vec3<f32>(0.0), vec3<f32>(1.0)), clamp(u.amount, 0.0, 1.0)), src.a);
}
