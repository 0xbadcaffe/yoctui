use super::*;

#[tokio::test]
async fn sdk_artifact_scan_rejects_root_symlink_mismatch_and_entry_escape() {
    use std::os::unix::fs::symlink;

    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    let missing = fixture.path().join("missing");
    assert!(matches!(
        SdkArtifactAdapter::new(missing.clone())
            .scan(request(missing))
            .await,
        Err(SdkArtifactAdapterError::MissingRoot(_))
    ));

    let linked = fixture.path().join("linked");
    symlink(&root, &linked).unwrap();
    assert!(matches!(
        SdkArtifactAdapter::new(linked.clone())
            .scan(request(linked))
            .await,
        Err(SdkArtifactAdapterError::SymlinkRoot(_))
    ));

    let other = fixture.path().join("other");
    fs::create_dir(&other).unwrap();
    assert!(matches!(
        SdkArtifactAdapter::new(root.clone())
            .scan(request(other))
            .await,
        Err(SdkArtifactAdapterError::RootMismatch { .. })
    ));

    let outside = fixture.path().join("outside.sh");
    fs::write(&outside, b"outside").unwrap();
    symlink(&outside, root.join("escaped.sh")).unwrap();
    let response = SdkArtifactAdapter::new(root.clone())
        .scan(request(root))
        .await
        .unwrap();
    assert!(matches!(
        response.outcome,
        SdkArtifactScanOutcome::Partial { .. }
    ));
    assert!(
        response
            .outcome
            .limitations()
            .iter()
            .any(|message| message.contains("symlink"))
    );
}
