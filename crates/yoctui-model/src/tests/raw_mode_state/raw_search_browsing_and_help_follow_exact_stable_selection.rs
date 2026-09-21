use super::*;

#[test]
fn raw_search_browsing_and_help_follow_exact_stable_selection() {
    let catalog = catalog(1);
    let mut state = RawModeState::new(&catalog);
    assert_eq!(state.category, Some(category("favorites")));
    assert!(state.command.is_none());

    select_build_category(&mut state, &catalog);
    assert_eq!(state.command, Some(command("build.target")));
    assert_eq!(
        state.selected_command(&catalog).unwrap().description,
        "Build one exact target."
    );
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::SelectCommand { delta: 99 },
    );
    assert_eq!(state.command, Some(command("build.version")));

    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::BeginSearch);
    for character in "reference-only".chars() {
        reduce_raw_mode(
            &mut state,
            &catalog,
            None,
            RawModeAction::AppendSearch(character),
        );
    }
    assert_eq!(state.command, Some(command("reference.pipeline")));
    assert_eq!(state.visible_commands(&catalog).len(), 1);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::Back);
    assert!(!state.search.editing);
    assert_eq!(state.focus, RawModeFocus::Commands);
}
