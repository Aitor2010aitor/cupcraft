//! Tabla id de efecto → operación del motor (`fx_engine`).
//! Cada fila: id, operación, tipo de segundo parámetro.

use crate::fx_engine::Op;

/// Segundo parámetro que expone la interfaz.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Second {
    /// Solo cantidad.
    None,
    /// "Detalle" 1..20 (default 8).
    Detail,
    /// Ángulo 0..360.
    Angle,
}

use Op::*;
use Second::{Angle as An, Detail as De, None as No};

pub const TABLE: &[(&str, Op, Second)] = &[
    // Color
    ("col_solid", SolidTint, De), ("col_gradover", GradOver, De), ("col_grad4", Grad4, No),
    ("col_gradmap", GradMap, No), ("col_palette", Palette, No), ("col_dither", Dither, De),
    ("col_spot", SpotColor, De), ("col_replace", ReplaceColor, De), ("col_remaprgb", RemapRgb, De),
    ("col_remaphsv", RemapHsv, De), ("col_mixer", Mixer, No), ("col_difference", Difference, De),
    ("col_darkglow", DarkGlow, No), ("col_hotcolor", HotColor, No), ("col_spectral", SpectralMap, De),
    // Desenfoque
    ("blr_inner", InnerBlur, No), ("blr_directional", Directional, An), ("blr_motion", Motion, An),
    ("blr_smear", Smear, An), ("blr_warp", WarpBlur, No), ("blr_zoom", ZoomBlur, No),
    ("blr_chromzoom", ChromZoom, No), ("blr_zoomstreak", ZoomStreak, No), ("blr_spinstreak", SpinStreak, No),
    ("blr_linstreak", LinStreak, An), ("blr_spin", SpinBlur, No), ("blr_vortex", VortexBlur, No),
    ("blr_mask", MaskBlur, No), ("blr_fillbehind", FillBehind, No),
    ("blr_streakstrips", StreakStrips, An),
    // Distorsión
    ("dst_puppet", Puppet, No), ("dst_innerpinch", InnerPinch, No), ("dst_glitch", Glitch, De),
    ("dst_noisedisp", NoiseDisp, De), ("dst_turbdisp", TurbDisp, De), ("dst_fractal", FractalWarp, De),
    ("dst_tiles", Tiles, De), ("dst_hextile", HexTiles, De), ("dst_tilerot", TileRot, De),
    ("dst_tileshift", TileShift, De), ("dst_hextilerot", HexRot, De), ("dst_hextileshift", HexShift, De),
    ("dst_squeeze", Squeeze, No), ("dst_360view", View360, De), ("dst_360reorient", Reorient, De),
    ("dst_stretchseg", StretchSeg, No), ("dst_bend", Bend, No), ("dst_curl", Curl, No),
    ("dst_magnify", Magnify, No), ("dst_glass", Glass, De), ("dst_dispmap", DispMap, No),
    ("dst_polardispmap", PolarDisp, No), ("dst_ominoglass", OminoGlass, De), ("dst_liquidglass", LiquidGlass, De),
    ("dst_pagecurl", PageCurl, No),
    // Estilizar
    ("sty_cartoon", Cartoon, De), ("sty_outline", Outline, No), ("sty_innerglow", InnerGlow, No),
    ("sty_edgeglow", EdgeGlow, No), ("sty_contourlines", ContourLines, De), ("sty_contourgrad", ContourGrad, No),
    ("sty_electricedges", ElectricEdges, No), ("sty_contourstrips", ContourStrips, De), ("sty_glowscan", GlowScan, De),
    ("sty_ribbon", Ribbon, No), ("sty_edgedetect", FindEdges, No), ("sty_emboss", Emboss, De),
    ("sty_smoothbevel", Bevel, No), ("sty_bumpmap", BumpMap, No), ("sty_roughen", Roughen, De),
    ("sty_halftone", HalftoneDots, De), ("sty_halftonelines", HalftoneLines, De), ("sty_cmykdots", CmykDots, De),
    ("sty_ascii", Ascii, De), ("sty_lightflare", LensFlare, No), ("sty_longshadow", LongShadow, No),
    ("sty_dropshadow", DropShadow, No), ("sty_radialshadow", RadialShadow, No), ("sty_pixeltiles", PixelTiles, De),
    ("sty_rgbsplit", RgbSplit, An), ("sty_scanlines", Scanlines, De), ("sty_stripes", Stripes, De),
    ("sty_voronoi", Voronoi, De), ("sty_star", StarMask, No), ("sty_checker", Checker, De),
    ("sty_grid", GridOv, De), ("sty_dots", DotsOv, De), ("sty_radialrays", RadialRays, De),
    ("sty_lightning", Lightning, De), ("sty_energybeam", EnergyBeam, De), ("sty_iridescence", Iridescence, De),
    ("sty_rays", GodRays, De), ("sty_starfield", Starfield, De), ("sty_simplestarfield", StarfieldSimple, De),
    ("sty_particles", Particles, De), ("sty_sharpen", Sharpen, No), ("sty_vignette", Vignette, No),
    ("sty_grunge", Grunge, No), ("sty_blink", Blink, No), ("sty_unsharp", Unsharp, No),
    ("sty_flicker", Flicker, De), ("sty_datamosh", Datamosh, De), ("sty_drawprogress", DrawProgress, No),
    ("sty_stroketaper", StrokeTaper, No), ("sty_ominodiff", OminoDiff, De), ("sty_hexarray", HexArray, De),
    ("sty_fade", Fade, No), ("sty_strokecolor", StrokeColor, De),
    // Ruido
    ("noi_filmgrain", FilmGrain, De), ("noi_turbulence", Turbulence, De), ("noi_clouds", Clouds, De),
    ("noi_fractalridges", FractalRidges, De), ("noi_dissolve", Dissolve, De), ("noi_blockdissolve", BlockDissolve, De),
    ("noi_blocknoise", BlockNoise, De), ("noi_noise", NoiseOv, De), ("noi_static", StaticTv, De),
    // Repetir
    ("rep_tile", RepTile, De), ("rep_repeat", Tiles, De), ("rep_linear", RepLinear, De),
    ("rep_radial", RepRadial, De), ("rep_scatter", RepScatter, De), ("rep_grid", Tiles, De),
    ("rep_alongpath", RepPath, De), ("rep_scatterfield", RepField, De),
    ("rep_feedback", Feedback, De),
    // Croma
    ("key_chroma", Chroma, De), ("key_spill", Spill, De), ("key_luma", LumaKey, No),
    ("key_diffmatte", DiffMatte, No), ("key_lumastamp", LumaStamp, No), ("key_solidmatte", SolidMatte, De),
    ("key_crop", Crop, No), ("key_wipe", Wipe, An), ("key_feather", Feather, No),
    ("key_radialwipe", RadialWipe, No), ("key_choker", Choker, No), ("key_fringe", Fringe, De),
    ("key_smoothedges", SmoothEdges, No), ("key_advchroma", AdvChroma, De),
    // Movimiento
    ("mov_autoshake", Shake, De), ("mov_randdisp", Shake, De), ("mov_randjitter", Jitter, De),
    ("mov_oscillate", Oscillate, De), ("mov_swing", Swing, No), ("mov_spin", Spin, No),
    ("mov_pulsesize", PulseSize, No), ("mov_stretchaxis", StretchAxis, De), ("mov_offset", Offset, De),
    ("mov_rastertransform", Raster, No), ("mov_alongpath", MovePath, No), ("mov_scaleassist", ScaleAssist, De),
    ("mov_truemotionblur", Motion, An), ("mov_pulseopacity", PulseOpacity, De), ("mov_echokeys", Echo, De),
    // Ritmo
    ("bet_zoom", BeatZoom, No), ("bet_shake", Shake, De), ("bet_flash", Flash, No),
    ("bet_blur", Blur, No), ("bet_glow", Glow, No), ("bet_rgbsplit", RgbSplit, An),
    ("bet_pulse", PulseSize, No), ("bet_rotate", Spin, No), ("bet_glitch", Glitch, De),
    // 3D
    ("d3_cube", Cube, De), ("d3_box", Box, De), ("d3_cylinder", Cylinder, De),
    ("d3_ellipsoid", Ellipsoid, De), ("d3_torus", Torus, De), ("d3_pyramid", Pyramid, De),
    ("d3_octahedron", Octahedron, De), ("d3_hexprism", HexPrism, De), ("d3_fliplayer", FlipLayer, No),
    ("d3_spherize", Spherize, No), ("d3_tunnel", Tunnel, De),
    ("d3_scatter", Scatter3, De), ("d3_fracture", Fracture, De), ("d3_hollowbox", HollowBox, De),
    ("d3_starprism", StarPrism, De), ("d3_starpoly", StarPoly, De), ("d3_heart", Heart, De),
    ("d3_cross", Cross3, De), ("d3_ring", Ring, De), ("d3_rasterextrude", RasterExtrude, No),
];

