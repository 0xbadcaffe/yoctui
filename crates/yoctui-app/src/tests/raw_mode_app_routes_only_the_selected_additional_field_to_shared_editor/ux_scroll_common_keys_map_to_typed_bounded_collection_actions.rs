use super::*;

#[test]
fn ux_scroll_common_keys_map_to_typed_bounded_collection_actions() {
    assert_eq!(collection_scroll_delta(Input::Up), Some(-1));
    assert_eq!(collection_scroll_delta(Input::Char('j')), Some(1));
    assert_eq!(collection_scroll_delta(Input::PageUp), Some(-10));
    assert_eq!(collection_scroll_delta(Input::PageDown), Some(10));
    assert_eq!(collection_scroll_delta(Input::Home), Some(isize::MIN));
    assert_eq!(collection_scroll_delta(Input::End), Some(isize::MAX));
    assert_eq!(collection_scroll_delta(Input::Char('G')), Some(isize::MAX));

    let mut chord_app = yoctui_model::App::new(8, 1_000);
    assert_eq!(
        keymap_action_for_app(&mut chord_app, Input::Char('g')),
        KeymapInputResult::Pending
    );
    assert_eq!(
        keymap_action_for_app(&mut chord_app, Input::Char('g')),
        KeymapInputResult::Action(Box::new(Action::ScrollCurrent { to_end: false }))
    );

    assert_eq!(
        tasks_action(false, Input::PageDown),
        Some(Action::ScrollBuildTasks { delta: 10 })
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Home),
        Some(Action::SelectRecipe { delta: isize::MIN })
    );
    assert_eq!(
        logs_action(false, Input::End),
        Some(Action::ScrollLogs { delta: -isize::MAX })
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::PageDown),
        Some(Action::SelectNavigator { delta: 10 })
    );

    let state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    assert_eq!(
        raw_mode_input(&state, Input::PageDown),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: 10 })
    );
}
