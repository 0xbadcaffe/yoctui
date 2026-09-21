use super::*;

#[tokio::test]
async fn sdk_artifact_scan_sorts_classifies_and_associates_records() {
    let fixture = fixture();
    let root = fixture.path().join("sdk");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("poky-toolchain.sh.target.manifest"), b"target").unwrap();
    fs::write(root.join("poky-toolchain.sh.sha256"), b"digest").unwrap();
    fs::write(root.join("poky-toolchain.host.manifest"), b"host").unwrap();
    fs::write(root.join("poky-toolchain.sh"), b"installer").unwrap();
    fs::write(root.join("README.txt"), b"other").unwrap();

    let response = SdkArtifactAdapter::new(root.clone())
        .scan(request(root))
        .await
        .unwrap();
    let SdkArtifactScanOutcome::Complete(artifacts) = response.outcome else {
        panic!("expected complete SDK inventory");
    };
    assert!(
        artifacts
            .windows(2)
            .all(|pair| pair[0].identity < pair[1].identity)
    );
    let installer = artifacts
        .iter()
        .find(|artifact| artifact.kind == SdkArtifactKind::Installer)
        .unwrap();
    assert_eq!(installer.checksums.len(), 1);
    assert_eq!(installer.manifests.len(), 2);
    assert!(installer.identity.size_bytes > 0);
}
