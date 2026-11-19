// brillo-brumoso (CapCut 7683823805017443602)
// Brillo frío difuminado + grading de escena, sin depender del modelo de piel.
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

fn bright_at(uv: vec2<f32>) -> vec3<f32> {
    let c = s(uv).rgb;
    let v = smoothstep(0.52, 0.94, luma(c));
    return mix(vec3<f32>(luma(c)), c, 0.18) * v;
}
fn glow_ring(uv: vec2<f32>, radius: vec2<f32>) -> vec3<f32> {
    var b = bright_at(uv + radius * vec2<f32>(1.0, 0.0));
    b += bright_at(uv + radius * vec2<f32>(-1.0, 0.0));
    b += bright_at(uv + radius * vec2<f32>(0.0, 1.0));
    b += bright_at(uv + radius * vec2<f32>(0.0, -1.0));
    b += bright_at(uv + radius * vec2<f32>(0.7071, 0.7071));
    b += bright_at(uv + radius * vec2<f32>(-0.7071, 0.7071));
    b += bright_at(uv + radius * vec2<f32>(0.7071, -0.7071));
    b += bright_at(uv + radius * vec2<f32>(-0.7071, -0.7071));
    return b * 0.125;
}
fn scene_grade(c: vec3<f32>) -> vec3<f32> {
    let y = luma(c);
    let v = pow(max(y, 0.0), 0.80) * 0.96;
    var outc = vec3<f32>(v) + (c - y) * 0.32;
    outc = outc * 0.91 + vec3<f32>(0.028, 0.128, 0.223);
    let hi = smoothstep(0.52, 0.94, y);
    return mix(outc, vec3<f32>(0.96, 0.991, 1.0), hi * 0.28);
}
fn soft_warm_grade(c: vec3<f32>) -> vec3<f32> {
    var outc = vec3<f32>(0.08015, 0.15100, 0.26143);
    outc += c.r * vec3<f32>(0.82628, -0.67853, -1.04026);
    outc += c.g * vec3<f32>(0.86414, 2.20035, 1.31848);
    outc += c.b * vec3<f32>(0.07706, 0.12619, 1.22685);
    outc += c.r * c.r * vec3<f32>(0.24433, 0.53972, 0.59135);
    outc += c.g * c.g * vec3<f32>(-0.33383, -1.17251, -0.39698);
    outc += c.b * c.b * vec3<f32>(-1.26041, -0.76455, -1.29648);
    outc += c.r * c.g * vec3<f32>(-1.26740, -0.19806, -0.35591);
    outc += c.r * c.b * vec3<f32>(0.70729, 0.73092, 1.39186);
    outc += c.g * c.b * vec3<f32>(0.80338, -0.08116, -0.77414);
    return clamp(outc, vec3<f32>(0.0), vec3<f32>(1.0));
}
fn warm_midtone_blend(c: vec3<f32>) -> f32 {
    let y = luma(c);
    let warmth = smoothstep(-0.06, 0.16, c.r - c.b);
    let brightness = smoothstep(0.22, 0.62, y);
    let chroma = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
    var skin_color = smoothstep(0.015, 0.10, c.r - c.g);
    skin_color *= 1.0 - smoothstep(0.20, 0.42, chroma);
    skin_color *= 1.0 - smoothstep(0.03, 0.18, c.b - c.g);
    var red_accent = smoothstep(0.025, 0.18, c.r - max(c.g, c.b));
    red_accent *= smoothstep(0.14, 0.36, y) * (1.0 - smoothstep(0.66, 0.90, y));
    return clamp(0.12 + brightness * (0.16 + warmth * 0.58) + skin_color * (0.70 - brightness * 0.20) + red_accent * 0.22, 0.12, 0.86);
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let source = s(in.uv);
    let amount = clamp(u.amount, 0.0, 1.0);
    if (amount <= 0.00001) { return source; }
    let c = clamp(source.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let y = luma(c);
    let tonal_blend = warm_midtone_blend(c);
    let bloom_gain = mix(0.42, 0.10, smoothstep(0.30, 0.78, y));
    var graded = mix(scene_grade(c), soft_warm_grade(c), tonal_blend);
    let size = max(vec2<f32>(u.res_x, u.res_y), vec2<f32>(1.0));
    let px = 1.0 / size;
    let short_side = min(size.x, size.y);
    let unit = px * short_side;
    var bloom = glow_ring(in.uv, unit * 0.008) * 0.42;
    bloom += glow_ring(in.uv, unit * 0.025) * 0.35;
    bloom += glow_ring(in.uv, unit * 0.055) * 0.23;
    let cool_bloom = bloom * vec3<f32>(0.87, 0.96, 1.0);
    graded += (1.0 - clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0))) * cool_bloom * bloom_gain;
    graded += cool_bloom * bloom_gain * 0.12;
    return vec4<f32>(mix(c, clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0)), amount), source.a);
}
