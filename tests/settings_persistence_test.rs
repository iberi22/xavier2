use xavier::settings::XavierSettings;
use tempfile::tempdir;
use std::env;

#[test]
fn test_settings_save_and_load() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("xavier.config.json");
    env::set_var("XAVIER_CONFIG_PATH", config_path.clone());

    let mut settings = XavierSettings::default();
    settings.server.port = 9999;
    settings.server.token = Some("test-token".to_string());
    settings.workspace.default_workspace_id = "test-ws".to_string();

    settings.save().expect("Failed to save settings");

    assert!(config_path.exists());

    let loaded = XavierSettings::load().expect("Failed to load settings").expect("Settings should exist");
    assert_eq!(loaded.server.port, 9999);
    assert_eq!(loaded.server.token, Some("test-token".to_string()));
    assert_eq!(loaded.workspace.default_workspace_id, "test-ws");
}
