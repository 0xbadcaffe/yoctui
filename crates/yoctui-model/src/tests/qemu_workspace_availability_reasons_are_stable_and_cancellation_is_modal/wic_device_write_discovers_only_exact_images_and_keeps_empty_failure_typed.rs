use super::*;

#[test]
fn wic_device_write_discovers_only_exact_images_and_keeps_empty_failure_typed() {
    let mut app = qemu_model_app();
    app.wic_capability = wic_model_capability();
    let Some(Effect::GetWicDevices(request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("deployed Wic discovery");
    };
    assert!(request.image.path.ends_with("core-image-minimal.wic"));
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: request.clone(),
            devices: Vec::new(),
            limitations: Vec::new(),
        },
    );
    assert!(matches!(
        app.wic_devices,
        WicDeviceInventoryState::Available {
            ref devices,
            ..
        } if devices.is_empty()
    ));
    assert!(app.wic_device_selection.is_none());
    assert!(update(&mut app, Action::ConfirmWicDeviceSelection).is_none());
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicDevicePicker(_))
    ));
    let _ = update(&mut app, Action::CancelWicDevicePicker);
    let Some(Effect::GetWicDevices(second_request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("second discovery");
    };
    assert!(second_request.generation > request.generation);
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryFailed {
            request: second_request.clone(),
            message: "root identity ambiguous".into(),
        },
    );
    assert!(matches!(
        &app.wic_devices,
        WicDeviceInventoryState::Failed { request, message }
            if request == &second_request && message == "root identity ambiguous"
    ));

    let mut compressed = qemu_model_app();
    compressed.wic_capability = wic_model_capability();
    if let ImageArtifactInventoryState::Available { inventory, .. } =
        &mut compressed.image_artifacts
    {
        inventory.artifacts[0].identity.path =
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic.gz".into();
        compressed.image_artifact_selection = Some(inventory.artifacts[0].identity.clone());
    }
    assert!(update(&mut compressed, Action::BeginSelectedWicDeviceWrite).is_none());

    let mut direct = qemu_model_app();
    direct.wic_capability = wic_model_capability();
    if let ImageArtifactInventoryState::Available { inventory, .. } = &mut direct.image_artifacts {
        inventory.artifacts[0].identity.path =
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.direct".into();
        direct.image_artifact_selection = Some(inventory.artifacts[0].identity.clone());
    }
    assert!(matches!(
        update(&mut direct, Action::BeginSelectedWicDeviceWrite),
        Some(Effect::GetWicDevices(WicDeviceInventoryRequest {
            image: WicOutputIdentity { ref path, .. },
            ..
        })) if path.ends_with("core-image-minimal.direct")
    ));
}
