use super::*;

#[tokio::test]
async fn image_artifact_adapter_scans_and_classifies_deterministically() {
    let fixture = fixture();
    let deploy = fixture.path().join("qemux86-64");
    fs::create_dir(&deploy).unwrap();
    fs::write(
        deploy.join("core-image-minimal-qemux86-64.rootfs.ext4"),
        b"rootfs",
    )
    .unwrap();
    fs::write(
        deploy.join("core-image-minimal-qemux86-64.manifest"),
        b"busybox",
    )
    .unwrap();
    fs::write(deploy.join("core-image-minimal-qemux86-64.wic"), b"wic").unwrap();
    fs::write(
        deploy.join("core-image-minimal-qemux86-64.rootfs.ext4.sha256"),
        b"abcdef  core-image-minimal-qemux86-64.rootfs.ext4\n",
    )
    .unwrap();

    let response = ImageArtifactAdapter::new(deploy.clone())
        .scan(request())
        .await
        .unwrap();
    let paths = response
        .inventory
        .artifacts
        .iter()
        .map(|artifact| artifact.identity.path.clone())
        .collect::<Vec<_>>();
    assert!(paths.windows(2).all(|pair| pair[0] < pair[1]));
    let rootfs = response
        .inventory
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == ImageArtifactKind::RootFilesystem)
        .unwrap();
    assert_eq!(rootfs.identity.image, "core-image-minimal");
    assert!(matches!(
        rootfs.checksums,
        ImageArtifactField::Available(ref checksums) if checksums.len() == 1
    ));
    assert!(matches!(
        rootfs.manifests,
        ImageArtifactField::Available(ref paths) if paths.len() == 1
    ));
    assert!(matches!(
        rootfs.wic_files,
        ImageArtifactField::Available(ref paths) if paths.len() == 1
    ));
}
