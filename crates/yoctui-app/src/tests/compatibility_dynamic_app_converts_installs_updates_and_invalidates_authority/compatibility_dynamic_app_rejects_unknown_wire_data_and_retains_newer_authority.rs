use super::*;

#[test]
fn compatibility_dynamic_app_rejects_unknown_wire_data_and_retains_newer_authority() {
    let first = compatibility_workspace_authority(1).normalize().unwrap();
    let mut unknown = daemon_compatibility_protocol(&first);
    unknown.capabilities[0].id = "future.unregistered.capability".into();
    assert!(compatibility_model_snapshot(&unknown).is_err());

    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    let mut malformed_snapshot = compatibility_workspace_daemon_snapshot(&first);
    malformed_snapshot.compatibility = Some(unknown);
    client.replace_app(&mut app, malformed_snapshot);
    assert!(app.workspace_compatibility.authority().is_none());
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("unknown capability ID")
    );

    let second = compatibility_workspace_authority(2).normalize().unwrap();
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&second));
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&first));
    assert_eq!(
        app.workspace_compatibility
            .authority()
            .unwrap()
            .snapshot
            .generation,
        2
    );
    assert!(app.notification.as_deref().unwrap().contains("stale"));
}
