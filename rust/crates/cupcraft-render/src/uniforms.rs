use bytemuck::{Pod, Zeroable};

/// Uniforms for full-frame post-processing shaders (blur, vhs, bloom, filters, etc.).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PostUniforms {
    pub time: f32,
    pub amount: f32,
    pub res_x: f32,
    pub res_y: f32,
}

impl Default for PostUniforms {
    fn default() -> Self {
        Self {
            time: 0.0,
            amount: 1.0,
            res_x: 1920.0,
            res_y: 1080.0,
        }
    }
}

/// Uniforms for individual clip layer rendering (transforms, color adjustment, crop).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LayerUniforms {
    /// 4x4 column-major affine model matrix (translate, rotate, scale, flip).
    pub model_matrix: [f32; 16],
    /// [brightness, contrast, saturate, hue_degrees]
    /// Defaults: [1.0, 1.0, 1.0, 0.0]
    pub color_adjust: [f32; 4],
    /// [grayscale, sepia, opacity, _padding]
    /// Defaults: [0.0, 0.0, 1.0, 0.0]
    pub extra_adjust: [f32; 4],
    /// [crop_left, crop_top, crop_right, crop_bottom] normalized 0..1
    pub crop: [f32; 4],
}

impl Default for LayerUniforms {
    fn default() -> Self {
        Self {
            model_matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            color_adjust: [1.0, 1.0, 1.0, 0.0],
            extra_adjust: [0.0, 0.0, 1.0, 0.0],
            crop: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

/// Uniforms for clip transition blending.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TransitionUniforms {
    /// Normalized progress 0.0 .. 1.0
    pub progress: f32,
    /// Numerical ID of the transition algorithm
    pub transition_type: u32,
    pub width: f32,
    pub height: f32,
}

impl Default for TransitionUniforms {
    fn default() -> Self {
        Self {
            progress: 0.0,
            transition_type: 0,
            width: 1920.0,
            height: 1080.0,
        }
    }
}
