use super::*;

#[tokio::test]
async fn image_artifact_adapter_reports_empty_partial_malformed_and_oversized_inputs() {
    let fixture = fixture();
    let deploy = fixture.path().join("qemux86-64");
    fs::create_dir(&deploy).unwrap();
    let empty = ImageArtifactAdapter::new(deploy.clone())
        .scan(request())
        .await
        .unwrap();
    assert!(empty.inventory.artifacts.is_empty());
    assert!(empty.limitations.is_empty());

    fs::write(deploy.join("image-qemux86-64.ext4"), b"image").unwrap();
    fs::write(deploy.join("image-qemux86-64.ext4.sha256"), b"not-a-record").unwrap();
    let mut oversized = File::create(deploy.join("image-qemux86-64.ext4.md5")).unwrap();
    oversized.set_len(MAX_CHECKSUM_FILE_BYTES + 1).unwrap();
    oversized.flush().unwrap();
    fs::create_dir(deploy.join("nested")).unwrap();
    let partial = ImageArtifactAdapter::new(deploy)
        .scan(request())
        .await
        .unwrap();
    assert!(
        partial
            .limitations
            .iter()
            .any(|message| message.contains("malformed checksum"))
    );
    assert!(
        partial
            .limitations
            .iter()
            .any(|message| message.contains("exceeded scan bounds"))
    );
    assert!(
        partial
            .limitations
            .iter()
            .any(|message| message.contains("depth limit"))
    );
}
