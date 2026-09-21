use super::*;

#[test]
fn devtool_publish_update_routes_only_confirmation_keys() {
    assert_eq!(
        devtool_update_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolUpdateRecipe)
    );
    assert_eq!(
        devtool_update_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolUpdateRecipe)
    );
    assert_eq!(devtool_update_confirmation_action(Input::Char('u')), None);
}