pub fn index_of(id: &str) -> Option<u16> {
    TABLE.iter().position(|r| r.0 == id).map(|i| i as u16)
}

pub fn op_at(i: u16) -> Option<Op> {
    TABLE.get(i as usize).map(|r| r.1)
}

pub fn second_of(id: &str) -> Option<Second> {
    TABLE.iter().find(|r| r.0 == id).map(|r| r.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todas_las_operaciones_corren_y_cambian_el_fotograma() {
        let (w, h) = (96usize, 64usize);
        let mut src = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                src[i] = (x * 255 / w) as u8;
                src[i + 1] = (y * 255 / h) as u8;
                src[i + 2] = ((x / 8 + y / 8) % 2 * 200) as u8;
                src[i + 3] = 255;
            }
        }
        let mut same = vec![];
        for (id, op, sec) in TABLE {
            for d in [1.0f32, 8.0, if *sec == Second::Angle { 90.0 } else { 20.0 }] {
                let out = crate::fx_engine::run(*op, &src, w, h, 0.7, d);
                assert_eq!(out.len(), src.len(), "{id}");
                if d == 8.0 && out == src && !matches!(*id, "key_solidmatte" | "key_feather" | "key_choker" | "key_smoothedges") {
                    same.push(*id);
                }
            }
        }
        assert!(same.is_empty(), "sin efecto visible: {same:?}");
    }
}
