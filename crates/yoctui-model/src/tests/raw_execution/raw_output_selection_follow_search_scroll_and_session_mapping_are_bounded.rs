use super::*;

#[test]
fn raw_output_selection_follow_search_scroll_and_session_mapping_are_bounded() {
    let (catalog, _, _) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let execution = queued(RawInteractionMode::NoninteractiveJob);
    let request = execution.request.id.clone();
    let command = execution.request.command.clone();
    let mut mode = RawModeState::new(&catalog);
    mode.execution_states.insert(request.clone(), execution);
    reduce_raw_mode(
        &mut mode,
        &catalog,
        None,
        RawModeAction::OpenExecution(command),
    );
    assert_eq!(mode.selected_execution().unwrap().request.id, request);
    reduce_raw_mode(&mut mode, &catalog, None, RawModeAction::ToggleOutputFollow);
    reduce_raw_mode(
        &mut mode,
        &catalog,
        None,
        RawModeAction::ScrollOutput {
            vertical: isize::MAX,
            horizontal: isize::MAX,
        },
    );
    assert!(!mode.output.follow);
    assert_eq!(mode.output.vertical_scroll, 0);
    assert_eq!(mode.output.horizontal_scroll, 0);
    reduce_raw_mode(&mut mode, &catalog, None, RawModeAction::BeginOutputSearch);
    for character in "héllo".chars() {
        reduce_raw_mode(
            &mut mode,
            &catalog,
            None,
            RawModeAction::AppendOutputSearch(character),
        );
    }
    assert_eq!(mode.output.query, "héllo");
    reduce_raw_mode(
        &mut mode,
        &catalog,
        None,
        RawModeAction::SelectOutputStream(RawOutputStream::Stderr),
    );
    assert_eq!(mode.output.stream, RawOutputStream::Stderr);
    assert_eq!(mode.output.vertical_scroll, 0);
    assert_eq!(
        RawSessionId::new("raw-session:daemon-17")
            .unwrap()
            .daemon_pty_id(),
        Some(RAW_DAEMON_PTY_NAMESPACE | 17)
    );
    assert_eq!(
        RawSessionId::new("raw-session:generic")
            .unwrap()
            .daemon_pty_id(),
        None
    );
}
