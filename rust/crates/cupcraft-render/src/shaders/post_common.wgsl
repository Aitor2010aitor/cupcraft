// === Uniforms de post-proceso ===
struct PostUniforms {
    time: f32,
    amount: f32,
    res_x: f32,
    res_y: f32,
};

@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Quad fullscreen: posiciones NDC directas
@vertex
fn vs_main(@location(0) a_pos: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.uv = a_pos * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y; // flip Y para coincidir con la textura GPU
    out.pos = vec4<f32>(a_pos, 0.0, 1.0);
    return out;
}

fn sample(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(u_tex, u_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}

fn hash_f(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn hash1(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn blur9(uv: vec2<f32>, sigma: f32) -> vec3<f32> {
    let res = max(vec2<f32>(u.res_x, u.res_y), vec2<f32>(1.0));
    let r = vec2<f32>(sigma) / res;
    var s = sample(uv).rgb * 0.2;
    s += (sample(uv + vec2<f32>(r.x, 0.0)).rgb + sample(uv - vec2<f32>(r.x, 0.0)).rgb
        + sample(uv + vec2<f32>(0.0, r.y)).rgb + sample(uv - vec2<f32>(0.0, r.y)).rgb) * 0.12;
    s += (sample(uv + r).rgb + sample(uv - r).rgb
        + sample(uv + vec2<f32>(r.x, -r.y)).rgb + sample(uv + vec2<f32>(-r.x, r.y)).rgb) * 0.08;
    return s;
}

fn blur_wide(uv: vec2<f32>, sigma: f32) -> vec3<f32> {
    return mix(blur9(uv, sigma), blur9(uv, sigma * 2.4), 0.5);
}
