use super::*;

#[tokio::test]
async fn sdk_artifact_scan_reports_partial_malformed_and_oversized_records() {
    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("valid.sh"), b"installer").unwrap();
    fs::write(root.join(".manifest"), b"malformed").unwrap();
    let long_name = format!("{}.txt", "x".repeat(MAX_SDK_NAME_BYTES));
    File::create(root.join(long_name))
        .unwrap()
        .write_all(b"x")
        .unwrap();

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
            .any(|message| { message.contains("malformed") || message.contains("oversized") })
    );
    assert_eq!(response.outcome.artifacts().len(), 1);
}
