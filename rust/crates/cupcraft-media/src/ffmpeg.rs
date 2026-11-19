use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccel {
    Nvenc,
    Qsv,
    Amf,
    Cpu,
}

#[derive(Debug, Clone)]
pub struct FfmpegLocation {
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: Option<PathBuf>,
}

impl FfmpegLocation {
    /// Detects FFmpeg binary on the system.
    pub fn find() -> Option<Self> {
        // 1. Check local data directory: %LOCALAPPDATA%\cupcraft\bin\ffmpeg.exe
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join("cupcraft").join("bin").join("ffmpeg.exe");
            if p.exists() {
                let probe = p.with_file_name("ffprobe.exe");
                return Some(Self {
                    ffmpeg_path: p,
                    ffprobe_path: if probe.exists() { Some(probe) } else { None },
                });
            }
        }

        // 2. Check tools/ffmpeg
        let local_tool = PathBuf::from("tools").join("ffmpeg").join("ffmpeg.exe");
        if local_tool.exists() {
            let probe = local_tool.with_file_name("ffprobe.exe");
            return Some(Self {
                ffmpeg_path: local_tool,
                ffprobe_path: if probe.exists() { Some(probe) } else { None },
            });
        }

        // 3. Check system PATH
        if let Ok(output) = Command::new("where.exe").arg("ffmpeg.exe").output() {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = out_str.lines().next() {
                    let p = PathBuf::from(first_line.trim());
                    if p.exists() {
                        let probe = p.with_file_name("ffprobe.exe");
                        return Some(Self {
                            ffmpeg_path: p,
                            ffprobe_path: if probe.exists() { Some(probe) } else { None },
                        });
                    }
                }
            }
        }

        None
    }

    /// Detects available hardware encoders (NVENC, QSV, AMF).
    pub fn detect_hardware_encoder(&self) -> HwAccel {
        let output = match Command::new(&self.ffmpeg_path).args(["-hide_banner", "-encoders"]).output() {
            Ok(o) => o,
            Err(_) => return HwAccel::Cpu,
        };

        let text = String::from_utf8_lossy(&output.stdout);
        if text.contains("h264_nvenc") {
            HwAccel::Nvenc
        } else if text.contains("h264_qsv") {
            HwAccel::Qsv
        } else if text.contains("h264_amf") {
            HwAccel::Amf
        } else {
            HwAccel::Cpu
        }
    }
}
