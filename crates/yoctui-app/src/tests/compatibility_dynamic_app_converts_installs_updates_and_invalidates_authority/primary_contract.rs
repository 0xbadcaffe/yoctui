use super::*;

#[test]
fn compatibility_dynamic_app_converts_installs_updates_and_invalidates_authority() {
    let first = compatibility_workspace_authority(1).normalize().unwrap();
    let wire = daemon_compatibility_protocol(&first);
    assert_eq!(compatibility_model_snapshot(&wire).unwrap(), first);

    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Inspector;
    let mut client = DaemonClientSnapshot::default();
    let snapshot = compatibility_workspace_daemon_snapshot(&first);
    let next_sequence = snapshot.sequence + 1;
    let next_generation = snapshot.generation + 1;
    client.replace_app(&mut app, snapshot);
    assert_eq!(
        app.workspace_compatibility
            .authority()
            .unwrap()
            .snapshot
            .generation,
        1
    );
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.focus, FocusTarget::Inspector);

    let second = compatibility_workspace_authority(2).normalize().unwrap();
    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: next_sequence,
                generation: next_generation,
                event: yoctui_protocol::daemon::DaemonEvent::CompatibilityChanged(Box::new(
                    daemon_compatibility_protocol(&second),
                )),
            },
        )
        .unwrap();
    assert_eq!(
        app.workspace_compatibility
            .authority()
            .unwrap()
            .snapshot
            .generation,
        2
    );
    assert_eq!(app.screen, Screen::Layers);

    client.disconnect_app(&mut app);
    assert!(app.workspace_compatibility.authority().is_none());
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.focus, FocusTarget::Inspector);
}
