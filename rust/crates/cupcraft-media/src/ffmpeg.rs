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
        let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
        if cache.is_none() {
            *cache = Some(Self::find_uncached());
        }
        cache.clone().flatten()
    }

    /// Olvida el resultado cacheado (p. ej. tras instalar FFmpeg con el permiso de administrador).
    pub fn refresh() -> Option<Self> {
        *cache().lock().unwrap_or_else(|e| e.into_inner()) = None;
        Self::find()
    }

    fn find_uncached() -> Option<Self> {
        #[cfg(unix)]
        {
            return find_unix();
        }
        #[cfg(not(unix))]
        Self::find_windows()
    }

    #[cfg(not(unix))]
    fn find_windows() -> Option<Self> {
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

fn cache() -> &'static std::sync::Mutex<Option<Option<FfmpegLocation>>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Option<Option<FfmpegLocation>>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(None))
}

/// Linux/macOS: busca `ffmpeg` en el PATH y en las rutas habituales.
#[cfg(unix)]
fn find_unix() -> Option<FfmpegLocation> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    for extra in ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin", "/snap/bin"] {
        dirs.push(PathBuf::from(extra));
    }
    for d in dirs {
        let p = d.join("ffmpeg");
        if p.is_file() {
            let probe = d.join("ffprobe");
            return Some(FfmpegLocation { ffmpeg_path: p, ffprobe_path: if probe.is_file() { Some(probe) } else { None } });
        }
    }
    None
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
