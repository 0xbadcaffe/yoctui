use super::*;

#[test]
fn exit_and_build_cancel_confirmations_trap_yes_no_and_enter() {
    assert_eq!(
        build_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmBuildCancellation)
    );
    assert_eq!(
        build_cancellation_confirmation_action(Input::Char('n')),
        Some(Action::CancelBuildCancellation)
    );
    assert_eq!(
        quit_confirmation_action(Input::Char('y')),
        Some(Action::ConfirmQuit)
    );
    assert_eq!(
        quit_confirmation_action(Input::Esc),
        Some(Action::CancelQuit)
    );
    assert_eq!(quit_confirmation_action(Input::Char('q')), None);
}
