use super::*;

#[test]
fn double_prefix_is_a_literal_control_b() {
    let now = Instant::now();
    let mut state = PrefixState::default();
    state.feed(Input::CtrlB, now);
    assert_eq!(
        state.feed(Input::CtrlB, now),
        PrefixEvent::Literal(Input::CtrlB)
    );
}
