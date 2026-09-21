use super::*;

#[test]
fn offline_navigation_preserves_last_observed_state() {
    let mut app = App::new_unconfigured(32, 4096);
    app.require_daemon = true;
    for screen in [
        Screen::Dashboard,
        Screen::BuildHistory,
        Screen::Logs,
        Screen::Tasks,
        Screen::Settings,
        Screen::Kernel,
        Screen::Firmware,
        Screen::Images,
        Screen::Packages,
    ] {
        assert!(crate::update_with_workspace_authority(&mut app, Action::Open(screen)).is_none());
        assert_eq!(app.screen, screen);
    }
    assert!(
        app.offline_notice()
            .unwrap()
            .contains("No build environment")
    );
    app.daemon.status = ClientReplicaStatus::Current;
    app.observe_daemon(true, UNIX_EPOCH);
    assert!(app.offline_notice().is_none());
    app.daemon.status = ClientReplicaStatus::Disconnected;
    app.observe_daemon(false, SystemTime::now());
    assert_eq!(app.last_daemon_update, Some(UNIX_EPOCH));
    assert!(app.offline_notice().unwrap().contains("not live"));
}
