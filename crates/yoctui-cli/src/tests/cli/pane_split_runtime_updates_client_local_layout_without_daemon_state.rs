use super::*;

#[test]
fn pane_split_runtime_updates_client_local_layout_without_daemon_state() {
    let mut app = App::new(16, 4096);
    let root = app.pane_layout.focused;
    let child = app
        .pane_layout
        .split(root, yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    assert_eq!(app.pane_layout.focused, child);
    assert_eq!(app.daemon.pty_sessions.len(), 0);
    app.pane_layout.focus(root).unwrap();
    app.pane_layout.close(root).unwrap();
    assert_eq!(app.pane_layout.pane_ids(), vec![child]);
}
