use super::*;

#[test]
fn ux_terminal_prefix_opens_the_terminal_workbench() {
    let now = Instant::now();
    let mut state = PrefixState::default();
    state.feed(Input::CtrlB, now);
    assert_eq!(
        state.feed(Input::Char('t'), now),
        PrefixEvent::Command(PrefixCommand::OpenTerminalSessions)
    );
    for (input, command) in [
        (Input::Char('['), PrefixCommand::CopyMode),
        (Input::Char('/'), PrefixCommand::Search),
        (Input::Char('r'), PrefixCommand::Rename),
        (Input::Char('O'), PrefixCommand::ReleaseControl),
        (Input::Char('K'), PrefixCommand::Kill),
        (Input::Char('z'), PrefixCommand::Zoom),
    ] {
        state.feed(Input::CtrlB, now);
        assert_eq!(state.feed(input, now), PrefixEvent::Command(command));
    }
}
