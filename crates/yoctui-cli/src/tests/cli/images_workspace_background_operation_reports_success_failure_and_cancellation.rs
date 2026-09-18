use super::*;

#[tokio::test]
async fn images_workspace_background_operation_reports_success_failure_and_cancellation() {
    let directory = std::env::temp_dir().join(format!(
        "yoctui-images-workspace-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let deploy = directory.join("qemux86-64");
    fs::create_dir_all(&deploy).unwrap();
    fs::write(
        deploy.join("core-image-minimal-qemux86-64.rootfs.ext4"),
        b"image",
    )
    .unwrap();
    let adapter = ImageArtifactAdapter::new(deploy);
    let qemu_inspector =
        QemuCapabilityInspector::with_executable(directory.join("missing-runqemu"));
    let wic_inspector = WicCapabilityInspector::with_executable(directory.join("missing-wic"));
    let mut wic_capability_operation = None;
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let effect = update(&mut app, Action::BeginImageArtifactInventory).unwrap();
    let mut operation = None;
    begin_image_artifact_operation(&mut app, Some(&adapter), &mut operation, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_image_artifact_operation(
                &mut app,
                &mut operation,
                &qemu_inspector,
                &wic_inspector,
                &mut wic_capability_operation,
            )
            .await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.image_artifacts,
        yoctui_model::ImageArtifactInventoryState::Available { .. }
    ));

    let effect = update(&mut app, Action::RefreshImageArtifactInventory).unwrap();
    begin_image_artifact_operation(&mut app, None, &mut operation, effect);
    assert!(matches!(
        app.image_artifacts,
        yoctui_model::ImageArtifactInventoryState::Failed { ref message, .. }
            if message.contains("DEPLOY_DIR_IMAGE")
    ));

    let effect = update(&mut app, Action::RefreshImageArtifactInventory).unwrap();
    begin_image_artifact_operation(&mut app, Some(&adapter), &mut operation, effect);
    assert!(
        operation
            .as_ref()
            .is_some_and(|operation| operation.cancellation.cancel())
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_image_artifact_operation(
                &mut app,
                &mut operation,
                &qemu_inspector,
                &wic_inspector,
                &mut wic_capability_operation,
            )
            .await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.image_artifacts,
        yoctui_model::ImageArtifactInventoryState::Failed { ref message, .. }
            if message.contains("cancelled")
    ));
    if let Some(operation) = wic_capability_operation {
        operation.handle.abort();
    }
    fs::remove_dir_all(directory).unwrap();
}
