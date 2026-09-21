use super::*;

#[test]
fn daemon_attach_uses_top_level_workspace_identity_without_build_event() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([7; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.workspace = Some(yoctui_protocol::daemon::WorkspaceIdentity {
        canonical_source: "/srv/yocto/poky".into(),
        canonical_build: "/srv/yocto/build".into(),
        identity_hash: "workspace-identity".into(),
    });
    assert!(snapshot.build_events.is_empty());

    let mut app = yoctui_model::App::new(64, 64 * 1024);
    app.workspace.build_dir = Some("/client/stale-build".into());
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Inspector;
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut app, snapshot);

    assert_eq!(
        app.workspace.source_dir.as_deref(),
        Some(std::path::Path::new("/srv/yocto/poky"))
    );
    assert_eq!(
        app.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/srv/yocto/build"))
    );
    assert!(matches!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Connected(ref profile)
            if profile.source_dir == std::path::Path::new("/srv/yocto/poky")
                && profile.build_dir == std::path::Path::new("/srv/yocto/build")
    ));
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.focus, FocusTarget::Inspector);
}
