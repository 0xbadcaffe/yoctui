use super::*;

#[test]
fn devtool_target_reset_routes_only_destructive_confirmation_keys() {
    assert_eq!(
        devtool_reset_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolReset)
    );
    assert_eq!(
        devtool_reset_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolReset)
    );
    assert_eq!(devtool_reset_confirmation_action(Input::Char('D')), None);
}
