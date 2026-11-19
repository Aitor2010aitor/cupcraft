use crate::buffer::AudioBuffer;

/// A single peak/RMS slice for timeline rendering.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WaveformPoint {
    pub min: f32,
    pub max: f32,
}

/// Pre-computed waveform data for instant timeline visual rendering.
#[derive(Clone, Debug, Default)]
pub struct Waveform {
    pub points_per_second: usize,
    pub points: Vec<WaveformPoint>,
}

impl Waveform {
    /// Generates waveform peaks from an AudioBuffer. Default resolution: 100 points per second.
    pub fn from_buffer(buffer: &AudioBuffer, points_per_second: usize) -> Self {
        let pps = points_per_second.max(10);
        let frames = buffer.frame_count();
        if frames == 0 || buffer.sample_rate == 0 {
            return Self {
                points_per_second: pps,
                points: Vec::new(),
            };
        }

        let frames_per_point = (buffer.sample_rate as f64 / pps as f64).max(1.0) as usize;
        let num_points = (frames + frames_per_point - 1) / frames_per_point;
        let mut points = Vec::with_capacity(num_points);

        let ch = buffer.channels as usize;
        for i in 0..num_points {
            let start_frame = i * frames_per_point;
            let end_frame = (start_frame + frames_per_point).min(frames);

            let mut min_val = 0.0f32;
            let mut max_val = 0.0f32;

            for f in start_frame..end_frame {
                let sample_idx = f * ch;
                // Average or max over channels
                let mut sum = 0.0f32;
                for c in 0..ch {
                    if sample_idx + c < buffer.samples.len() {
                        sum += buffer.samples[sample_idx + c];
                    }
                }
                let val = sum / ch as f32;
                if val < min_val {
                    min_val = val;
                }
                if val > max_val {
                    max_val = val;
                }
            }

            points.push(WaveformPoint {
                min: min_val.clamp(-1.0, 1.0),
                max: max_val.clamp(-1.0, 1.0),
            });
        }

        Self {
            points_per_second: pps,
            points,
        }
    }

    /// Gets a slice of waveform points corresponding to the time range [start_sec, end_sec].
    pub fn points_in_range(&self, start_sec: f64, end_sec: f64) -> &[WaveformPoint] {
        if self.points.is_empty() || end_sec <= start_sec || start_sec < 0.0 {
            return &[];
        }
        let start_idx = ((start_sec * self.points_per_second as f64) as usize).min(self.points.len());
        let end_idx = ((end_sec * self.points_per_second as f64).ceil() as usize).min(self.points.len());

        if start_idx < end_idx {
            &self.points[start_idx..end_idx]
        } else {
            &[]
        }
    }
}
