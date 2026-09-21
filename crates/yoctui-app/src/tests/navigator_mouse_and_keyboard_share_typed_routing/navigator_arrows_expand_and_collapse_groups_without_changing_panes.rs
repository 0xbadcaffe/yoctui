use super::*;

#[test]
fn navigator_arrows_expand_and_collapse_groups_without_changing_panes() {
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Right),
        Some(Action::ExpandNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Left),
        Some(Action::CollapseNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Enter),
        Some(Action::ActivateNavigator)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
}
