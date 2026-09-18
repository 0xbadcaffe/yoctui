use super::*;

#[test]
fn parses_retention_and_backend_settings() {
    let config: FileConfig = toml::from_str(
            "backend = 'process'\nlog_retention_entries = 42\nlog_retention_bytes = 1024\nrefresh_ms = 50\ncancellation_timeout_ms = 250\ndefault_target = 'core-image-minimal'\neditor = 'nano'",
        )
        .unwrap();
    assert!(matches!(config.backend, Some(Backend::Process)));
    assert_eq!(config.log_retention_entries, Some(42));
    assert_eq!(config.default_target.as_deref(), Some("core-image-minimal"));
    assert_eq!(config.editor.as_deref(), Some("nano"));
    assert_eq!(config.cancellation_timeout_ms, Some(250));
}
