use super::*;

#[test]
fn image_console_terminal_kinds_keep_protocol_identity() {
    assert_eq!(
        wire_terminal_kind(yoctui_model::TerminalCreationKind::QemuConsole),
        yoctui_protocol::daemon::PtyKind::QemuConsole
    );
    assert_eq!(
        wire_terminal_kind(yoctui_model::TerminalCreationKind::SshConsole),
        yoctui_protocol::daemon::PtyKind::SshConsole
    );
}
