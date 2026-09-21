use super::*;

#[test]
fn rootfs_sources_roundtrip_preserves_missing_and_cleaned_paths() {
    let sources = RootfsSourcesData {
        query: RootfsSourcesRequestData {
            request: data().request,
            daemon_instance_id: crate::daemon::DaemonInstanceId([7; 16]),
            compatibility_generation: 3,
        },
        image_manifest: None,
        pkgdata_dir: Some("/build/pkgdata".into()),
        image_rootfs: Some("/build/cleaned-rootfs".into()),
    };
    sources.validate().unwrap();
    let outcome = crate::daemon::CommandOutcome::RootfsSources {
        sources: Box::new(sources.clone()),
    };
    let decoded: crate::daemon::CommandOutcome =
        serde_json::from_slice(&serde_json::to_vec(&outcome).unwrap()).unwrap();
    assert_eq!(decoded, outcome);
    let command = crate::daemon::DaemonCommand::InspectRootfsSources {
        query: sources.query,
    };
    let decoded: crate::daemon::DaemonCommand =
        serde_json::from_slice(&serde_json::to_vec(&command).unwrap()).unwrap();
    assert_eq!(decoded, command);
}
