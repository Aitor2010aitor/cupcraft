#[cfg(test)]
mod tests {
    use crate::exporter::ExportFormat;
    use crate::ffmpeg::{FfmpegLocation, HwAccel};
    use cupcraft_audio::AudioBuffer;

    #[test]
    fn ffmpeg_location_find_does_not_panic() {
        let _loc = FfmpegLocation::find();
    }

    #[test]
    fn hw_accel_enum_variants() {
        assert_ne!(HwAccel::Nvenc, HwAccel::Cpu);
        assert_ne!(HwAccel::Qsv, HwAccel::Amf);
    }

    #[test]
    fn export_format_variants() {
        assert_ne!(ExportFormat::Mp4H264, ExportFormat::Mp4H265);
        assert_ne!(ExportFormat::Mp4H264, ExportFormat::WebmVp9);
    }

    #[test]
    fn write_wav_produces_valid_riff_header() {
        let temp_wav = std::env::temp_dir().join(format!("test_riff_{}.wav", std::process::id()));
        let buf = AudioBuffer::new(44100, 2, vec![0.0, 0.5, -0.5, 0.0]);

        crate::exporter::write_wav(&temp_wav, &buf).expect("failed to write test wav");
        assert!(temp_wav.exists());

        let bytes = std::fs::read(&temp_wav).expect("failed to read test wav");
        assert!(bytes.len() >= 44);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");

        let _ = std::fs::remove_file(temp_wav);
    }
}
