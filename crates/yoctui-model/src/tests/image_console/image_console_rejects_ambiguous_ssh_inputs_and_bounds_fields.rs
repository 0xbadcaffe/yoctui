use super::*;

#[test]
fn image_console_rejects_ambiguous_ssh_inputs_and_bounds_fields() {
    let mut dialog = ImageConsoleDialog::new(ImageConsoleDraft::for_artifact(
        image(),
        ImageArtifactKind::RootFilesystem,
    ));
    assert!(dialog.cycle_choice(false));
    assert_eq!(dialog.draft.mode, ImageConsoleMode::Ssh);
    dialog.draft.host = "-oProxyCommand=bad".into();
    assert!(
        dialog
            .draft
            .preview(
                &QemuCapability::MissingTool,
                &SshClientCapability::Available {
                    executable: "/usr/bin/ssh".into()
                }
            )
            .unwrap_err()
            .contains("SSH host")
    );
    dialog.draft.host = "target.example".into();
    dialog.draft.port = "0".into();
    assert!(
        dialog
            .draft
            .preview(
                &QemuCapability::MissingTool,
                &SshClientCapability::Available {
                    executable: "/usr/bin/ssh".into()
                }
            )
            .unwrap_err()
            .contains("SSH port")
    );
}
