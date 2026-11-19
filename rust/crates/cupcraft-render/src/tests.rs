//! Tests de compilación de shaders WGSL y del pipeline headless.

#[cfg(test)]
mod tests {
    use crate::{
        effect_map::{EffectId, effect_id_from_name, effect_id_from_capcut, shader_source},
        transition::TransitionKind,
    };

    // ---- Verificar que todos los shaders tienen source no vacío ----

    fn all_effect_ids() -> Vec<EffectId> {
        vec![
            EffectId::RadialBlur, EffectId::DirectionalBlur, EffectId::RgbSplit,
            EffectId::Chromatic, EffectId::Vhs, EffectId::ZoomPulse, EffectId::Pixelate,
            EffectId::Mirror, EffectId::Vignette, EffectId::Grain, EffectId::Posterize,
            EffectId::Wave, EffectId::Bloom,
            EffectId::BrumaBorgona, EffectId::AtardecerEnsueno, EffectId::BrilloBrumoso,
            EffectId::PixelNoir, EffectId::CarinoRetro, EffectId::SuenoCaqui,
            EffectId::JengibreSuave, EffectId::LienzoEmbrujado,
            EffectId::CapcutLut,
        ]
    }

    #[test]
    fn all_shaders_have_source() {
        for id in all_effect_ids() {
            let src = shader_source(&id);
            assert!(!src.is_empty(), "Shader source vacío para {:?}", id);
            assert!(src.contains("fn fs_main"), "Shader sin fs_main: {:?}", id);
        }
    }

    #[test]
    fn transition_shader_has_source() {
        let src = include_str!("shaders/transitions.wgsl");
        assert!(!src.is_empty());
        assert!(src.contains("fn fs_main"));
    }

    #[test]
    fn effect_id_from_name_roundtrip() {
        let pairs = [
            ("radial-blur", EffectId::RadialBlur),
            ("vhs", EffectId::Vhs),
            ("bloom", EffectId::Bloom),
            ("bruma-borgona", EffectId::BrumaBorgona),
            ("lienzo-embrujado", EffectId::LienzoEmbrujado),
        ];
        for (name, expected) in pairs {
            assert_eq!(effect_id_from_name(name), Some(expected), "fallo en '{}'", name);
        }
        assert_eq!(effect_id_from_name("no-existe"), None);
    }

    #[test]
    fn capcut_id_mapping() {
        assert_eq!(effect_id_from_capcut("7696747334054448391"), Some(EffectId::BrumaBorgona));
        assert_eq!(effect_id_from_capcut("7694537677269486868"), Some(EffectId::AtardecerEnsueno));
        assert_eq!(effect_id_from_capcut("0000000000000000000"), None);
    }

    #[test]
    fn transition_from_str_roundtrip() {
        let cases = [
            ("fade", TransitionKind::Fade),
            ("flash", TransitionKind::Flash),
            ("slide-left", TransitionKind::SlideLeft),
            ("push-right", TransitionKind::PushRight),
            ("drift-vhs", TransitionKind::DriftVhs),
            ("glitch-rgb", TransitionKind::GlitchRgb),
            ("kaleido", TransitionKind::Kaleido),
            ("no-existe", TransitionKind::Crossfade),
        ];
        for (s, expected) in cases {
            assert_eq!(TransitionKind::from_str(s), expected, "fallo en '{}'", s);
        }
    }

    #[test]
    fn all_transitions_have_u32() {
        // Verifica que ningún enum variant produce u32 duplicado en el rango básico
        let kinds = [
            TransitionKind::Fade, TransitionKind::Flash, TransitionKind::WipeLeft,
            TransitionKind::Iris, TransitionKind::DriftVhs, TransitionKind::GlitchRgb,
        ];
        let ids: Vec<u32> = kinds.iter().map(|k| k.as_u32()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "u32 duplicados en TransitionKind");
    }

    // ---- Test de GpuRenderer (solo si hay GPU disponible) ----

    #[test]
    #[ignore = "requiere GPU; ejecutar con: cargo test --ignored"]
    fn all_shaders_compile_on_gpu() {
        let renderer = crate::GpuRenderer::new().expect("No GPU");
        for id in all_effect_ids() {
            let src = shader_source(&id);
            let _module = renderer.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("test_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(src)),
            });
        }
        let tr_src = include_str!("shaders/transitions.wgsl");
        let _tr_mod = renderer.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("test_tr_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(tr_src)),
        });
        let layer_src = include_str!("shaders/layer.wgsl");
        let _layer_mod = renderer.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("test_layer_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(layer_src)),
        });
    }

    #[test]
    #[ignore = "requiere GPU; ejecutar con: cargo test --ignored"]
    fn gpu_renderer_apply_effect() {
        let renderer = crate::GpuRenderer::new().expect("No GPU");
        let w = 128u32;
        let h = 128u32;
        let frame: Vec<u8> = (0..w * h * 4).map(|i| ((i * 7) % 256) as u8).collect();
        let out = renderer.apply_effect(&frame, w, h, &EffectId::RadialBlur, 0.0, 0.5);
        assert_eq!(out.len(), (w * h * 4) as usize);
    }

    #[test]
    #[ignore = "requiere GPU; ejecutar con: cargo test --ignored"]
    fn gpu_renderer_blend_transition() {
        let renderer = crate::GpuRenderer::new().expect("No GPU");
        let w = 128u32;
        let h = 128u32;
        let a: Vec<u8> = vec![255u8; (w * h * 4) as usize];
        let b: Vec<u8> = vec![0u8; (w * h * 4) as usize];
        let out = renderer.blend_transition(&a, &b, w, h, TransitionKind::Fade, 0.5);
        assert_eq!(out.len(), (w * h * 4) as usize);
        // Con fade al 50%, cada canal debería estar cerca de 128
        let avg_r: u32 = out.iter().step_by(4).map(|&x| x as u32).sum::<u32>() / (w * h);
        assert!(avg_r > 100 && avg_r < 160, "avg_r inesperado: {}", avg_r);
    }
}
