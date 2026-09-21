use super::*;

#[tokio::test]
async fn ux_rootfs_denies_stale_cancelled_mismatched_and_escaping_sources() {
    let (build, request, sources) = fixture();
    let adapter = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 5);
    assert!(matches!(
        adapter.scan(request.clone()).await,
        Err(RootfsCompositionAdapterError::StaleGeneration { .. })
    ));
    let cancellation = RootfsCompositionCancellation::default();
    cancellation.cancel();
    let adapter = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4);
    assert_eq!(
        adapter
            .scan_with_cancellation(request.clone(), cancellation)
            .await,
        Err(RootfsCompositionAdapterError::Cancelled)
    );
    let mut mismatch = request.clone();
    mismatch.image.image = "another-image".into();
    assert_eq!(
        adapter.scan(mismatch).await,
        Err(RootfsCompositionAdapterError::ImageMismatch)
    );
    let outside = std::env::temp_dir().join(format!("yoctui-outside-{}", std::process::id()));
    fs::create_dir_all(&outside).unwrap();
    let mut escaping = sources;
    escaping.image_rootfs = Some(outside.clone());
    let adapter = RootfsCompositionAdapter::new(build.clone(), escaping, 4);
    assert!(matches!(
        adapter.scan(request).await,
        Err(RootfsCompositionAdapterError::PathEscape(_))
    ));
    fs::remove_dir_all(build).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
