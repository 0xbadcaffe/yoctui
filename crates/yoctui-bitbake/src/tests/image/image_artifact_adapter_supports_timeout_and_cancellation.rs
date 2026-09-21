use super::*;

#[tokio::test]
async fn image_artifact_adapter_supports_timeout_and_cancellation() {
    let fixture = fixture();
    let deploy = fixture.path().join("qemux86-64");
    fs::create_dir(&deploy).unwrap();
    let cancellation = ImageArtifactCancellation::default();
    cancellation.cancel();
    assert_eq!(
        ImageArtifactAdapter::new(deploy.clone())
            .scan_with_cancellation(request(), cancellation)
            .await,
        Err(ImageArtifactAdapterError::Cancelled)
    );
    assert!(matches!(
        ImageArtifactAdapter::new(deploy)
            .with_timeout(Duration::ZERO)
            .scan(request())
            .await,
        Err(ImageArtifactAdapterError::Timeout(_))
    ));
}
