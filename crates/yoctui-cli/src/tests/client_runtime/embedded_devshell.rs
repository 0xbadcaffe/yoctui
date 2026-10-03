use super::*;
use yoctui_model::TerminalCreationKind as Kind;

#[test]
fn embedded_devshell_and_menuconfig_wire_commands_reuse_the_validated_relay() {
    for (kind, task, wire_kind) in [
        (
            Kind::Devshell,
            "devshell",
            yoctui_protocol::daemon::PtyKind::Devshell,
        ),
        (
            Kind::Menuconfig,
            "menuconfig",
            yoctui_protocol::daemon::PtyKind::Menuconfig,
        ),
    ] {
        let argv = vec![
            "bitbake".into(),
            "recipe with spaces".into(),
            "-c".into(),
            task.into(),
        ];
        let command =
            embedded_terminal_command(kind, std::path::Path::new("/usr/bin/env"), &argv).unwrap();
        assert!(std::path::Path::new(&command.program).is_absolute());
        assert_eq!(
            command.arguments[..4],
            ["__menuconfig-relay", "--bitbake", "/usr/bin/env", "--"]
        );
        assert_eq!(command.arguments[4..], argv);
        assert!(command.environment_profile_id.is_none());
        assert_eq!(wire_terminal_kind(kind), wire_kind);
    }
}

#[test]
fn embedded_devshell_relay_does_not_change_other_typed_terminal_commands() {
    for kind in [
        Kind::BuildShell,
        Kind::DevtoolShell,
        Kind::Utility,
        Kind::GitUi,
        Kind::QemuConsole,
        Kind::SshConsole,
    ] {
        let argv = vec![
            "literal;not-a-shell-command".into(),
            "space argument".into(),
        ];
        let command =
            embedded_terminal_command(kind, std::path::Path::new("/selected/tool"), &argv).unwrap();
        assert_eq!(command.program, "/selected/tool");
        assert_eq!(command.arguments, argv);
        assert!(command.environment_profile_id.is_none());
    }
}
