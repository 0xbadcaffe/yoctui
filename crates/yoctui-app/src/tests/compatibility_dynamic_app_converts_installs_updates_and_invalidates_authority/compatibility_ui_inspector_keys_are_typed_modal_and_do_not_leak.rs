use super::*;

#[test]
fn compatibility_ui_inspector_keys_are_typed_modal_and_do_not_leak() {
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('1')),
        Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::All
        ))
    );
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('5')),
        Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Attention
        ))
    );
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('/')),
        Some(Action::BeginCompatibilitySearch)
    );
    assert_eq!(
        compatibility_ui_inspector_action(true, Input::Char('x')),
        Some(Action::AppendCompatibilityQuery('x'))
    );
    assert_eq!(
        compatibility_ui_inspector_action(true, Input::Backspace),
        Some(Action::BackspaceCompatibilityQuery)
    );
    assert_eq!(
        compatibility_ui_inspector_action(true, Input::Esc),
        Some(Action::FinishCompatibilitySearch)
    );
    assert_eq!(compatibility_ui_inspector_action(false, Input::Tab), None);
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('q')),
        None
    );
}
