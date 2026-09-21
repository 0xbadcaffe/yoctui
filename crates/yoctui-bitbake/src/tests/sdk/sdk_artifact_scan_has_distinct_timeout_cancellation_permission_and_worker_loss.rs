use super::*;

#[tokio::test]
async fn sdk_artifact_scan_has_distinct_timeout_cancellation_permission_and_worker_loss() {
    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    let cancellation = SdkArtifactCancellation::default();
    cancellation.cancel();
    assert_eq!(
        SdkArtifactAdapter::new(root.clone())
            .scan_with_cancellation(request(root.clone()), cancellation)
            .await,
        Err(SdkArtifactAdapterError::Cancelled)
    );
    assert!(matches!(
        SdkArtifactAdapter::new(root.clone())
            .with_timeout(Duration::ZERO)
            .scan(request(root.clone()))
            .await,
        Err(SdkArtifactAdapterError::Timeout(_))
    ));
    assert!(matches!(
        SdkArtifactAdapter::new(root.clone())
            .with_worker_panic()
            .scan(request(root.clone()))
            .await,
        Err(SdkArtifactAdapterError::WorkerLost(_))
    ));
    assert_eq!(
        root_io_error(
            &root,
            io::Error::new(io::ErrorKind::PermissionDenied, "denied")
        ),
        SdkArtifactAdapterError::PermissionDenied(root)
    );
}
