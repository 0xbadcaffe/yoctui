use super::*;

#[test]
fn qemu_adapter_capability_distinguishes_available_missing_image_and_failure() {
    let directory = fixture_dir("capability");
    let program = directory.join("runqemu");
    executable(&program, "exit 0");
    let deploy = directory.join("qemux86-64");
    fs::create_dir(&deploy).unwrap();
    let image_path = deploy.join("core-image-minimal.wic");
    fs::write(&image_path, b"wic").unwrap();
    let image = artifact(image_path, ImageArtifactKind::Wic);
    assert!(matches!(
        QemuCapabilityInspector::with_executable(program.clone())
            .inspect(std::slice::from_ref(&image)),
        QemuCapability::Available {
            compatible_images,
            ..
        } if compatible_images == vec![image.identity.clone()]
    ));
    assert_eq!(
        QemuCapabilityInspector::with_executable(directory.join("missing")).inspect(&[]),
        QemuCapability::MissingTool
    );
    assert_eq!(
        QemuCapabilityInspector::with_executable("definitely-missing-runqemu".into()).inspect(&[]),
        QemuCapability::MissingTool
    );
    assert_eq!(
        QemuCapabilityInspector::with_executable(program.clone()).inspect(&[]),
        QemuCapability::MissingCompatibleImage
    );
    let stale = artifact(deploy.join("missing.wic"), ImageArtifactKind::Wic);
    assert!(matches!(
        QemuCapabilityInspector::with_executable(program).inspect(&[stale]),
        QemuCapability::Failed { message } if message.contains("does not exist")
    ));
    fs::remove_dir_all(directory).unwrap();
}
