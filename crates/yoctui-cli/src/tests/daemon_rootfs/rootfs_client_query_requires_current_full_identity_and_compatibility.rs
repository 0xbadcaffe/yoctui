use super::*;

#[test]
fn rootfs_client_query_requires_current_full_identity_and_compatibility() {
    let (build, query, compatibility, _) = fixture();
    let request = RootfsCompositionRequest {
        generation: query.request.generation,
        image: ImageArtifactIdentity {
            machine: "machine".into(),
            image: "image".into(),
            path: query.request.image.path.clone().into(),
        },
    };
    let mut app = App::new(16, 4096);
    assert!(query_for_app(&app, &request).is_err());
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.instance_id = Some(DaemonModelInstanceId(query.daemon_instance_id.0));
    assert!(query_for_app(&app, &request).is_err());
    app.workspace_compatibility.install(compatibility).unwrap();
    assert_eq!(query_for_app(&app, &request).unwrap(), query);
    app.daemon.status = ClientReplicaStatus::Stale;
    assert!(query_for_app(&app, &request).is_err());
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.instance_id.as_mut().unwrap().0[15] ^= 1;
    assert_ne!(query_for_app(&app, &request).unwrap(), query);
    fs::remove_dir_all(build).unwrap();
}
