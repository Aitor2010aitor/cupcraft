// transitions.wgsl — composición de transición entre dos fotogramas (entrante A y anterior B).
// El fragment shader produce el píxel final combinando A y B según el tipo y progreso.
//
// Tipos (u32) corresponden a TransitionKind en Rust:
//   0 = fade, 1 = flash, 2 = black, 3 = slide_left, 4 = slide_right, 5 = slide_up, 6 = slide_down,
//   7 = push_left, 8 = push_right, 9 = push_up, 10 = push_down, 11 = diagonal,
//   12 = zoom_in, 13 = zoom_out, 14 = spin, 15 = crosszoom, 16 = dolly,
//   17 = rotate_in, 18 = warp, 19 = pulse, 20 = glitch, 21 = film_burn, 22 = grain_tr,
//   23 = wipe_left, 24 = wipe_right, 25 = wipe_up, 26 = wipe_down,
//   27 = iris, 28 = barn, 29 = blinds, 30 = clock, 31 = split,
//   32 = curtain, 33 = blocks, 34 = vignette_tr, 35 = slam_hit,
//   36 = shake_punch, 37 = earthquake, 38 = whip_left, 39 = whip_right,
//   40 = zoom_snap, 41 = spin_slam, 42 = swirl_warp,
//   43 = heart_iris, 44 = star_iris, 45 = page_curl, 46 = kaleido,
//   47 = drift_voronoi, 48 = drift_swirl, 49 = drift_honeycomb,
//   50 = drift_vhs, 51 = drift_plasma, 52 = drift_liquid,
//   53 = drift_matrix, 54 = drift_ink, 55 = drift_interference,
//   56 = drift_bubbles, 57 = fire_burst, 58 = lightning_flash,
//   59 = lens_flare, 60 = strobe, 61 = smoke_cloud,
//   62 = glitch_rgb, 63 = pixelate_tr, 64 = rgb_pulse,
//   65 = swing, 66 = crossfade

struct TransitionUniforms {
    progress: f32,
    transition_type: u32,
    width: f32,
    height: f32,
}

@group(0) @binding(0) var<uniform> u: TransitionUniforms;
@group(0) @binding(1) var u_a: texture_2d<f32>;   // Frame incoming (nuevo clip)
@group(0) @binding(2) var u_b: texture_2d<f32>;   // Frame outgoing (clip anterior)
@group(0) @binding(3) var u_sampler: sampler;

struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) a_pos: vec2<f32>) -> VO {
    var o: VO; o.uv = a_pos * 0.5 + vec2<f32>(0.5); o.uv.y = 1.0 - o.uv.y;
    o.pos = vec4<f32>(a_pos, 0.0, 1.0); return o;
}

fn sa(uv: vec2<f32>) -> vec4<f32> { return textureSample(u_a, u_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))); }
fn sb(uv: vec2<f32>) -> vec4<f32> { return textureSample(u_b, u_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))); }

fn hash_f(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }
fn hash1(n: f32) -> f32 { return fract(sin(n) * 43758.5453); }
fn ease_out(t: f32) -> f32 { let u2 = 1.0 - t; return 1.0 - u2 * u2 * u2; }

