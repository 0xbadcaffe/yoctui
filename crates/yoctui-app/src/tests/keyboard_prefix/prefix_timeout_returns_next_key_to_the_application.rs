use super::*;

#[test]
fn prefix_timeout_returns_next_key_to_the_application() {
    let now = Instant::now();
    let mut state = PrefixState::new(Duration::from_millis(10));
    assert_eq!(state.feed(Input::CtrlB, now), PrefixEvent::Awaiting);
    assert_eq!(
        state.feed(Input::Char('x'), now + Duration::from_millis(11)),
        PrefixEvent::Literal(Input::Char('x'))
    );
    assert!(!state.pending());
}
