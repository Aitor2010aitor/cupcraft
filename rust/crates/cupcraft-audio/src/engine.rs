use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use cupcraft_core::model::Project;
use crate::mixer::AudioMixer;

/// Atomic playhead time tracker in seconds.
pub struct PlayheadClock {
    time_nanos: std::sync::atomic::AtomicU64,
}

impl PlayheadClock {
    pub fn new(initial_sec: f64) -> Self {
        Self {
            time_nanos: std::sync::atomic::AtomicU64::new((initial_sec * 1_000_000_000.0) as u64),
        }
    }

    pub fn get(&self) -> f64 {
        self.time_nanos.load(Ordering::Relaxed) as f64 / 1_000_000_000.0
    }

    pub fn set(&self, sec: f64) {
        self.time_nanos.store((sec.max(0.0) * 1_000_000_000.0) as u64, Ordering::Relaxed);
    }

    pub fn advance(&self, delta_sec: f64) {
        let delta_nanos = (delta_sec * 1_000_000_000.0) as u64;
        self.time_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
    }
}

/// Real-time playback engine powered by CPAL.
pub struct AudioEngine {
    _stream: Option<Stream>,
    pub is_playing: Arc<AtomicBool>,
    /// Silencio general del reproductor (botón de volumen): el reloj sigue avanzando.
    pub muted: Arc<AtomicBool>,
    pub clock: Arc<PlayheadClock>,
    pub mixer: Arc<Mutex<AudioMixer>>,
    pub project: Arc<Mutex<Project>>,
}

impl AudioEngine {
    pub fn new(project: Project) -> Self {
        let clock = Arc::new(PlayheadClock::new(0.0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let muted = Arc::new(AtomicBool::new(false));
        let mixer = Arc::new(Mutex::new(AudioMixer::new()));
        let project = Arc::new(Mutex::new(project));

        let mut stream_opt = None;

        if let Ok(host) = cpal::default_host().default_output_device().ok_or(()) {
            if let Ok(supported_config) = host.default_output_config() {
                let sample_format = supported_config.sample_format();
                let config: StreamConfig = supported_config.into();

                let clock_clone = Arc::clone(&clock);
                let playing_clone = Arc::clone(&is_playing);
                let muted_clone = Arc::clone(&muted);
                let mixer_clone = Arc::clone(&mixer);
                let project_clone = Arc::clone(&project);

                let channels = config.channels as usize;
                let sample_rate = config.sample_rate.0 as f64;
                let dt = 1.0 / sample_rate;

                let err_fn = |err| log::error!("CPAL audio stream error: {}", err);

                let stream = match sample_format {
                    SampleFormat::F32 => host.build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            if !playing_clone.load(Ordering::Relaxed) {
                                data.fill(0.0);
                                return;
                            }

                            let proj_guard = project_clone.lock();
                            let mixer_guard = mixer_clone.lock();
                            let frames = data.len() / channels;
                            let gain = if muted_clone.load(Ordering::Relaxed) { 0.0 } else { 1.0 };

                            for f in 0..frames {
                                let t = clock_clone.get();
                                let (l, r) = mixer_guard.sample_at(&proj_guard, t);
                                let (l, r) = (l * gain, r * gain);

                                let idx = f * channels;
                                if channels >= 2 {
                                    data[idx] = l;
                                    data[idx + 1] = r;
                                    for c in 2..channels {
                                        data[idx + c] = 0.0;
                                    }
                                } else if channels == 1 {
                                    data[idx] = (l + r) * 0.5;
                                }

                                clock_clone.advance(dt);
                            }
                        },
                        err_fn,
                        None,
                    ),
                    _ => Err(cpal::BuildStreamError::StreamConfigNotSupported),
                };

                if let Ok(s) = stream {
                    let _ = s.play();
                    stream_opt = Some(s);
                }
            }
        }

        Self {
            _stream: stream_opt,
            is_playing,
            muted,
            clock,
            mixer,
            project,
        }
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn seek(&self, time_sec: f64) {
        self.clock.set(time_sec);
    }

    /// `true` si hay un dispositivo de salida activo (el reloj avanza con el audio).
    pub fn has_output(&self) -> bool {
        self._stream.is_some()
    }

    pub fn current_time(&self) -> f64 {
        self.clock.get()
    }
}
