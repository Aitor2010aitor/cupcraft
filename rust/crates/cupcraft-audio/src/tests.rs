#[cfg(test)]
mod tests {
    use crate::buffer::AudioBuffer;
    use crate::mixer::{clip_gain, AudioMixer};
    use crate::waveform::Waveform;
    use cupcraft_core::model::{ClipKind, Project, Track, TrackType};
    use cupcraft_core::timeline::make_clip;

    #[test]
    fn audio_buffer_sample_at_interpolation() {
        let buf = AudioBuffer::new(2, 2, vec![0.0, 0.0, 1.0, 0.5]);
        assert_eq!(buf.duration(), 1.0);

        let (l0, r0) = buf.sample_at(0.0);
        assert!((l0 - 0.0).abs() < 1e-4);
        assert!((r0 - 0.0).abs() < 1e-4);

        let (l_half, r_half) = buf.sample_at(0.25);
        assert!((l_half - 0.5).abs() < 1e-4);
        assert!((r_half - 0.25).abs() < 1e-4);
    }

    #[test]
    fn clip_gain_fades_and_volume() {
        let mut clip = make_clip(ClipKind::Audio, "c1", 0.0, 10.0);
        clip.volume = 0.8;
        clip.fade_in = 2.0;
        clip.fade_out = 3.0;

        assert!((clip_gain(&clip, 0.0) - 0.0).abs() < 1e-4);
        assert!((clip_gain(&clip, 1.0) - 0.4).abs() < 1e-4);
        assert!((clip_gain(&clip, 5.0) - 0.8).abs() < 1e-4);
        assert!((clip_gain(&clip, 8.5) - 0.4).abs() < 1e-4);
        assert!((clip_gain(&clip, 10.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn waveform_generation() {
        let mut samples = Vec::new();
        for i in 0..1000 {
            let val = if i % 2 == 0 { 0.8f32 } else { -0.8f32 };
            samples.push(val);
        }
        let buf = AudioBuffer::new(1000, 1, samples);
        let wf = Waveform::from_buffer(&buf, 10);
        assert_eq!(wf.points.len(), 10);
        assert!(wf.points[0].min <= -0.7);
        assert!(wf.points[0].max >= 0.7);
    }

    #[test]
    fn mixer_multi_track_mixes_accurately() {
        let mut mixer = AudioMixer::new();

        let buf1 = AudioBuffer::new(100, 2, vec![0.5; 200]);
        mixer.insert_media("m1".to_string(), buf1);

        let buf2 = AudioBuffer::new(100, 2, vec![0.2; 200]);
        mixer.insert_media("m2".to_string(), buf2);

        let mut p = Project::new("test");
        p.tracks.clear();

        let mut c1 = make_clip(ClipKind::Audio, "c1", 0.0, 1.0);
        c1.media_id = Some("m1".to_string());
        let t1 = Track {
            id: "t1".into(),
            kind: TrackType::Audio,
            name: "Audio 1".into(),
            muted: false,
            hidden: false,
            locked: false,
            clips: vec![c1],
        };

        let mut c2 = make_clip(ClipKind::Audio, "c2", 0.0, 1.0);
        c2.kind = ClipKind::Audio;
        c2.media_id = Some("m2".to_string());
        let t2 = Track {
            id: "t2".into(),
            kind: TrackType::Audio,
            name: "Audio 2".into(),
            muted: false,
            hidden: false,
            locked: false,
            clips: vec![c2],
        };

        p.tracks.push(t1);
        p.tracks.push(t2);

        let (l, r) = mixer.sample_at(&p, 0.5);
        assert!((l - 0.7).abs() < 1e-3);
        assert!((r - 0.7).abs() < 1e-3);
    }
}
