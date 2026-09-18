use super::*;

#[test]
fn no_build_directory_resolves_to_unconfigured_startup() {
    let directory = std::env::temp_dir().join(format!("yoctui-no-build-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let config_path = directory.join("config.toml");
    fs::write(&config_path, "").unwrap();
    let cli = Cli::try_parse_from(["yoctui", "--config", config_path.to_str().unwrap()]).unwrap();
    let mut session = Session::default();
    session
        .recent_build_dirs
        .push(std::env::current_dir().unwrap());
    let resolved = resolve_config(&cli, &session).unwrap();
    assert!(!resolved.build_dir_configured);
    assert_eq!(resolved.build_dir, PathBuf::from("/"));
    fs::remove_file(config_path).unwrap();
    fs::remove_dir(directory).unwrap();
}