@fragment fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let p = u.progress;      // 0..1
    let gap = 1.0 - p;
    let uv = in.uv;
    let tt = u.transition_type;

    // --- Fade / crossfade ---
    if (tt == 0u || tt == 66u) {
        return mix(sb(uv), sa(uv), p);
    }

    // --- Flash (white flash) ---
    if (tt == 1u) {
        let alpha = clamp(p * 1.8, 0.0, 1.0);
        let flash = clamp((1.0 - abs(p - 0.5) * 2.0) * 2.5, 0.0, 1.0);
        return mix(mix(sb(uv), sa(uv), alpha), vec4<f32>(1.0), flash * 0.85);
    }

    // --- Black flash ---
    if (tt == 2u) {
        let flash = 1.0 - abs(p - 0.5) * 2.0;
        return mix(mix(sb(uv), sa(uv), p), vec4<f32>(0.0, 0.0, 0.0, 1.0), flash);
    }

    // --- Slide left (A entrante desde derecha) ---
    if (tt == 3u) {
        let uv_a = uv + vec2<f32>(gap, 0.0);
        return select(sb(uv), sa(uv_a), uv_a.x <= 1.0 && uv_a.x >= 0.0);
    }
    // --- Slide right ---
    if (tt == 4u) {
        let uv_a = uv - vec2<f32>(p, 0.0);
        return select(sb(uv), sa(uv_a), uv_a.x >= 0.0 && uv_a.x <= 1.0);
    }
    // --- Slide up ---
    if (tt == 5u) {
        let uv_a = uv + vec2<f32>(0.0, gap);
        return select(sb(uv), sa(uv_a), uv_a.y <= 1.0 && uv_a.y >= 0.0);
    }
    // --- Slide down ---
    if (tt == 6u) {
        let uv_a = uv - vec2<f32>(0.0, p);
        return select(sb(uv), sa(uv_a), uv_a.y >= 0.0 && uv_a.y <= 1.0);
    }

    // --- Push left (B sale hacia izquierda, A entra desde derecha) ---
    if (tt == 7u) {
        let uv_a = uv + vec2<f32>(gap, 0.0);
        let uv_b = uv - vec2<f32>(p, 0.0);
        if (uv.x >= p) { return sa(uv_a); } else { return sb(uv_b); }
    }
    // --- Push right ---
    if (tt == 8u) {
        let uv_a = uv - vec2<f32>(p, 0.0);
        let uv_b = uv + vec2<f32>(p, 0.0);
        if (uv.x <= gap) { return sa(uv_a); } else { return sb(uv_b); }
    }
    // --- Push up ---
    if (tt == 9u) {
        let uv_a = uv + vec2<f32>(0.0, gap);
        let uv_b = uv - vec2<f32>(0.0, p);
        if (uv.y >= p) { return sa(uv_a); } else { return sb(uv_b); }
    }
    // --- Push down ---
    if (tt == 10u) {
        let uv_a = uv - vec2<f32>(0.0, p);
        let uv_b = uv + vec2<f32>(0.0, p);
        if (uv.y <= gap) { return sa(uv_a); } else { return sb(uv_b); }
    }

    // --- Diagonal ---
    if (tt == 11u) {
        let uv_a = uv + vec2<f32>(gap, gap);
        return select(sb(uv), sa(uv_a), uv_a.x <= 1.0 && uv_a.y <= 1.0);
    }

    // --- Zoom in (A crece sobre B) ---
    if (tt == 12u) {
        let scale = 1.0 / (1.0 + gap * 0.34);
        let uv_a = (uv - 0.5) * scale + 0.5;
        return mix(sb(uv), sa(uv_a), clamp(p * 1.4 - 0.1, 0.0, 1.0));
    }
    // --- Zoom out ---
    if (tt == 13u) {
        let scale = 0.68 + p * 0.32;
        let uv_a = (uv - 0.5) / scale + 0.5;
        return mix(sb(uv), sa(uv_a), clamp(p * 1.4 - 0.1, 0.0, 1.0));
    }

    // --- Crosszoom ---
    if (tt == 15u) {
        let scale_a = 1.0 / (1.0 + gap * 1.1);
        let uv_a = (uv - 0.5) * scale_a + 0.5;
        return mix(sb(uv), sa(uv_a), clamp(0.15 + p * 1.6, 0.0, 1.0));
    }

    // --- Wipe left ---
    if (tt == 23u) {
        return select(sb(uv), sa(uv), uv.x < ease_out(p));
    }
    // --- Wipe right ---
    if (tt == 24u) {
        return select(sb(uv), sa(uv), uv.x > 1.0 - ease_out(p));
    }
    // --- Wipe up ---
    if (tt == 25u) {
        return select(sb(uv), sa(uv), uv.y > 1.0 - ease_out(p));
    }
    // --- Wipe down ---
    if (tt == 26u) {
        return select(sb(uv), sa(uv), uv.y < ease_out(p));
    }

    // --- Iris (círculo expandiéndose) ---
    if (tt == 27u) {
        let d = distance(uv, vec2<f32>(0.5));
        let radius = 0.75 * ease_out(p);
        return select(sb(uv), sa(uv), d < radius);
    }

    // --- Barn (dos mitades abriendo hacia el centro) ---
    if (tt == 28u) {
        let half = ease_out(p) * 0.5;
        return select(sb(uv), sa(uv), uv.x < half || uv.x > 1.0 - half);
    }

    // --- Blinds (8 barras horizontales) ---
    if (tt == 29u) {
        let rows = 8.0;
        let row_h = 1.0 / rows;
        let row_frac = fract(uv.y / row_h);
        return select(sb(uv), sa(uv), row_frac < ease_out(p));
    }

    // --- Vignette transition ---
    if (tt == 34u) {
        let d = distance(uv, vec2<f32>(0.5)) * 1.42;
        let mask = smoothstep(ease_out(p) * 1.4, ease_out(p) * 1.4 - 0.3, d);
        return mix(sb(uv), sa(uv), mask);
    }

    // --- Drift VHS ---
    if (tt == 50u) {
        let shift = sin(uv.y * 60.0 + p * 15.0) * gap * 0.025;
        let uv_a = vec2<f32>(uv.x + shift, uv.y);
        return mix(sb(uv), sa(uv_a), clamp(p + shift * 4.0, 0.0, 1.0));
    }

    // --- Pixelate transition ---
    if (tt == 63u) {
        let size2 = max(2.0, gap * 48.0 + 2.0);
        let pix_uv = floor(uv * size2 + 0.5) / size2;
        return mix(sb(pix_uv), sa(pix_uv), p);
    }

    // --- Drift plasma ---
    if (tt == 51u) {
        let wave = sin(uv.y * 20.0 + p * 10.0) * gap * 0.04;
        return mix(sb(uv), sa(uv + vec2<f32>(wave, 0.0)), clamp(p * 1.3, 0.0, 1.0));
    }

    // --- Glitch RGB ---
    if (tt == 62u) {
        let shift = (hash1(floor(uv.y * 32.0) + p * 7.0) - 0.5) * gap * 0.08;
        let col_r = sa(uv + vec2<f32>(shift * 1.5, 0.0)).r;
        let col_g = sa(uv).g;
        let col_b = sa(uv - vec2<f32>(shift * 1.5, 0.0)).b;
        return mix(sb(uv), vec4<f32>(col_r, col_g, col_b, 1.0), clamp(p * 1.5, 0.0, 1.0));
    }

    // --- Default: crossfade ---
    return mix(sb(uv), sa(uv), p);
}
