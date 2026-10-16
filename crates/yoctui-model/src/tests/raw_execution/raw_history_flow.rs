use super::*;

#[test]
fn raw_history_opens_retained_execution_and_escape_returns_to_history() {
    let (catalog, _, _) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let execution = terminal_for_history("retained", RawExecutionOutcome::Succeeded, 200);
    let id = execution.request.id.clone();
    let record = RawHistoryRecord::from_terminal(&execution).unwrap();
    let mut state = RawModeState::new(&catalog);
    state.history.push(record);
    state.execution_states.insert(id.clone(), execution);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::OpenHistory);
    assert_eq!(state.history_request_ids(), vec![id.clone()]);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::ActivateHistory);
    assert_eq!(state.view, RawModeView::Execution);
    assert_eq!(state.output.request, Some(id));
    assert!(state.selected_execution().unwrap().phase.is_terminal());
    assert!(state.form.is_none());
    assert!(state.preview.is_none());
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::CloseExecution);
    assert_eq!(state.view, RawModeView::History);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::Back);
    assert_eq!(state.view, RawModeView::Browser);
}

#[test]
fn raw_history_metadata_never_invents_execution_or_replays_a_command() {
    let (catalog, _, _) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let execution = terminal_for_history("metadata", RawExecutionOutcome::Succeeded, 200);
    let mut state = RawModeState::new(&catalog);
    state
        .history
        .push(RawHistoryRecord::from_terminal(&execution).unwrap());
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::OpenHistory);
    reduce_raw_mode(&mut state, &catalog, None, RawModeAction::ActivateHistory);
    assert_eq!(state.view, RawModeView::History);
    assert!(state.execution_states.is_empty());
    assert!(state.output.request.is_none());
    assert!(
        state
            .notification
            .as_deref()
            .unwrap()
            .contains("Only history metadata")
    );
}

#[test]
fn raw_history_explicit_new_review_preserves_typed_parameters_and_arguments() {
    let (catalog, request, _) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let execution = queued(RawInteractionMode::NoninteractiveJob);
    let expected = execution.request.clone();
    let authority = super::super::raw_preview_tests::authority(request.capability_generation, true);
    let mut state = RawModeState::new(&catalog);
    state
        .execution_states
        .insert(expected.id.clone(), execution);
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::OpenHistory,
    );
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::ReviewHistory,
    );
    assert_eq!(state.view, RawModeView::Form);
    let form = state.form.as_ref().unwrap();
    for (id, value) in &expected.parameters {
        assert_eq!(form.fields[id].value.as_ref(), Some(value));
    }
    assert_eq!(
        RawAdditionalArguments::parse(&form.additional_arguments.editor.text)
            .unwrap()
            .as_slice(),
        expected.additional_arguments
    );
    assert!(state.preview.is_none());
    reduce_raw_mode(&mut state, &catalog, Some(&authority), RawModeAction::Back);
    assert_eq!(state.view, RawModeView::History);
}

#[test]
fn raw_output_end_restores_follow_and_preview_scroll_is_bounded() {
    let (catalog, _, _) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let mut state = RawModeState::new(&catalog);
    state.output.follow = false;
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::ScrollOutput {
            vertical: isize::MIN,
            horizontal: 0,
        },
    );
    assert!(state.output.follow);
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::ScrollPreview { delta: isize::MAX },
    );
    assert!(state.preview_scroll < usize::from(u16::MAX));
    reduce_raw_mode(
        &mut state,
        &catalog,
        None,
        RawModeAction::ScrollPreview { delta: isize::MIN },
    );
    assert_eq!(state.preview_scroll, 0);
}

#[test]
fn raw_history_selection_stays_on_request_when_newer_jobs_arrive() {
    let (catalog, _, _) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let first = terminal_for_history("first", RawExecutionOutcome::Succeeded, 200);
    let id = first.request.id.clone();
    let mut state = RawModeState::new(&catalog);
    state.execution_states.insert(id.clone(), first);
    let selected = state.selected_history_request();
    let mut newer = queued(RawInteractionMode::NoninteractiveJob);
    newer.request.id = RawRequestId::new("raw-request:newer").unwrap();
    newer.queued_unix_ms = 300;
    state
        .execution_states
        .insert(newer.request.id.clone(), newer);
    state.restore_history_selection(selected.as_ref());
    assert_eq!(state.history_selection, 1);
    assert_eq!(state.selected_history_request(), Some(id));
}
