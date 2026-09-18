use super::*;

#[test]
fn daemon_cli_commands_are_typed_and_destructive_session_kill_is_explicit() {
    let attach = Cli::try_parse_from(["yoctui", "attach"]).unwrap();
    assert!(matches!(attach.command, Some(Command::Attach)));
    let sessions = Cli::try_parse_from(["yoctui", "sessions"]).unwrap();
    assert!(matches!(sessions.command, Some(Command::Sessions)));
    let kill = Cli::try_parse_from(["yoctui", "session", "kill", "7"]).unwrap();
    assert!(matches!(
        kill.command,
        Some(Command::Session {
            command: SessionCliCommand::Kill {
                id: 7,
                force: false
            }
        })
    ));
}
