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
