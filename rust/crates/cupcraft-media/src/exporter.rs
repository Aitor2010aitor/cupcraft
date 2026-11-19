use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use cupcraft_audio::AudioBuffer;
use crate::ffmpeg::{FfmpegLocation, HwAccel};
use crate::probe::ProbeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Mp4H264,
    Mp4H265,
    WebmVp9,
}

pub struct VideoEncoderSession {
    child: Child,
    width: u32,
    height: u32,
    total_frames: u64,
    current_frame: u64,
}

impl VideoEncoderSession {
    /// Starts an encoding session by spawning FFmpeg with rawvideo RGBA on stdin.
    pub fn start<P: AsRef<Path>>(
        output_path: P,
        width: u32,
        height: u32,
        fps: u32,
        total_frames: u64,
        format: ExportFormat,
        use_gpu: bool,
    ) -> Result<Self, ProbeError> {
        let loc = FfmpegLocation::find().ok_or(ProbeError::FfmpegNotFound)?;
        let hw = if use_gpu {
            loc.detect_hardware_encoder()
        } else {
            HwAccel::Cpu
        };

        let mut cmd = Command::new(&loc.ffmpeg_path);
        cmd.args([
            "-y", // Overwrite output
            "-f", "rawvideo",
            "-pix_fmt", "rgba",
            "-s", &format!("{}x{}", width, height),
            "-r", &fps.to_string(),
            "-i", "pipe:0", // Read video from stdin
        ]);

        // Codec selection with HW acceleration
        match (format, hw) {
            (ExportFormat::Mp4H264, HwAccel::Nvenc) => {
                cmd.args(["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "20"]);
            }
            (ExportFormat::Mp4H264, HwAccel::Qsv) => {
                cmd.args(["-c:v", "h264_qsv", "-global_quality", "20"]);
            }
            (ExportFormat::Mp4H264, HwAccel::Amf) => {
                cmd.args(["-c:v", "h264_amf", "-quality", "speed"]);
            }
            (ExportFormat::Mp4H264, HwAccel::Cpu) => {
                cmd.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"]);
            }
            (ExportFormat::Mp4H265, HwAccel::Nvenc) => {
                cmd.args(["-c:v", "hevc_nvenc", "-preset", "p4", "-cq", "22"]);
            }
            (ExportFormat::Mp4H265, HwAccel::Qsv) => {
                cmd.args(["-c:v", "hevc_qsv", "-global_quality", "22"]);
            }
            (ExportFormat::Mp4H265, HwAccel::Amf) => {
                cmd.args(["-c:v", "hevc_amf"]);
            }
            (ExportFormat::Mp4H265, HwAccel::Cpu) => {
                cmd.args(["-c:v", "libx265", "-preset", "fast", "-crf", "24", "-pix_fmt", "yuv420p"]);
            }
            (ExportFormat::WebmVp9, _) => {
                cmd.args(["-c:v", "libvpx-vp9", "-crf", "28", "-b:v", "0", "-pix_fmt", "yuv420p"]);
            }
        }

        cmd.arg(output_path.as_ref());
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = cmd.spawn()?;
        Ok(Self {
            child,
            width,
            height,
            total_frames,
            current_frame: 0,
        })
    }

    /// Feeds a single RGBA frame into the encoder pipe.
    pub fn push_frame(&mut self, rgba: &[u8]) -> std::io::Result<f32> {
        let expected_size = (self.width * self.height * 4) as usize;
        let stdin = self.child.stdin.as_mut().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "FFmpeg stdin unavailable")
        })?;

        if rgba.len() >= expected_size {
            stdin.write_all(&rgba[..expected_size])?;
        } else {
            stdin.write_all(rgba)?;
            let pad = vec![0u8; expected_size - rgba.len()];
            stdin.write_all(&pad)?;
        }

        self.current_frame += 1;
        let progress = if self.total_frames > 0 {
            (self.current_frame as f32 / self.total_frames as f32).min(1.0)
        } else {
            1.0
        };

        Ok(progress)
    }

    /// Closes stdin and waits for FFmpeg to finish encoding.
    pub fn finish(mut self) -> Result<(), ProbeError> {
        drop(self.child.stdin.take());
        let status = self.child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(ProbeError::ParseError)
        }
    }
}

/// Muxes an existing video file with a separate audio buffer into a finalized video file.
pub fn mux_audio_video<P: AsRef<Path>>(
    video_only_path: P,
    audio: &AudioBuffer,
    output_path: P,
) -> Result<(), ProbeError> {
    let loc = FfmpegLocation::find().ok_or(ProbeError::FfmpegNotFound)?;

    // Write audio to temporary WAV file
    let temp_audio = std::env::temp_dir().join(format!("cupcraft_temp_audio_{}.wav", std::process::id()));
    write_wav(&temp_audio, audio)?;

    let status = Command::new(&loc.ffmpeg_path)
        .args([
            "-y",
            "-i",
        ])
        .arg(video_only_path.as_ref())
        .args([
            "-i",
        ])
        .arg(&temp_audio)
        .args([
            "-c:v", "copy",
            "-c:a", "aac",
            "-b:a", "192k",
            "-shortest",
        ])
        .arg(output_path.as_ref())
        .output()?;

    let _ = std::fs::remove_file(temp_audio);

    if status.status.success() {
        Ok(())
    } else {
        Err(ProbeError::ParseError)
    }
}

pub fn write_wav<P: AsRef<Path>>(path: P, audio: &AudioBuffer) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::BufWriter;

    let mut file = BufWriter::new(File::create(path)?);
    let num_samples = audio.samples.len() as u32;
    let num_channels = audio.channels;
    let sample_rate = audio.sample_rate;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = num_channels * (bits_per_sample / 8);
    let data_bytes = num_samples * 2;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size (16 for PCM)
    file.write_all(&1u16.to_le_bytes())?;  // AudioFormat (1 for PCM)
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;

    for &s in &audio.samples {
        let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    file.flush()
}
