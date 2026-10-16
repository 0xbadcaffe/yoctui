use super::*;

#[test]
fn raw_favorite_category_opens_saved_defaults_and_escaped_arguments() {
    let catalog = catalog(1);
    let mut state = RawModeState::new(&catalog);
    state
        .favorites
        .push(favorite_fixture(&catalog, "build.target", 0));
    let expected = state.favorites[0].clone();
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::ReprojectCatalog);
    let authority = authority(1, true);
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::OpenSelected,
    );
    assert_eq!(state.view, RawModeView::Form);
    assert_eq!(
        state.form.as_ref().unwrap().fields[&parameter("target")].value,
        expected
            .parameter_defaults
            .get(&parameter("target"))
            .cloned()
    );
    assert_eq!(
        RawAdditionalArguments::parse(
            &state
                .form
                .as_ref()
                .unwrap()
                .additional_arguments
                .editor
                .text
        )
        .unwrap(),
        expected.additional_arguments
    );
    reduce_raw_mode(&mut state, &catalog, Some(&authority), RawModeAction::Back);
    assert_eq!(state.view, RawModeView::Browser);
}

#[test]
fn raw_favorite_rename_validates_and_removal_requires_confirmation() {
    let catalog = catalog(1);
    let mut state = RawModeState::new(&catalog);
    state
        .favorites
        .push(favorite_fixture(&catalog, "build.target", 0));
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::OpenFavorites);
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::BeginFavoriteRename,
    );
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::ClearFavoriteName);
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::ConfirmFavoriteRename,
    );
    assert!(
        state.favorite_rename.is_some(),
        "Invalid names keep the editor open"
    );
    for character in "Poky café".chars() {
        reduce_raw_mode(
            &mut state,
            &catalog,
            None,
            RawModeAction::AppendFavoriteName(character),
        );
    }
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::ConfirmFavoriteRename,
    );
    assert_eq!(state.favorites[0].name, "Poky café");
    assert!(!state.modal_active());
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::RemoveFavorite);
    assert!(state.modal_active());
    assert_eq!(state.favorites.len(), 1);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::CancelFavorite);
    assert_eq!(state.favorites.len(), 1);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::RemoveFavorite);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::ConfirmFavorite);
    assert!(state.favorites.is_empty());
}
