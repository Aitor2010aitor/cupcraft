use std::collections::HashMap;
use cupcraft_core::model::{Clip, ClipKind, Project};
use crate::buffer::AudioBuffer;

fn clamp01(v: f32) -> f32 {
    v.max(0.0).min(1.0)
}

/// Computes the effective volume gain for a clip at a local time offset (including fadeIn / fadeOut).
pub fn clip_gain(clip: &Clip, local_sec: f64) -> f32 {
    let mut gain = clip.volume as f32;
    let fade_in = clip.fade_in as f32;
    let local = local_sec as f32;

    if fade_in > 0.0 && local < fade_in {
        gain *= clamp01(local / fade_in);
    }

    let fade_out = clip.fade_out as f32;
    let duration = clip.duration as f32;
    let remaining = duration - local;

    if fade_out > 0.0 && remaining < fade_out {
        gain *= clamp01(remaining / fade_out);
    }

    gain.max(0.0)
}

/// Multi-track timeline audio mixer.
pub struct AudioMixer {
    pub media: HashMap<String, AudioBuffer>,
    pub master_volume: f32,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioMixer {
    pub fn new() -> Self {
        Self {
            media: HashMap::new(),
            master_volume: 1.0,
        }
    }

    pub fn insert_media(&mut self, media_id: String, buffer: AudioBuffer) {
        self.media.insert(media_id, buffer);
    }

    pub fn remove_media(&mut self, media_id: &str) {
        self.media.remove(media_id);
    }

    /// Evaluates mixed stereo sample `(left, right)` at timeline timestamp `time_sec`.
    pub fn sample_at(&self, project: &Project, time_sec: f64) -> (f32, f32) {
        if time_sec < 0.0 {
            return (0.0, 0.0);
        }

        let mut mix_l = 0.0f32;
        let mut mix_r = 0.0f32;

        for track in &project.tracks {
            if track.muted || track.hidden {
                continue;
            }

            for clip in &track.clips {
                // Audio is produced by Audio and Video clips
                if clip.kind != ClipKind::Audio && clip.kind != ClipKind::Video {
                    continue;
                }

                if time_sec < clip.start || time_sec >= clip.start + clip.duration {
                    continue;
                }

                let media_id = match &clip.media_id {
                    Some(id) => id,
                    None => continue,
                };

                let buffer = match self.media.get(media_id) {
                    Some(b) => b,
                    None => continue,
                };

                let local = time_sec - clip.start;
                let speed = clip.speed.max(0.0001);
                let source_time = clip.in_point + local * speed;

                let (s_l, s_r) = buffer.sample_at(source_time);
                let gain = clip_gain(clip, local);

                mix_l += s_l * gain;
                mix_r += s_r * gain;
            }
        }

        let out_l = (mix_l * self.master_volume).clamp(-1.0, 1.0);
        let out_r = (mix_r * self.master_volume).clamp(-1.0, 1.0);
        (out_l, out_r)
    }

    /// Renders an entire project's audio mix into an in-memory `AudioBuffer`
    /// for exporting to disk or encoding with FFmpeg.
    pub fn render_project_audio(
        &self,
        project: &Project,
        sample_rate: u32,
    ) -> AudioBuffer {
        let duration = project.duration();
        let total_frames = (duration * sample_rate as f64).ceil() as usize;
        let mut samples = Vec::with_capacity(total_frames * 2);

        let dt = 1.0 / sample_rate as f64;
        for frame in 0..total_frames {
            let t = frame as f64 * dt;
            let (l, r) = self.sample_at(project, t);
            samples.push(l);
            samples.push(r);
        }

        AudioBuffer::new(sample_rate, 2, samples)
    }
}
