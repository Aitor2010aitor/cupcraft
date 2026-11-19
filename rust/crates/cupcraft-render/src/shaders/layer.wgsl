struct LayerUniforms {
    model_matrix: mat4x4<f32>,
    color_adjust: vec4<f32>, // brightness, contrast, saturate, hue
    extra_adjust: vec4<f32>, // grayscale, sepia, opacity, blend_mode
    crop: vec4<f32>,         // left, top, right, bottom
};

@group(0) @binding(0) var<uniform> layer: LayerUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = layer.model_matrix * vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = select(vec4<f32>(c.gb, K.xy), vec4<f32>(c.bg, K.wz), c.b < c.g);
    let q = select(vec4<f32>(c.r, p.yzx), vec4<f32>(p.xyw, c.r), p.x < c.r);
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.uv.x < layer.crop.x || in.uv.x > (1.0 - layer.crop.z) ||
        in.uv.y < layer.crop.y || in.uv.y > (1.0 - layer.crop.w)) {
        discard;
    }

    var color = textureSample(u_tex, u_sampler, in.uv);
    if (color.a <= 0.0) {
        discard;
    }

    var rgb = color.rgb;

    let brightness = layer.color_adjust.x;
    rgb = rgb * brightness;

    let contrast = layer.color_adjust.y;
    rgb = (rgb - vec3<f32>(0.5)) * contrast + vec3<f32>(0.5);

    let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    let saturate_val = layer.color_adjust.z;
    rgb = mix(vec3<f32>(luma), rgb, saturate_val);

    let hue_deg = layer.color_adjust.w;
    if (abs(hue_deg) > 0.01) {
        var hsv = rgb_to_hsv(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
        hsv.x = fract(hsv.x + hue_deg / 360.0);
        rgb = hsv_to_rgb(hsv);
    }

    let gray_amt = layer.extra_adjust.x;
    if (gray_amt > 0.0) {
        let g = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
        rgb = mix(rgb, vec3<f32>(g), gray_amt);
    }

    let sepia_amt = layer.extra_adjust.y;
    if (sepia_amt > 0.0) {
        let s_r = dot(rgb, vec3<f32>(0.393, 0.769, 0.189));
        let s_g = dot(rgb, vec3<f32>(0.349, 0.686, 0.168));
        let s_b = dot(rgb, vec3<f32>(0.272, 0.534, 0.131));
        rgb = mix(rgb, vec3<f32>(s_r, s_g, s_b), sepia_amt);
    }

    let opacity = layer.extra_adjust.z;
    let final_alpha = color.a * opacity;

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), final_alpha);
}
