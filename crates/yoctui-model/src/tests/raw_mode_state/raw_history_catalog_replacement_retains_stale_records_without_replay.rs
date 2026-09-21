use super::*;

#[test]
fn raw_history_catalog_replacement_retains_stale_records_without_replay() {
    let original = catalog(1);
    let mut state = RawModeState::new(&original);
    select_build_category(&mut state, &original);
    reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);
    state.history.push(RawHistoryRecord {
        schema_version: RAW_HISTORY_SCHEMA_VERSION,
        request_id: RawRequestId::new("raw-request:history-1").unwrap(),
        catalog_version: 1,
        command: command("build.target"),
        parameters: BTreeMap::new(),
        interaction: RawInteractionMode::NoninteractiveJob,
        started_unix_ms: 10,
        ended_unix_ms: 20,
        outcome: RawExecutionOutcome::Succeeded,
        exit_code: Some(0),
        durable_reference: None,
    });
    assert_eq!(state.favorites[0].command, command("build.target"));
    assert_eq!(state.history[0].command, command("build.target"));

    reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);
    assert!(state.favorite_confirmation.is_some());
    assert_eq!(state.favorites[0].command, command("build.target"));
    reduce_raw_mode(&mut state, &original, None, RawModeAction::CancelFavorite);
    assert_eq!(state.favorites[0].command, command("build.target"));
    reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);
    reduce_raw_mode(&mut state, &original, None, RawModeAction::ConfirmFavorite);
    assert!(state.favorites.is_empty());
    reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);

    reduce_raw_mode(&mut state, &original, None, RawModeAction::OpenFavorites);
    let available = authority(1, true);
    reduce_raw_mode(
        &mut state,
        &original,
        Some(&available),
        RawModeAction::ActivateFavorite,
    );
    assert_eq!(state.command, Some(command("build.target")));
    assert_eq!(state.view, RawModeView::Form);
    assert_eq!(state.focus, RawModeFocus::Form);

    let mut replacement = catalog(2);
    replacement
        .commands
        .retain(|item| item.id != command("build.target"));
    replacement = replacement.normalize().unwrap();
    reduce_raw_mode(
        &mut state,
        &replacement,
        None,
        RawModeAction::ReprojectCatalog,
    );
    assert_eq!(state.favorites.len(), 1);
    assert!(state.favorites[0].project(&replacement, None).stale);
    assert_eq!(state.history.len(), 1);
    assert_ne!(state.command, Some(command("build.target")));
    reduce_raw_mode(&mut state, &replacement, None, RawModeAction::OpenHistory);
    reduce_raw_mode(
        &mut state,
        &replacement,
        None,
        RawModeAction::ActivateHistory,
    );
    assert!(
        state
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("stale or unavailable"))
    );
}
