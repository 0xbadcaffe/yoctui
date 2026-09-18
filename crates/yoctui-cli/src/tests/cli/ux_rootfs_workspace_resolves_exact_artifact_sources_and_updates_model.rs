use super::*;

#[tokio::test]
async fn ux_rootfs_workspace_resolves_exact_artifact_sources_and_updates_model() {
    let build = std::env::temp_dir().join(format!(
        "yoctui-rootfs-workspace-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let deploy = build.join("tmp/deploy/images/qemux86-64");
    let pkgdata = build.join("tmp/pkgdata/qemux86-64/runtime");
    let rootfs = build.join("tmp/work/qemux86-64/core-image-minimal/1.0/rootfs");
    fs::create_dir_all(&deploy).unwrap();
    fs::create_dir_all(&pkgdata).unwrap();
    fs::create_dir_all(rootfs.join("bin")).unwrap();
    let image_path = deploy.join("core-image-minimal.rootfs.ext4");
    let manifest = deploy.join("core-image-minimal.rootfs.manifest");
    fs::write(&image_path, b"image").unwrap();
    fs::write(&manifest, "busybox qemux86-64 1.0\n").unwrap();
    fs::write(
        pkgdata.join("busybox"),
        "PN: busybox\nSECTION: base\nPKGSIZE: 7\nFILES_INFO: {\"/bin/busybox\":{}}\n",
    )
    .unwrap();
    fs::write(rootfs.join("bin/busybox"), b"busybox").unwrap();
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: image_path,
    };
    let artifact = yoctui_model::ImageArtifact {
        identity: identity.clone(),
        kind: yoctui_model::ImageArtifactKind::RootFilesystem,
        size_bytes: yoctui_model::ImageArtifactField::Available(5),
        modified_unix_seconds: yoctui_model::ImageArtifactField::Unavailable,
        checksums: yoctui_model::ImageArtifactField::Unavailable,
        manifests: yoctui_model::ImageArtifactField::Available(vec![manifest.clone()]),
        licenses: yoctui_model::ImageArtifactField::Unavailable,
        spdx: yoctui_model::ImageArtifactField::Unavailable,
        wic_files: yoctui_model::ImageArtifactField::Unavailable,
    };
    let mut app = App::new(10, 1_000);
    app.image_artifacts = yoctui_model::ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: yoctui_model::ImageArtifactField::Available(deploy),
            artifacts: vec![artifact],
        },
    };
    app.image_artifact_selection = Some(identity);
    app.workspace.variables.insert(
        "PKGDATA_DIR".into(),
        build
            .join("tmp/pkgdata/qemux86-64")
            .to_string_lossy()
            .into_owned(),
    );
    app.workspace
        .variables
        .insert("IMAGE_ROOTFS".into(), rootfs.to_string_lossy().into_owned());
    let effect = update(&mut app, Action::BeginSelectedRootfsComposition).unwrap();
    let mut operation = None;
    let Effect::GetRootfsComposition(request) = effect else {
        unreachable!()
    };
    begin_rootfs_composition_operation_with_sources(
        &mut app,
        &build,
        &mut operation,
        request.clone(),
        RootfsCompositionSources {
            image: request.image,
            manifest: Some(manifest),
            pkgdata_directory: Some(build.join("tmp/pkgdata/qemux86-64")),
            image_rootfs: Some(rootfs),
        },
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_rootfs_composition_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let composition = app.rootfs_composition.composition().unwrap();
    assert_eq!(
        composition.package_inventory().unwrap().packages[0]
            .identity
            .name,
        "busybox"
    );
    assert!(
        composition
            .filesystem_tree()
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.identity.0 == Path::new("/bin/busybox"))
    );
    fs::remove_dir_all(build).unwrap();
}
