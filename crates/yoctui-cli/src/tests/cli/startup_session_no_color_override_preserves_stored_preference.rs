use super::*;

#[test]
fn startup_session_no_color_override_preserves_stored_preference() {
    let directory = std::env::temp_dir().join(format!(
        "yoctui-startup-session-color-{}",
        std::process::id()
    ));
    let path = directory.join("session.toml");
    let mut session = Session {
        color_enabled: Some(true),
        ..Session::default()
    };
    let mut app = App::new(8, 1024);
    app.color_enabled = false;

    persist_settings(Some(&path), &mut session, &app, false).unwrap();

    let saved = read_session(Some(&path)).unwrap();
    assert!(saved.preferences.unwrap().color_enabled);
    assert_eq!(saved.color_enabled, None, "legacy field is normalized away");
    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
