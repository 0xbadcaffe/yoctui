use super::*;

#[test]
fn raw_category_input_routes_bounded_browser_columns() {
    let state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    assert_eq!(
        raw_mode_input(&state, Input::Char('j')),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: 1 })
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('k')),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: -1 })
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('l')),
        Some(yoctui_model::RawModeAction::FocusCommands)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::FocusCommands)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('h')),
        Some(yoctui_model::RawModeAction::FocusCategories)
    );
}
