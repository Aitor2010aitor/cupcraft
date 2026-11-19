/// In-memory representation of decoded audio data.
#[derive(Clone, Debug)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved audio samples in [-1.0, 1.0] range.
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels: channels.max(1),
            samples,
        }
    }

    /// Creates a buffer with silence.
    pub fn silent(duration_sec: f64, sample_rate: u32, channels: u16) -> Self {
        let total_frames = (duration_sec * sample_rate as f64) as usize;
        let total_samples = total_frames * channels as usize;
        Self {
            sample_rate,
            channels: channels.max(1),
            samples: vec![0.0; total_samples],
        }
    }

    /// Duration in seconds.
    pub fn duration(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        (self.samples.len() / self.channels as usize) as f64 / self.sample_rate as f64
    }

    /// Number of audio frames (pairs of samples if stereo).
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    /// Samples the audio at an exact fractional second with linear interpolation.
    /// Returns `(left, right)` in range `[-1.0, 1.0]`.
    pub fn sample_at(&self, time_sec: f64) -> (f32, f32) {
        if time_sec < 0.0 || self.samples.is_empty() || self.sample_rate == 0 {
            return (0.0, 0.0);
        }

        let frame_pos = time_sec * self.sample_rate as f64;
        let frame_idx = frame_pos.floor() as usize;
        let frac = (frame_pos - frame_pos.floor()) as f32;

        let frames = self.frame_count();
        if frame_idx >= frames {
            return (0.0, 0.0);
        }

        let ch = self.channels as usize;
        let next_idx = (frame_idx + 1).min(frames - 1);

        if ch == 1 {
            let s0 = self.samples[frame_idx];
            let s1 = self.samples[next_idx];
            let s = s0 + (s1 - s0) * frac;
            (s, s)
        } else {
            let idx0 = frame_idx * ch;
            let idx1 = next_idx * ch;

            let l0 = self.samples[idx0];
            let r0 = self.samples[idx0 + 1];
            let l1 = self.samples[idx1];
            let r1 = self.samples[idx1 + 1];

            (l0 + (l1 - l0) * frac, r0 + (r1 - r0) * frac)
        }
    }
}
