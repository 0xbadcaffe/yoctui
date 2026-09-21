use super::*;

#[test]
fn image_console_wire_specs_preserve_qemu_and_ssh_session_kinds() {
    for (wire, model) in [
        (PtyKind::QemuConsole, PtySessionKind::QemuConsole),
        (PtyKind::SshConsole, PtySessionKind::SshConsole),
    ] {
        let spec = wire_spec(
            PtySessionId(17),
            "image console".into(),
            wire,
            "/tmp".into(),
            PtyCommand {
                program: "/bin/true".into(),
                arguments: Vec::new(),
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();
        assert_eq!(spec.kind, model);
    }
}
