use super::*;

#[test]
fn qemu_workspace_availability_reasons_are_stable_and_cancellation_is_modal() {
    let mut app = qemu_model_app();
    let mut unsupported = qemu_model_app();
    if let ImageArtifactInventoryState::Available { inventory, .. } =
        &mut unsupported.image_artifacts
    {
        inventory.artifacts[0].kind = ImageArtifactKind::Manifest;
    }
    assert_eq!(
        unsupported.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu requires a root filesystem or Wic artifact.")
    );
    app.qemu_capability = QemuCapability::NotInspected;
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu capability has not been inspected.")
    );
    app.qemu_capability = QemuCapability::MissingTool;
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu is not available.")
    );
    app.qemu_capability = QemuCapability::MissingCompatibleImage;
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("No compatible deployed runqemu image is available.")
    );
    app.qemu_capability = QemuCapability::Failed {
        message: "inspection denied".into(),
    };
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu capability inspection failed: inspection denied")
    );
    app.qemu_capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: Vec::new(),
    };
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("The selected artifact is not in the inspected runqemu capability.")
    );
    app.qemu_capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: vec![qemu_model_artifact().identity],
    };
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    let Some(Effect::StartQemuSession { id, .. }) = update(&mut app, Action::ConfirmQemuLaunch)
    else {
        panic!("start effect");
    };
    let _ = update(
        &mut app,
        Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::QemuSessionRunning { id });
    let _ = update(&mut app, Action::BeginActiveQemuSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuCancellationConfirmation(candidate)) if *candidate == id
    ));
    let _ = update(&mut app, Action::CancelQemuSessionCancellation);
    assert!(app.active_dialog().is_none());
    assert_eq!(
        app.background_jobs
            .get(app.qemu_session(id).expect("session").background_job_id)
            .map(|job| job.status),
        Some(BackgroundJobStatus::Running)
    );
}
