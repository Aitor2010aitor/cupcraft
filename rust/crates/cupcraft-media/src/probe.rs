use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use crate::ffmpeg::FfmpegLocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub has_video: bool,
    pub has_audio: bool,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ProbeError {
    #[error("FFmpeg not found")]
    FfmpegNotFound,
    #[error("Failed to run ffprobe: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse media metadata")]
    ParseError,
}

pub fn probe_media<P: AsRef<Path>>(path: P) -> Result<MediaInfo, ProbeError> {
    let loc = FfmpegLocation::find().ok_or(ProbeError::FfmpegNotFound)?;
    let p = path.as_ref();

    if let Some(ffprobe) = &loc.ffprobe_path {
        let output = Command::new(ffprobe)
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(p)
            .output()?;

        if output.status.success() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                let streams = json.get("streams").and_then(|s| s.as_array());
                let format = json.get("format");

                let mut duration = format
                    .and_then(|f| f.get("duration"))
                    .and_then(|d| d.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let mut width = 0u32;
                let mut height = 0u32;
                let mut fps = 30.0;
                let mut has_video = false;
                let mut has_audio = false;
                let mut video_codec = None;
                let mut audio_codec = None;

                if let Some(arr) = streams {
                    for s in arr {
                        let codec_type = s.get("codec_type").and_then(|t| t.as_str()).unwrap_or("");
                        let codec_name = s.get("codec_name").and_then(|n| n.as_str()).map(|n| n.to_string());

                        if codec_type == "video" && !has_video {
                            has_video = true;
                            video_codec = codec_name;
                            width = s.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
                            height = s.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;

                            if let Some(r_frame_rate) = s.get("r_frame_rate").and_then(|r| r.as_str()) {
                                if let Some((num, den)) = r_frame_rate.split_once('/') {
                                    if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                                        if d > 0.0 {
                                            fps = n / d;
                                        }
                                    }
                                }
                            }

                            if duration == 0.0 {
                                if let Some(d) = s.get("duration").and_then(|d| d.as_str()).and_then(|s| s.parse::<f64>().ok()) {
                                    duration = d;
                                }
                            }
                        } else if codec_type == "audio" && !has_audio {
                            has_audio = true;
                            audio_codec = codec_name;
                            if duration == 0.0 {
                                if let Some(d) = s.get("duration").and_then(|d| d.as_str()).and_then(|s| s.parse::<f64>().ok()) {
                                    duration = d;
                                }
                            }
                        }
                    }
                }

                return Ok(MediaInfo {
                    duration,
                    width,
                    height,
                    fps,
                    has_video,
                    has_audio,
                    video_codec,
                    audio_codec,
                });
            }
        }
    }

    // Fallback: parse standard ffmpeg -i output
    let output = Command::new(&loc.ffmpeg_path)
        .arg("-i")
        .arg(p)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut duration = 0.0;
    let width = 1920;
    let height = 1080;
    let fps = 30.0;
    let has_video = stderr.contains("Video:");
    let has_audio = stderr.contains("Audio:");

    // Duration: 00:01:23.45
    if let Some(pos) = stderr.find("Duration: ") {
        let rest = &stderr[pos + 10..];
        if let Some(comma) = rest.find(',') {
            let dur_str = &rest[..comma].trim();
            let parts: Vec<&str> = dur_str.split(':').collect();
            if parts.len() == 3 {
                if let (Ok(h), Ok(m), Ok(s)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
                    duration = h * 3600.0 + m * 60.0 + s;
                }
            }
        }
    }

    Ok(MediaInfo {
        duration,
        width,
        height,
        fps,
        has_video,
        has_audio,
        video_codec: if has_video { Some("h264".into()) } else { None },
        audio_codec: if has_audio { Some("aac".into()) } else { None },
    })
}
