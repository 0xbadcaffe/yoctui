use super::*;

#[tokio::test]
async fn sdk_artifact_scan_bounds_directory_entries_deterministically() {
    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    for index in (0..=MAX_DIRECTORY_ENTRIES).rev() {
        fs::write(root.join(format!("{index:05}.txt")), b"record").unwrap();
    }

    let first = SdkArtifactAdapter::new(root.clone())
        .scan(request(root.clone()))
        .await
        .unwrap();
    let second = SdkArtifactAdapter::new(root.clone())
        .scan(request(root))
        .await
        .unwrap();
    assert!(matches!(
        first.outcome,
        SdkArtifactScanOutcome::Partial { .. }
    ));
    assert_eq!(first.outcome.artifacts(), second.outcome.artifacts());
    assert_eq!(first.outcome.limitations(), second.outcome.limitations());
    assert_eq!(first.outcome.artifacts().len(), MAX_DIRECTORY_ENTRIES);
    assert!(
        first
            .outcome
            .artifacts()
            .iter()
            .all(|artifact| !artifact.identity.path.ends_with("04096.txt"))
    );
}
