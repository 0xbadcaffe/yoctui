use super::*;

#[test]
fn settings_persistence_preserves_unrelated_session_fields() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-settings-save-{}", std::process::id()));
    let path = directory.join("session.toml");
    let mut session = Session {
        last_target: Some("core-image-minimal".into()),
        recent_build_dirs: vec![PathBuf::from("/build")],
        ..Session::default()
    };
    let mut app = App::new(10, 1_000);
    app.theme = Theme::HighContrast;
    app.animation_speed = AnimationSpeed::Slow;
    app.reduced_motion = true;
    app.color_enabled = false;
    app.logs.wrap = true;
    app.logs.follow = false;

    persist_settings(Some(&path), &mut session, &app, true).unwrap();
    let saved = read_session(Some(&path)).unwrap();
    assert_eq!(saved.last_target.as_deref(), Some("core-image-minimal"));
    assert_eq!(saved.recent_build_dirs, [PathBuf::from("/build")]);
    let preferences = saved.preferences.unwrap();
    assert_eq!(preferences.theme, Theme::HighContrast);
    assert_eq!(preferences.animation_speed, AnimationSpeed::Slow);
    assert!(preferences.reduced_motion);
    assert!(!preferences.color_enabled);
    assert!(preferences.log_wrap);
    assert!(!preferences.log_follow);
    assert_eq!(saved.theme, None, "legacy fields are normalized away");

    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
