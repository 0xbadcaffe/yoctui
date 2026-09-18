use super::*;

#[test]
fn startup_session_legacy_backend_does_not_override_bridge_default() {
    let directory = std::env::temp_dir().join(format!(
        "yoctui-startup-session-backend-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    let config_path = directory.join("config.toml");
    fs::write(&config_path, "").unwrap();
    let cli = Cli::try_parse_from(["yoctui", "--config", config_path.to_str().unwrap()]).unwrap();
    let session = Session {
        last_backend: Some(Backend::Process),
        ..Session::default()
    };

    let resolved = resolve_config(&cli, &session).unwrap();
    assert_eq!(resolved.backend, Backend::Bridge);

    fs::remove_file(config_path).unwrap();
    fs::remove_dir(directory).unwrap();
}
