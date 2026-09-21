use super::*;

#[test]
fn ux_terminal_workbench_bounds_search_rename_paste_and_scrollback() {
    let mut state = TerminalWorkbenchState::default();
    for _ in 0..MAX_TERMINAL_SEARCH_BYTES {
        assert!(state.append_query('x'));
    }
    assert!(!state.append_query('x'));
    assert!(!state.append_rename('\n'));
    assert!(!state.stage_paste(&"x".repeat(MAX_TERMINAL_PASTE_BYTES + 1)));
    assert!(state.stage_paste("printf 'safe review'\n"));
    assert_eq!(state.mode, TerminalWorkbenchMode::PasteReview);
    state.set_scrollback_offset(500, 42);
    assert_eq!(state.scrollback_offset, 42);
    state.reset_transient_mode();
    assert!(state.pending_paste.is_empty());
}
