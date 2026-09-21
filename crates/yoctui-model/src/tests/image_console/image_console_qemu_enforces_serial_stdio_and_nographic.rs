use super::*;

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
