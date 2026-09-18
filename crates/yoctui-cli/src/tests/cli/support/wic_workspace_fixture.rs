use super::*;

#[cfg(unix)]
pub(crate) async fn wic_workspace_fixture(
    name: &str,
    create_body: &str,
) -> (PathBuf, PathBuf, App) {
    let directory = std::env::temp_dir().join(format!(
        "yoctui-wic-workspace-{}-{name}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let build_dir = directory.join("build");
    let deploy = directory.join("deploy");
    fs::create_dir_all(&build_dir).unwrap();
    fs::create_dir_all(&deploy).unwrap();
    let executable = directory.join("wic");
    write_test_executable(
        &executable,
        &format!("#!/bin/sh\nif [ \"$1\" = \"list\" ]; then exit 0; fi\n{create_body}\n"),
    );
    let kickstart = directory.join("directdisk.wks");
    fs::write(
        &kickstart,
        "part / --source=rootfs --fstype=ext4 --size=64\n",
    )
    .unwrap();
    let image_path = deploy.join("core-image-minimal.ext4");
    fs::write(&image_path, b"rootfs").unwrap();
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: image_path,
    };
    let artifact = yoctui_model::ImageArtifact {
        identity: identity.clone(),
        kind: yoctui_model::ImageArtifactKind::RootFilesystem,
        size_bytes: yoctui_model::ImageArtifactField::Available(6),
        modified_unix_seconds: yoctui_model::ImageArtifactField::Unavailable,
        checksums: yoctui_model::ImageArtifactField::Unavailable,
        manifests: yoctui_model::ImageArtifactField::Unavailable,
        licenses: yoctui_model::ImageArtifactField::Unavailable,
        spdx: yoctui_model::ImageArtifactField::Unavailable,
        wic_files: yoctui_model::ImageArtifactField::Unavailable,
    };
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("WKS_FILE".into(), kickstart.display().to_string());
    app.image_artifact_selection = Some(identity);
    app.image_artifacts = ImageArtifactInventoryState::Available {
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
    let inspector = configure_wic_capability_inspector(
        &app,
        WicCapabilityInspector::with_executable(executable),
    );
    let effect = update(&mut app, Action::InspectWicCapability).unwrap();
    let mut capability_operation = None;
    begin_wic_capability_operation(&mut app, &inspector, &mut capability_operation, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while capability_operation.is_some() {
            poll_wic_capability_operation(&mut app, &inspector, &mut capability_operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        matches!(app.wic_capability, WicCapability::Available { .. }),
        "{:?}",
        app.wic_capability
    );
    (directory, build_dir, app)
}
