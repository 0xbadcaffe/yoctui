use super::*;

#[test]
fn prefix_requires_a_second_key_and_resets_after_command() {
    let now = Instant::now();
    let mut state = PrefixState::new(Duration::from_secs(1));
    assert_eq!(state.feed(Input::CtrlB, now), PrefixEvent::Awaiting);
    assert!(state.pending());
    assert_eq!(
        state.feed(Input::Char('n'), now),
        PrefixEvent::Command(PrefixCommand::NextSession)
    );
    assert!(!state.pending());
}
