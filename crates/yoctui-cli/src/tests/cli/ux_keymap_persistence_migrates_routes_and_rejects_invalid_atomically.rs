use super::*;

#[test]
fn ux_keymap_persistence_migrates_routes_and_rejects_invalid_atomically() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-keymap-persistence-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.toml");
    fs::write(
            &path,
            "[keymap]\nschema_version = 0\n[[keymap.bindings]]\naction = 'navigate.logs'\nkeys = ['z', 'g l']\n",
        )
        .unwrap();

    let session = read_session(Some(&path)).unwrap();
    assert_eq!(
        session.keymap.schema_version,
        yoctui_model::KEYMAP_SCHEMA_VERSION
    );
    let mut app = App::new(8, 1024);
    app.install_preferences(session_preferences(&session).unwrap())
        .unwrap();
    assert!(matches!(
        keymap_action_for_app(&mut app, Input::Char('z')),
        yoctui_app::KeymapInputResult::Action(action)
            if matches!(*action, Action::Open(Screen::Logs))
    ));
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('g')),
        yoctui_app::KeymapInputResult::Pending
    );
    assert!(matches!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        yoctui_app::KeymapInputResult::Action(action)
            if matches!(*action, Action::Open(Screen::Logs))
    ));

    write_session(Some(&path), &session).unwrap();
    let before = fs::read(&path).unwrap();
    let mut invalid = session;
    invalid.keymap.overrides[0].sequences = vec!["e".parse().unwrap()];
    assert!(write_session(Some(&path), &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(fs::read_dir(&directory).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));

    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
