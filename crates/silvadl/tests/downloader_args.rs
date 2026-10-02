use silvadl::{AppConfig, MediaKind};

#[test]
fn video_selector_honors_configured_resolution_and_fragments() {
    let config = AppConfig::defaults().unwrap().download;
    let args = config.args_for("https://example.test/video");
    assert!(args.contains(&"bv*[height<=1080]+ba/b[height<=1080]".to_owned()));
    assert!(args.windows(2).any(|v| v == ["--concurrent-fragments", "8"]));
}

#[test]
fn audio_uses_configured_postprocessor_format() {
    let mut config = AppConfig::defaults().unwrap().download;
    config.media_kind = MediaKind::Audio;
    config.audio_format = "flac".into();
    let args = config.args_for("https://example.test/audio");
    assert!(args.windows(2).any(|v| v == ["--audio-format", "flac"]));
}

#[test]
fn output_template_keeps_items_in_selected_folder() {
    let config = AppConfig::defaults().unwrap().download;
    let args = config.args_for("https://example.test/playlist-item");
    let template = args.windows(2).find(|pair| pair[0] == "--output").map(|pair| pair[1].as_str()).unwrap();
    assert_eq!(template, "%(title)s [%(id)s].%(ext)s");
    assert!(!template.contains('/'));
}

#[test]
fn persisted_config_round_trips() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let config = AppConfig::defaults().unwrap();
    config.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();
    assert_eq!(loaded.download.concurrent_fragments, config.download.concurrent_fragments);
}
