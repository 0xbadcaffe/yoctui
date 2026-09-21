use super::*;

#[tokio::test]
async fn sdk_artifact_scan_bounds_traversed_directories() {
    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    let mut directory = root.clone();
    for index in 0..=MAX_SDK_DIRECTORIES {
        directory = directory.join(format!("{index:03}"));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("record.txt"), b"record").unwrap();
    }

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
            .any(|message| message.contains("directories were omitted"))
    );
    assert_eq!(response.outcome.artifacts().len(), MAX_SDK_DIRECTORIES - 1);
}
