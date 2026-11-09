//! Prueba de integración con un archivo exportado por la app JS real
//! (`tests/fixtures/rodaje-otono.opencup.json`).
//!
//! Comprueba que un proyecto con texto, clip GL, clip de audio, campos
//! desconocidos (keyframes/animación) y medios incrustados sobrevive el
//! viaje completo: archivo → almacén → archivo.

use cupcraft_persist::{ImportReport, ProjectFile, Store, PROJECT_FILE_KIND};

fn fixture_text() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/rodaje-otono.opencup.json"
    ))
    .expect("falta el fixture")
}

#[test]
fn parses_the_real_js_export() {
    let file = ProjectFile::parse(&fixture_text()).expect("el fixture debe ser válido");
    assert_eq!(file.kind, PROJECT_FILE_KIND);
    assert_eq!(file.file_version, 1);
    assert_eq!(file.app, "openCUP");
    assert_eq!(file.project.name, "Rodaje otoño");
    assert_eq!(file.project.ratio, "9:16");
    assert_eq!(file.project.resolution, "1080p");
    assert_eq!(file.project.fps, 30);
    assert_eq!(file.project.tracks.len(), 3);
    assert_eq!(file.media.len(), 3);

    // Pistas en el mismo orden y con sus clips.
    let kinds: Vec<&str> = file
        .project
        .tracks
        .iter()
        .map(|t| match t.kind {
            cupcraft_persist::TrackType::Text => "text",
            cupcraft_persist::TrackType::Video => "video",
            cupcraft_persist::TrackType::Audio => "audio",
            cupcraft_persist::TrackType::Overlay => "overlay",
        })
        .collect();
    assert_eq!(kinds, ["text", "video", "audio"]);

    // Clip de texto: sus campos de texto se conservan.
    let text_clip = &file.project.tracks[0].clips[0];
    assert_eq!(text_clip.kind, cupcraft_persist::ClipKind::Text);
    let text = text_clip.text.as_ref().expect("el clip de texto lleva su objeto text");
    assert_eq!(text["content"], "Otoño en la sierra");
    assert_eq!(text["size"], 96);

    // Clip de vídeo: filtro GL + campos no modelados (keyframes, animation).
    let video_clip = &file.project.tracks[1].clips[0];
    assert_eq!(video_clip.gl_filter.as_deref(), Some("bruma-borgona"));
    assert_eq!(video_clip.gl_filter_name.as_deref(), Some("Bruma borgoña"));
    assert_eq!(video_clip.gl_filter_amount, Some(0.7));
    assert_eq!(video_clip.filter, "vivid");
    assert_eq!(video_clip.speed, 1.5);
    assert_eq!(video_clip.in_point, 1.1);
    assert!(video_clip.extra.contains_key("keyframes"), "los keyframes se conservan");
    assert!(video_clip.extra.contains_key("animation"), "la animación se conserva");

    // Audio con fades.
    let audio_clip = &file.project.tracks[2].clips[0];
    assert_eq!(audio_clip.kind, cupcraft_persist::ClipKind::Audio);
    assert!((audio_clip.fade_in - 1.0).abs() < 1e-9);
    assert!((audio_clip.fade_out - 2.0).abs() < 1e-9);

    // Medios: dos con bytes y uno (el audio) sin archivo embebido.
    let (metas, missing) = file.split_media();
    assert_eq!(metas.len(), 3);
    assert_eq!(missing, 1, "m3 venía sin data");
    assert_eq!(metas[0].duration, Some(6.8));
}

#[test]
fn round_trip_through_the_store_keeps_everything() {
    let text = fixture_text();

    let mut store = Store::open_in_memory().unwrap();
    let report: ImportReport = store.import_project_file(&text).unwrap();
    assert_eq!(report.id, "prj_fixture_rodaje");
    assert_eq!(report.name, "Rodaje otoño");
    assert_eq!(report.media, 3);
    assert_eq!(report.missing, 1);

    // El proyecto quedó guardado con sus tres pistas y seis clips.
    let project = store.get_project("prj_fixture_rodaje").unwrap().expect("proyecto importado");
    assert_eq!(project.tracks.iter().map(|t| t.clips.len()).sum::<usize>(), 4);
    assert_eq!(project.media.len(), 3);
    assert_eq!(project.thumb.as_deref().map(|t| t.starts_with("data:image/webp")), Some(true));

    // Los dos medios con bytes están en el almacén; el tercero no.
    let m1 = store.get_media("m1").unwrap().expect("m1 con bytes");
    assert_eq!(m1.mime, "video/webm");
    assert!(!m1.bytes.is_empty());
    let m2 = store.get_media("m2").unwrap().expect("m2 con bytes");
    assert_eq!(m2.mime, "image/png");
    assert_eq!(m2.bytes.len(), 70, "el PNG de 1x1");
    assert!(store.get_media("m3").unwrap().is_none(), "m3 venía sin archivo");

    // Reexportar y volver a importar: todo sigue intacto.
    let reexported = store.export_project_file("prj_fixture_rodaje").unwrap().to_text();
    let mut other = Store::open_in_memory().unwrap();
    let again = other.import_project_file(&reexported).unwrap();
    assert_eq!(again.media, 3);
    assert_eq!(again.missing, 1, "m3 sigue sin bytes");
    let project2 = other.get_project("prj_fixture_rodaje").unwrap().unwrap();
    let clip = &project2.tracks[1].clips[0];
    assert_eq!(clip.gl_filter.as_deref(), Some("bruma-borgona"));
    assert!(clip.extra.contains_key("keyframes"), "los campos desconocidos sobreviven");
    assert_eq!(clip.speed, 1.5);
    assert_eq!(project2.tracks[0].clips[0].text.as_ref().unwrap()["content"], "Otoño en la sierra");
}
