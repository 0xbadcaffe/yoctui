use super::*;

#[test]
fn raw_search_app_maps_browser_history_and_favorite_input_mechanically() {
    let catalog = yoctui_model::RawCatalog::builtin();
    let mut state = yoctui_model::RawModeState::new(&catalog);
    assert_eq!(
        raw_mode_input(&state, Input::Right),
        Some(yoctui_model::RawModeAction::FocusCommands)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Down),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: 1 })
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('/')),
        Some(yoctui_model::RawModeAction::BeginSearch)
    );
    reduce_raw_mode_state(
        &mut state,
        &catalog,
        None,
        yoctui_model::RawModeAction::BeginSearch,
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('x')),
        Some(yoctui_model::RawModeAction::AppendSearch('x'))
    );
    assert_eq!(
        raw_mode_input(&state, Input::Esc),
        Some(yoctui_model::RawModeAction::FinishSearch)
    );

    state.search.editing = false;
    state.view = yoctui_model::RawModeView::History;
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::ActivateHistory)
    );
    state.view = yoctui_model::RawModeView::Favorites;
    assert_eq!(
        raw_mode_input(&state, Input::Down),
        Some(yoctui_model::RawModeAction::SelectFavorite { delta: 1 })
    );
    state.favorite_confirmation = Some(yoctui_model::RawFavoriteConfirmation {
        command: catalog.commands[0].id.clone(),
        return_focus: yoctui_model::RawModeFocus::Favorites,
    });
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::ConfirmFavorite)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Esc),
        Some(yoctui_model::RawModeAction::CancelFavorite)
    );
    assert_eq!(raw_mode_input(&state, Input::Down), None);
}
