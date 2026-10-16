fn edit_raw_favorite_name(state: &mut RawModeState, action: RawModeAction) {
    match action {
        RawModeAction::BeginFavoriteRename => {
            state.favorite_rename = state.favorites.get(state.favorite_selection)
                .map(|favorite| favorite.name.clone());
            state.notification = None;
        }
        RawModeAction::AppendFavoriteName(character) => {
            if let Some(name) = state.favorite_rename.as_mut()
                && !character.is_control() && name.len() + character.len_utf8() <= MAX_RAW_FAVORITE_NAME_BYTES {
                name.push(character);
            }
        }
        RawModeAction::BackspaceFavoriteName => {
            if let Some(name) = state.favorite_rename.as_mut() { name.pop(); }
        }
        RawModeAction::ClearFavoriteName => {
            if let Some(name) = state.favorite_rename.as_mut() { name.clear(); }
        }
        RawModeAction::ConfirmFavoriteRename => {
            if let Some(name) = state.favorite_rename.clone() {
                rename_raw_favorite(state, name.clone());
                if state.favorites.get(state.favorite_selection).is_some_and(|f| f.name == name) {
                    state.favorite_rename = None;
                }
            }
        }
        RawModeAction::CancelFavoriteRename => {
            state.favorite_rename = None;
            state.notification = None;
        }
        _ => {}
    }
}
