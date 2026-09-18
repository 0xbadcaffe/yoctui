use super::*;

#[test]
fn session_round_trip_preserves_preferences() {
    let directory = std::env::temp_dir().join(format!("yoctui-session-{}", std::process::id()));
    let path = directory.join("session.toml");
    write_session(
        Some(&path),
        &Session {
            preferences: None,
            last_target: Some("core-image-minimal".into()),
            last_screen: Some(Screen::Logs),
            log_filter: Some(Severity::Warning),
            log_recipe_filter: Some("busybox".into()),
            log_task_filter: Some("do_compile".into()),
            log_build_filter: Some("core-image-minimal".into()),
            log_wrap: Some(true),
            log_follow: Some(false),
            theme: Some(Theme::MatrixGreen),
            animation_speed: Some(AnimationSpeed::Slow),
            reduced_motion: Some(true),
            color_enabled: Some(true),
            last_backend: Some(Backend::Process),
            recent_build_dirs: vec![PathBuf::from("/build")],
            pane_layout: None,
            raw_favorites: Vec::new(),
            keymap: yoctui_model::KeymapPreferences::default(),
            onboarding: None,
        },
    )
    .unwrap();
    assert_eq!(
        read_session(Some(&path)).unwrap(),
        Session {
            preferences: None,
            last_target: Some("core-image-minimal".into()),
            last_screen: Some(Screen::Logs),
            log_filter: Some(Severity::Warning),
            log_recipe_filter: Some("busybox".into()),
            log_task_filter: Some("do_compile".into()),
            log_build_filter: Some("core-image-minimal".into()),
            log_wrap: Some(true),
            log_follow: Some(false),
            theme: Some(Theme::MatrixGreen),
            animation_speed: Some(AnimationSpeed::Slow),
            reduced_motion: Some(true),
            color_enabled: Some(true),
            last_backend: Some(Backend::Process),
            recent_build_dirs: vec![PathBuf::from("/build")],
            pane_layout: None,
            raw_favorites: Vec::new(),
            keymap: yoctui_model::KeymapPreferences::default(),
            onboarding: None,
        }
    );
    fs::remove_file(&path).unwrap();
    fs::remove_dir(&directory).unwrap();
}
