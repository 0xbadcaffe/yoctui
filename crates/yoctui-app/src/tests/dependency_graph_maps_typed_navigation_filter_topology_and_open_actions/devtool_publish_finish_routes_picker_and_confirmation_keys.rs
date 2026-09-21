use super::*;

#[test]
fn devtool_publish_finish_routes_picker_and_confirmation_keys() {
    assert_eq!(
        devtool_finish_picker_action(Input::Up),
        Some(Action::SelectDevtoolFinishLayer { delta: -1 })
    );
    assert_eq!(
        devtool_finish_picker_action(Input::Down),
        Some(Action::SelectDevtoolFinishLayer { delta: 1 })
    );
    assert_eq!(
        devtool_finish_picker_action(Input::Enter),
        Some(Action::PreviewDevtoolFinish)
    );
    assert_eq!(
        devtool_finish_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolFinish)
    );
    assert_eq!(
        devtool_finish_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolFinishConfirmation)
    );
}
