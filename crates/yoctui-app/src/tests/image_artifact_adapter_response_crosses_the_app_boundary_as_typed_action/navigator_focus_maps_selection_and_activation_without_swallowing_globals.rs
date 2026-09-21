use super::*;

#[test]
fn navigator_focus_maps_selection_and_activation_without_swallowing_globals() {
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Down),
        Some(Action::SelectNavigator { delta: 1 })
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Enter),
        Some(Action::ActivateNavigator)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::CtrlP),
        None,
        "unmapped global input must continue to the shared key route"
    );
}
