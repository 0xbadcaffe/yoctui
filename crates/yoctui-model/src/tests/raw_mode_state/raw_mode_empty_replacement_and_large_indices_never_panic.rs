use super::*;

#[test]
fn raw_mode_empty_replacement_and_large_indices_never_panic() {
    let catalog = catalog(1);
    let mut state = RawModeState::new(&catalog);
    let empty = RawCatalog {
        version: 2,
        categories: Vec::new(),
        commands: Vec::new(),
    };
    reduce_raw_mode(&mut state, &empty, None, RawModeAction::ReprojectCatalog);
    reduce_raw_mode(
        &mut state,
        &empty,
        None,
        RawModeAction::SelectCategory { delta: isize::MAX },
    );
    reduce_raw_mode(
        &mut state,
        &empty,
        None,
        RawModeAction::SelectCommand { delta: isize::MIN },
    );
    assert!(state.category.is_none());
    assert!(state.command.is_none());
    assert_eq!(state.history_selection, 0);
    assert_eq!(state.favorite_selection, 0);
}
