use super::*;

#[tokio::test]
async fn sdk_artifact_scan_distinguishes_empty_and_unavailable_metadata() {
    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    let empty = SdkArtifactAdapter::new(root.clone())
        .scan(request(root.clone()))
        .await
        .unwrap();
    assert_eq!(empty.outcome, SdkArtifactScanOutcome::Empty);

    fs::write(root.join("poky-toolchain.sh"), b"installer").unwrap();
    let response = SdkArtifactAdapter::new(root.clone())
        .scan(request(root))
        .await
        .unwrap();
    let artifact = response.outcome.artifacts().first().unwrap();
    assert_eq!(artifact.sdk_kind, None);
    assert_eq!(artifact.machine, None);
    assert_eq!(artifact.host_tuple, None);
    assert_eq!(artifact.target_tuple, None);
    assert_eq!(artifact.published, None);
}
