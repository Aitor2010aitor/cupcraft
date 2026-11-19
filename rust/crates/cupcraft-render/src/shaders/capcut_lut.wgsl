// capcut-lut: cubo 64³ en textura 512×512 (8×8 celdas).
// Puerto exacto del pass6.frag de los paquetes de CapCut.

struct PostUniforms { time: f32, amount: f32, res_x: f32, res_y: f32 }
@group(0) @binding(0) var<uniform> u: PostUniforms;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
@group(0) @binding(3) var u_lut: texture_2d<f32>;
@group(0) @binding(4) var u_lut_sampler: sampler;

struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}

fn take_filter(input_color: vec4<f32>, alpha: f32) -> vec4<f32> {
    let tc = input_color;
    let blue_color = tc.b * 63.0;
    var quad1: vec2<f32>;
    quad1.y = floor(floor(blue_color) / 8.0);
    quad1.x = floor(blue_color) - (quad1.y * 8.0);
    var quad2: vec2<f32>;
    quad2.y = floor(ceil(blue_color) / 8.0);
    quad2.x = ceil(blue_color) - (quad2.y * 8.0);
    var tex1: vec2<f32>;
    tex1.x = (quad1.x / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * tc.r);
    tex1.y = (quad1.y / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * tc.g);
    var tex2: vec2<f32>;
    tex2.x = (quad2.x / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * tc.r);
    tex2.y = (quad2.y / 8.0) + 0.5 / 512.0 + ((1.0 / 8.0 - 1.0 / 512.0) * tc.g);
    let new1 = textureSample(u_lut, u_lut_sampler, tex1);
    let new2 = textureSample(u_lut, u_lut_sampler, tex2);
    let new_color = mix(new1, new2, fract(blue_color));
    return mix(tc, vec4<f32>(new_color.rgb, tc.w), alpha);
}

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let cur = textureSample(u_tex, u_sampler, in.uv);
    var result = take_filter(cur, u.amount);
    result = vec4<f32>(mix(cur.rgb, result.rgb, cur.a), cur.a);
    return result;
}
