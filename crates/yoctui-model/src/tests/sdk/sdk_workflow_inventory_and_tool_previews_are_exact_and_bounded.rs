use super::*;

#[test]
fn sdk_workflow_inventory_and_tool_previews_are_exact_and_bounded() {
    let request = SdkArtifactInventoryRequest {
        generation: 1,
        root: "/deploy/sdk".into(),
        machine: "qemux86-64".into(),
    };
    let artifact = SdkArtifact {
        identity: SdkArtifactIdentity {
            path: "/deploy/sdk/poky.sh".into(),
            size_bytes: 42,
            modified_unix_seconds: 7,
        },
        kind: SdkArtifactKind::Installer,
        sdk_kind: Some(SdkKind::Standard),
        machine: Some("qemux86-64".into()),
        host_tuple: Some("x86_64-pokysdk-linux".into()),
        target_tuple: Some("x86_64-poky-linux".into()),
        checksums: vec!["/deploy/sdk/poky.sh.sha256".into()],
        manifests: Vec::new(),
        published: None,
    };
    assert_eq!(
        normalize_sdk_artifacts(&request, vec![artifact.clone(), artifact])
            .unwrap()
            .len(),
        1
    );
    let publish = SdkPublishPreview::new(
        "/opt/poky/oe-publish-sdk".into(),
        SdkArtifactIdentity {
            path: "/deploy/sdk/poky.sh".into(),
            size_bytes: 42,
            modified_unix_seconds: 7,
        },
        "/srv/sdk".into(),
    )
    .unwrap();
    assert_eq!(publish.argv[1], Path::new("/deploy/sdk/poky.sh"));
    let native = SdkNativePreview::new(SdkNativeRequest {
        executable: "/opt/poky/oe-run-native".into(),
        mode: SdkNativeMode::RunNative,
        extracted_root: Some("/opt/sdk".into()),
        recipe: "cmake-native".into(),
        tool: Some("cmake".into()),
        arguments: vec!["--version".into()],
    })
    .unwrap();
    assert_eq!(
        native.argv,
        [
            "/opt/poky/oe-run-native",
            "cmake-native",
            "cmake",
            "--version"
        ]
        .map(PathBuf::from)
    );
}
