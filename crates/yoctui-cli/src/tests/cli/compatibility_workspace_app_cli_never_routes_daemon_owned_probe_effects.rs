use super::*;

#[test]
fn compatibility_workspace_app_cli_never_routes_daemon_owned_probe_effects() {
    let mut app = App::new(16, 4096);
    let effect = compatibility_workspace_action(&mut app, Action::InspectTestCapability);
    assert!(effect.is_none());
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Environment probing is daemon-owned")
    );
}

#[test]
fn native_navigation_cli_preserves_optional_screen_changes_without_probe_effects() {
    for destination in [Screen::Sdk, Screen::Testing, Screen::Security, Screen::Qa] {
        let mut app = App::new(16, 4096);
        app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
        app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([42; 16]));
        assert!(compatibility_workspace_action(&mut app, Action::Open(destination)).is_none());
        assert_eq!(app.screen, destination);
        assert!(app.notification.is_none());
        assert!(app.daemon.jobs.is_empty());
        assert!(app.daemon.pty_sessions.is_empty());
    }
}
