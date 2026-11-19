// atardecer-ensueno (CapCut 7694537677269486868)
// Grading cálido de sol con máscaras de tono y velo de sol arriba a la derecha.
struct PostUniforms { time: f32, amount: f32, res_x: f32, res_y: f32 }
@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}
fn luminance(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
fn soft_hue_mask(value: f32, center: f32, width: f32) -> f32 {
    var d = abs(value - center);
    d = min(d, 1.0 - d);
    return 1.0 - smoothstep(width * 0.45, width, d);
}
fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0/3.0, 2.0/3.0, -1.0);
    let p = select(vec4<f32>(c.gb, K.wz), vec4<f32>(c.bg, K.xy), c.b < c.g);
    let q = select(vec4<f32>(c.r, p.yzx), vec4<f32>(p.xyw, c.r), p.x < c.r);
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y)/(6.0*d+e)), d/(q.x+e), q.x);
}

fn warm_sun_grade(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let hsv = rgb_to_hsv(color);
    let source_y = luminance(color);
    let green_mask = soft_hue_mask(hsv.x, 0.325, 0.16) * smoothstep(0.10, 0.42, hsv.y);
    let yellow_mask = soft_hue_mask(hsv.x, 0.155, 0.11) * smoothstep(0.10, 0.38, hsv.y);
    let blue_mask = soft_hue_mask(hsv.x, 0.565, 0.16) * smoothstep(0.10, 0.40, hsv.y);
    let skin_mask = soft_hue_mask(hsv.x, 0.065, 0.12) * smoothstep(0.06, 0.28, hsv.y);
    var graded = color;
    let gs = green_mask * (0.55 + 0.45 * (1.0 - source_y));
    graded.r += gs * (0.145 + graded.g * 0.095);
    graded.g += gs * 0.022;
    graded.b -= gs * 0.108;
    graded.r += yellow_mask * 0.062;  graded.g += yellow_mask * 0.012;  graded.b -= yellow_mask * 0.058;
    graded.r -= blue_mask * 0.018;    graded.g += blue_mask * 0.016;    graded.b += blue_mask * 0.020;
    graded.r += skin_mask * 0.036;    graded.g += skin_mask * 0.007;    graded.b -= skin_mask * 0.022;
    graded = clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0));
    graded *= vec3<f32>(1.026, 0.998, 0.944);
    graded.r += source_y * 0.008;
    graded += vec3<f32>(0.005, 0.005, 0.003);
    graded = clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0));
    let y = luminance(graded);
    var saturated = mix(vec3<f32>(y), graded, 1.22);
    saturated = mix(saturated, mix(vec3<f32>(y), graded, 1.50), max(green_mask, yellow_mask) * 0.76);
    var target_y = clamp((y - 0.50) * 1.16 + 0.50, 0.0, 1.0);
    target_y = pow(target_y, 0.985);
    saturated *= target_y / max(luminance(saturated), 0.025);
    graded = clamp(saturated, vec3<f32>(0.0), vec3<f32>(1.0));
    let after_y = luminance(graded);
    let shadow = 1.0 - smoothstep(0.16, 0.54, after_y);
    graded = mix(graded, graded * vec3<f32>(0.965, 1.010, 1.005) + vec3<f32>(0.004, 0.009, 0.007), shadow * 0.23);
    let cream = vec3<f32>(1.000, 0.820, 0.515);
    let shoulder = smoothstep(0.60, 0.97, after_y);
    graded = vec3<f32>(1.0) - (vec3<f32>(1.0) - graded) * (vec3<f32>(1.0) - cream * shoulder * 0.095);
    let sun_vec = (uv - vec2<f32>(0.84, 0.08)) * vec2<f32>(0.86, 1.0);
    var sun = 1.0 - smoothstep(0.08, 0.76, length(sun_vec));
    sun *= 0.45 + 0.55 * smoothstep(0.32, 0.90, source_y);
    graded = vec3<f32>(1.0) - (vec3<f32>(1.0) - graded) * (vec3<f32>(1.0) - vec3<f32>(1.000, 0.735, 0.360) * sun * 0.125);
    let veil = smoothstep(0.42, 0.94, after_y);
    graded = mix(graded, vec3<f32>(1.000, 0.920, 0.760), veil * 0.045);
    let orange_range = smoothstep(0.06, 0.38, source_y) * (1.0 - smoothstep(0.90, 1.0, source_y));
    let orange_color = max(green_mask, yellow_mask);
    let orange_amount = orange_range * (0.018 + orange_color * 0.012 + skin_mask * 0.004);
    graded += orange_amount * vec3<f32>(1.000, -0.260, -0.550);
    graded = clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0));
    let centered = (uv - 0.5) * vec2<f32>(1.0, 0.88);
    let vignette = smoothstep(0.42, 0.70, length(centered));
    graded *= 1.0 - vignette * 0.085;
    return clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let original = textureSample(u_tex, u_sampler, in.uv);
    let amount = clamp(u.amount, 0.0, 1.0);
    if (amount <= 0.00001) { return original; }
    let color = clamp(original.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let graded = warm_sun_grade(color, vec2<f32>(in.uv.x, 1.0 - in.uv.y));
    return vec4<f32>(mix(color, graded, amount), original.a);
}
