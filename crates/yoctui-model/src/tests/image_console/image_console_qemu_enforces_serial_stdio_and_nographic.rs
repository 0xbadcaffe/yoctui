use super::*;

#[test]
fn image_console_qemu_zstd_filesystem_uses_snapshot_in_both_launch_routes() {
    let mut image = image();
    image.path = "/deploy/core-image-minimal-qemux86-64.rootfs-20261010172613.ext4.zst".into();
    let capability = QemuCapability::Available {
        executable: "/opt/poky/runqemu".into(),
        compatible_images: vec![image.clone()],
    };
    let draft = ImageConsoleDraft::for_artifact(image.clone(), ImageArtifactKind::RootFilesystem);
    let preview = draft
        .preview(&capability, &SshClientCapability::Missing)
        .unwrap();
    assert!(
        preview
            .arguments
            .iter()
            .any(|argument| argument == "snapshot")
    );
    let advanced = QemuLaunchDraft::for_artifact(image, ImageArtifactKind::RootFilesystem);
    assert_eq!(
        advanced
            .preview(&capability)
            .unwrap()
            .request
            .arguments()
            .iter()
            .filter(|argument| **argument == PathBuf::from("snapshot"))
            .count(),
        1
    );
}

#[test]
fn image_console_qemu_enforces_serial_stdio_and_nographic() {
    let image = image();
    let draft = ImageConsoleDraft::for_artifact(image.clone(), ImageArtifactKind::RootFilesystem);
    let preview = draft
        .preview(
            &QemuCapability::Available {
                executable: "/opt/poky/runqemu".into(),
                compatible_images: vec![image],
            },
            &SshClientCapability::Missing,
        )
        .unwrap();
    assert_eq!(preview.kind, TerminalCreationKind::QemuConsole);
    assert!(
        preview
            .arguments
            .iter()
            .any(|argument| argument == "nographic")
    );
    assert!(
        preview
            .arguments
            .iter()
            .any(|argument| argument == "serialstdio")
    );
}

#[test]
fn image_console_qemu_pins_openbmc_flash_config_and_preserves_deployed_image() {
    let image = ImageArtifactIdentity {
        machine: "romulus".into(),
        image: "obmc-phosphor-image".into(),
        path: "/deploy/obmc-phosphor-image-romulus-20261009022649.static.mtd".into(),
    };
    let draft = ImageConsoleDraft::for_artifact(image.clone(), ImageArtifactKind::RootFilesystem);
    assert_eq!(draft.networking, QemuNetworkingMode::None);
    assert_eq!(draft.memory_mib, "512");
    let preview = draft
        .preview(
            &QemuCapability::Available {
                executable: "/opt/runqemu".into(),
                compatible_images: vec![image.clone()],
            },
            &SshClientCapability::Missing,
        )
        .unwrap();
    assert_eq!(preview.arguments[0], image.path.display().to_string());
    assert_eq!(
        preview.arguments[1],
        "/deploy/obmc-phosphor-image-romulus-20261009022649.qemuboot.conf"
    );
    for argument in [
        "snapshot",
        "nonetwork",
        "nographic",
        "serialstdio",
        "qemuparams=-m 512",
    ] {
        assert!(preview.arguments.iter().any(|value| value == argument));
    }
}
