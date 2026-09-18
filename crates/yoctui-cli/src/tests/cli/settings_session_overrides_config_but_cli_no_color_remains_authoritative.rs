use super::*;

#[test]
fn settings_session_overrides_config_but_cli_no_color_remains_authoritative() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-settings-precedence-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let config_path = directory.join("config.toml");
    fs::write(
        &config_path,
        "theme = 'dark'\nanimation_speed = 'fast'\nreduced_motion = false\ncolor = true\n",
    )
    .unwrap();
    let cli = Cli::try_parse_from([
        "yoctui",
        "--config",
        config_path.to_str().unwrap(),
        "--no-color",
    ])
    .unwrap();
    let session = Session {
        theme: Some(Theme::MatrixGreen),
        animation_speed: Some(AnimationSpeed::Slow),
        reduced_motion: Some(true),
        color_enabled: Some(true),
        ..Session::default()
    };

    let resolved = resolve_config(&cli, &session).unwrap();
    assert_eq!(resolved.theme, Theme::MatrixGreen);
    assert_eq!(resolved.animation_speed, AnimationSpeed::Slow);
    assert!(resolved.reduced_motion);
    assert!(!resolved.color);
    assert!(resolved.color_forced_off);

    fs::remove_file(config_path).unwrap();
    fs::remove_dir(directory).unwrap();
}
