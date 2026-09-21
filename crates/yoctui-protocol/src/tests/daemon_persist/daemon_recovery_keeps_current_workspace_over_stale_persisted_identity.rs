use super::*;

#[test]
fn daemon_recovery_keeps_current_workspace_over_stale_persisted_identity() {
    let mut prior = snapshot();
    prior.workspace = Some(WorkspaceIdentity {
        canonical_source: "/srv/old/poky".into(),
        canonical_build: "/srv/old/build".into(),
        identity_hash: "old-workspace".into(),
    });
    let persisted = DaemonPersistedState::capture(
        &prior,
        99,
        "same-boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    let current_workspace = WorkspaceIdentity {
        canonical_source: "/srv/current/poky".into(),
        canonical_build: "/srv/current/build".into(),
        identity_hash: "current-workspace".into(),
    };
    let mut current = snapshot();
    current.workspace = Some(current_workspace.clone());

    let (recovered, _) = recover_persisted_snapshot(current, &persisted, "same-boot");

    assert_eq!(recovered.workspace, Some(current_workspace));
}
