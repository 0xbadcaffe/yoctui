use super::*;

#[tokio::test]
async fn image_artifact_adapter_rejects_symlink_missing_escape_and_machine_mismatch() {
    use std::os::unix::fs::symlink;

    let fixture = fixture();
    let missing = fixture.path().join("qemux86-64");
    assert!(matches!(
        ImageArtifactAdapter::new(missing).scan(request()).await,
        Err(ImageArtifactAdapterError::InvalidDeployDirectory(_))
    ));

    let real = fixture.path().join("real");
    fs::create_dir(&real).unwrap();
    let linked = fixture.path().join("qemux86-64");
    symlink(&real, &linked).unwrap();
    assert!(matches!(
        ImageArtifactAdapter::new(linked).scan(request()).await,
        Err(ImageArtifactAdapterError::SymlinkDeployDirectory(_))
    ));

    let wrong = fixture.path().join("qemuarm64");
    fs::create_dir(&wrong).unwrap();
    assert!(matches!(
        ImageArtifactAdapter::new(wrong).scan(request()).await,
        Err(ImageArtifactAdapterError::MachineMismatch { .. })
    ));

    let deploy = fixture.path().join("machine");
    fs::create_dir(&deploy).unwrap();
    let outside = fixture.path().join("outside");
    fs::write(&outside, b"outside").unwrap();
    symlink(&outside, deploy.join("escaped")).unwrap();
    let mut machine_request = request();
    machine_request.machine = "machine".into();
    let response = ImageArtifactAdapter::new(deploy)
        .scan(machine_request)
        .await
        .unwrap();
    assert!(
        response
            .limitations
            .iter()
            .any(|message| message.contains("symlink deploy entry"))
    );
}
