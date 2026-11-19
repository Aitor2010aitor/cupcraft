use std::path::Path;
use std::process::{Command, Stdio};
use crate::ffmpeg::FfmpegLocation;
use crate::probe::ProbeError;

/// Extracts a single video frame at timestamp `time_sec` as raw RGBA8 pixels.
pub fn extract_frame<P: AsRef<Path>>(
    video_path: P,
    time_sec: f64,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, ProbeError> {
    let loc = FfmpegLocation::find().ok_or(ProbeError::FfmpegNotFound)?;

    let scale_arg = format!("scale={}:{}", width, height);
    let time_str = format!("{:.3}", time_sec.max(0.0));

    let output = Command::new(&loc.ffmpeg_path)
        .args([
            "-ss", &time_str,
            "-i",
        ])
        .arg(video_path.as_ref())
        .args([
            "-vframes", "1",
            "-vf", &scale_arg,
            "-f", "rawvideo",
            "-pix_fmt", "rgba",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    let expected_len = (width * height * 4) as usize;
    if output.stdout.len() >= expected_len {
        Ok(output.stdout[..expected_len].to_vec())
    } else {
        // Return blank frame if decoding failed
        Ok(vec![0u8; expected_len])
    }
}

/// Extracts a series of evenly spaced thumbnail frames for the timeline filmstrip.
pub fn extract_filmstrip<P: AsRef<Path>>(
    video_path: P,
    duration_sec: f64,
    count: usize,
    width: u32,
    height: u32,
) -> Vec<(f64, Vec<u8>)> {
    if count == 0 || duration_sec <= 0.0 {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(count);
    let step = duration_sec / count as f64;

    for i in 0..count {
        let t = (i as f64 + 0.5) * step;
        let frame = extract_frame(&video_path, t, width, height).unwrap_or_else(|_| {
            vec![0u8; (width * height * 4) as usize]
        });
        result.push((t, frame));
    }

    result
}
