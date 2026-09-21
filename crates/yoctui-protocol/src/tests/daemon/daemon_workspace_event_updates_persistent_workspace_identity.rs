use super::*;

#[test]
fn daemon_workspace_event_updates_persistent_workspace_identity() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Workspace {
            data: WorkspaceData {
                build_dir: Some("/srv/yocto/build".into()),
                source_dir: Some("/srv/yocto/poky".into()),
                variables: std::collections::HashMap::new(),
                variable_provenance: std::collections::HashMap::new(),
                variable_provenance_chain: std::collections::HashMap::new(),
                bitbake_version: Some("2.18.0".into()),
                release: Some("6.0.2".into()),
                layers: Vec::new(),
                recipes: Vec::new(),
            },
        }))
        .unwrap();

    let identity = journal.snapshot().workspace.as_ref().unwrap();
    assert_eq!(identity.canonical_source, "/srv/yocto/poky");
    assert_eq!(identity.canonical_build, "/srv/yocto/build");
    assert_eq!(identity.identity_hash.len(), 16);
}
