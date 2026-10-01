use super::*;

mod prefix_requires_a_second_key_and_resets_after_command;

mod prefix_timeout_returns_next_key_to_the_application;

mod double_prefix_is_a_literal_control_b;

mod ux_terminal_prefix_opens_the_terminal_workbench;

#[test]
fn editor_gitui_prefix_restores_editor_without_consuming_unprefixed_text() {
    let mut state = PrefixState::default();
    let now = Instant::now();
    assert_eq!(
        state.feed(Input::Char('e'), now),
        PrefixEvent::Literal(Input::Char('e'))
    );
    assert_eq!(state.feed(Input::CtrlB, now), PrefixEvent::Awaiting);
    assert_eq!(
        state.feed(Input::Char('e'), now),
        PrefixEvent::Command(PrefixCommand::RestoreEditor)
    );
    assert!(!state.pending());
}
