use super::*;

#[test]
fn image_console_ssh_is_argv_only_and_keeps_host_key_defaults() {
    let mut draft = ImageConsoleDraft::for_artifact(image(), ImageArtifactKind::RootFilesystem);
    draft.mode = ImageConsoleMode::Ssh;
    draft.host = "192.0.2.44".into();
    draft.user = "root".into();
    draft.port = "2222".into();
    draft.identity_file = "/workspace/keys/id_ed25519".into();
    let preview = draft
        .preview(
            &QemuCapability::MissingTool,
            &SshClientCapability::Available {
                executable: "/usr/bin/ssh".into(),
            },
        )
        .unwrap();
    assert_eq!(preview.kind, TerminalCreationKind::SshConsole);
    assert_eq!(preview.program, PathBuf::from("/usr/bin/ssh"));
    assert_eq!(
        preview.arguments,
        [
            "-t",
            "-p",
            "2222",
            "-i",
            "/workspace/keys/id_ed25519",
            "root@192.0.2.44"
        ]
    );
    assert!(!preview.arguments.iter().any(|argument| {
        argument.contains("StrictHostKeyChecking") || argument.contains("UserKnownHostsFile")
    }));
}
