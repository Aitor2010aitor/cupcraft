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
    /// Detects FFmpeg binary on the system (resultado cacheado durante la sesión).
    pub fn find() -> Option<Self> {
        static CACHE: std::sync::OnceLock<Option<FfmpegLocation>> = std::sync::OnceLock::new();
        CACHE.get_or_init(Self::find_uncached).clone()
    }

    fn find_uncached() -> Option<Self> {
        // 0. Junto al ejecutable: <exe>	oolsfmpegfmpeg.exe o carpetas padre (instalación portable)
        if let Ok(exe) = std::env::current_exe() {
            let mut dir = exe.parent().map(|d| d.to_path_buf());
            for _ in 0..5 {
                let Some(d) = dir.clone() else { break };
                for cand in [d.join("ffmpeg.exe"), d.join("tools").join("ffmpeg").join("ffmpeg.exe")] {
                    if cand.exists() {
                        let probe = cand.with_file_name("ffprobe.exe");
                        return Some(Self { ffmpeg_path: cand, ffprobe_path: if probe.exists() { Some(probe) } else { None } });
                    }
                }
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
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
        if let Ok(output) = crate::ffmpeg::command("where.exe").arg("ffmpeg.exe").output() {
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
        let output = match crate::ffmpeg::command(&self.ffmpeg_path).args(["-hide_banner", "-encoders"]).output() {
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

/// Crea un `Command` que en Windows no abre ventana de consola (CREATE_NO_WINDOW).
pub fn command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}
