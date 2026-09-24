use super::*;

#[test]
fn qemu_and_wic_launch_inspect_capability_then_resume() {
    let mut qemu = qemu_model_app();
    qemu.qemu_capability = QemuCapability::NotInspected;
    assert_eq!(
        update(&mut qemu, Action::BeginSelectedQemuLaunch),
        Some(Effect::InspectQemuCapability)
    );
    assert!(qemu.pending_qemu_launch);
    let identity = qemu_model_artifact().identity;
    let _ = update(
        &mut qemu,
        Action::QemuCapabilityLoaded(QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: vec![identity],
        }),
    );
    assert!(!qemu.pending_qemu_launch);
    assert!(matches!(qemu.active_dialog(), Some(Dialog::QemuLaunch(_))));

    let mut wic = qemu_model_app();
    wic.wic_capability = WicCapability::NotInspected;
    assert_eq!(
        update(&mut wic, Action::BeginSelectedWicCreate),
        Some(Effect::InspectWicCapability)
    );
    assert!(wic.pending_wic_create);
    let _ = update(
        &mut wic,
        Action::WicCapabilityLoaded(wic_model_capability()),
    );
    assert!(!wic.pending_wic_create);
    assert!(matches!(
        wic.active_dialog(),
        Some(Dialog::WicCreateTomlEditor { .. })
    ));
}
