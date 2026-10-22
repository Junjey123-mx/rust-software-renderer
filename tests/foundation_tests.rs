use rust_software_renderer::config::AppConfig;

#[test]
fn default_resolution_is_800_by_600() {
    let config = AppConfig::default();

    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
}

#[test]
fn default_target_fps_is_60() {
    let config = AppConfig::default();

    assert_eq!(config.target_fps, 60);
}
