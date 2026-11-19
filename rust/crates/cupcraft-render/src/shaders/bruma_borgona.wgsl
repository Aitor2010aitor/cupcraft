// bruma-borgona (CapCut 7696747334054448391)
// Papel borgoña + trama de puntos en tinta rotada + contorno de tinta.
struct PostUniforms { time: f32, amount: f32, res_x: f32, res_y: f32 }
@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}
fn dims() -> vec2<f32> { return max(vec2<f32>(u.res_x, u.res_y), vec2<f32>(1.0)); }
fn safe_uv(p: vec2<f32>) -> vec2<f32> { let t = 0.5 / dims(); return clamp(p, t, vec2<f32>(1.0) - t); }
fn lum(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }
fn tone_curve(l: f32) -> f32 { return mix(l, clamp((l - 0.01176471) / 0.77228549, 0.0, 1.0), 0.68); }
fn L(p: vec2<f32>) -> f32 { return tone_curve(lum(textureSample(u_tex, u_sampler, safe_uv(p)).rgb)); }
fn screen_l(p: vec2<f32>) -> f32 { return 0.24 + 0.76 * pow(max(L(p), 0.0), 0.62); }
fn blur_l(p: vec2<f32>, radius: f32) -> f32 {
    let d = vec2<f32>(radius * 1.73205081) / dims();
    return L(p) * 0.44444444 +
        (L(p + vec2<f32>(d.x,0.0)) + L(p - vec2<f32>(d.x,0.0)) + L(p + vec2<f32>(0.0,d.y)) + L(p - vec2<f32>(0.0,d.y))) * 0.11111111 +
        (L(p+d) + L(p-d) + L(p+vec2<f32>(d.x,-d.y)) + L(p+vec2<f32>(-d.x,d.y))) * 0.02777778;
}
fn blur_screen(p: vec2<f32>, radius: f32) -> f32 {
    let d = vec2<f32>(radius * 1.73205081) / dims();
    return screen_l(p) * 0.44444444 +
        (screen_l(p+vec2<f32>(d.x,0.0)) + screen_l(p-vec2<f32>(d.x,0.0)) +
         screen_l(p+vec2<f32>(0.0,d.y)) + screen_l(p-vec2<f32>(0.0,d.y))) * 0.11111111 +
        (screen_l(p+d) + screen_l(p-d) + screen_l(p+vec2<f32>(d.x,-d.y)) + screen_l(p+vec2<f32>(-d.x,d.y))) * 0.02777778;
}
fn retained_palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.018, 0.003, 0.010);
    let b = vec3<f32>(0.66, 0.035, 0.073);
    let c = vec3<f32>(0.99, 0.75, 0.65);
    if (t < 0.5) { return mix(a, b, t * 2.0); } else { return mix(b, c, (t - 0.5) * 2.0); }
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let original = textureSample(u_tex, u_sampler, safe_uv(in.uv));
    let sc = max(min(dims().x, dims().y) / 900.0, 0.35);
    let l = tone_curve(lum(original.rgb));
    let cell = 8.6 * sc;
    let p = in.uv * dims() - 0.5;
    let grid = vec2<f32>(p.x * 0.8660254 - p.y * 0.5, p.x * 0.5 + p.y * 0.8660254) / cell;
    let dist = length(fract(grid) - 0.5);
    let radius = sqrt(clamp(1.0 - blur_screen(in.uv, cell * 0.32), 0.0, 1.0)) * 0.70;
    let aa = max(0.028, 0.42 / cell);
    let dots = 1.0 - smoothstep(radius - aa, radius + aa, dist);
    let px = 1.0 / dims();
    let gx = (blur_l(in.uv + vec2<f32>(px.x, 0.0), 0.65*sc) - blur_l(in.uv - vec2<f32>(px.x, 0.0), 0.65*sc)) * 0.5;
    let gy = (blur_l(in.uv + vec2<f32>(0.0, px.y), 0.65*sc) - blur_l(in.uv - vec2<f32>(0.0, px.y), 0.65*sc)) * 0.5;
    let outline = smoothstep(0.028, 0.115, length(vec2<f32>(gx, gy))) * 0.45;
    let ink = max(dots, outline);
    let paper = mix(vec3<f32>(0.94, 0.045, 0.10), vec3<f32>(1.0, 0.91, 0.82), smoothstep(0.70, 0.98, l) * 0.80);
    let full = mix(paper, vec3<f32>(0.025, 0.005, 0.016), ink) * 0.75 + retained_palette(l) * 0.25;
    return vec4<f32>(mix(original.rgb, clamp(full, vec3<f32>(0.0), vec3<f32>(1.0)), clamp(u.amount, 0.0, 1.0)), original.a);
}
