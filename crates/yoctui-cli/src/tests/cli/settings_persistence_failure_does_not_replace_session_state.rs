use super::*;

#[test]
fn settings_persistence_failure_does_not_replace_session_state() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-settings-failure-{}", std::process::id()));
    fs::write(&directory, "not a directory").unwrap();
    let path = directory.join("session.toml");
    let mut session = Session {
        theme: Some(Theme::DarkPro),
        ..Session::default()
    };
    let mut app = App::new(10, 1_000);
    app.theme = Theme::WhiteClassic;

    assert!(persist_settings(Some(&path), &mut session, &app, true).is_err());
    assert_eq!(session.theme, Some(Theme::DarkPro));

    fs::remove_file(directory).unwrap();
}
