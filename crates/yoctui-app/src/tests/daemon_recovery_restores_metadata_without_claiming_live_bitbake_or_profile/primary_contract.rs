use super::*;

#[cfg(unix)]
#[test]
fn daemon_recovery_restores_metadata_without_claiming_live_bitbake_or_profile() {
    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "current-boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.workspace = Some(yoctui_protocol::daemon::WorkspaceIdentity {
        canonical_source: "/work/poky".into(),
        canonical_build: "/work/poky/build".into(),
        identity_hash: "identity".into(),
    });
    snapshot.project_profile =
        yoctui_protocol::daemon::ProjectProfileSummary::Loaded { schema_version: 1 };
    snapshot.bitbake.version = Some("2.8.1".into());
    snapshot.bitbake.capabilities =
        vec![yoctui_protocol::daemon::BitBakeCapability::WorkspaceInspection];
    let persisted = yoctui_protocol::daemon_persist::DaemonPersistedState::capture(
        &snapshot,
        1,
        "previous-boot".into(),
        Vec::new(),
        yoctui_protocol::daemon_persist::PersistedPreferences::default(),
    );

    recover_daemon_model_metadata(&mut state, &persisted, "current-boot").unwrap();
    assert_eq!(
        state.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/work/poky/build"))
    );
    assert_eq!(
        state.project_profile,
        yoctui_model::ProjectProfileState::NotLoaded
    );
    assert_eq!(
        state.bitbake.lifecycle,
        yoctui_model::DaemonBitBakeLifecycle::Disconnected
    );
    assert_eq!(
        state.session.recovery,
        yoctui_model::DaemonRecoveryState::Degraded
    );
    assert!(
        state
            .session
            .recovery_warnings
            .iter()
            .any(|warning| warning.contains("must be reloaded"))
    );
}
