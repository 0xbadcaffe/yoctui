use super::*;

#[test]
fn layout_restore_round_trips_client_local_pane_tree() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-layout-restore-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.toml");
    let mut session = Session::default();
    let mut app = App::new(8, 1024);
    let root = app.pane_layout.focused;
    app.pane_layout
        .split(root, yoctui_model::SplitAxis::Vertical)
        .unwrap();
    persist_settings(Some(&path), &mut session, &app, true).unwrap();
    let restored = read_session(Some(&path)).unwrap().pane_layout.unwrap();
    assert_eq!(restored, app.pane_layout);
    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
