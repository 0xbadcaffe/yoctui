use crate::tests::ux_terminal_fixture;
use crate::{Action, ClientDaemonLifecycle, Effect, TerminalEffect, TerminalWorkbenchMode, update};

#[test]
fn terminal_search_clear_and_history_preserve_mode_query_and_writer_lease() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    app.daemon.pty_screens[0].scrollback_lines = 50;
    update(&mut app, Action::TerminalBeginSearch);
    update(&mut app, Action::TerminalAppendSearch('界'));
    let session_id = app.selected_terminal_session().unwrap().id;
    for (delta, offset) in [(10, 10), (isize::MAX, 50), (-1, 49), (isize::MIN + 1, 0)] {
        assert_eq!(
            update(&mut app, Action::TerminalScroll { delta }),
            Some(Effect::Terminal(TerminalEffect::Viewport {
                session_id,
                scrollback_offset: offset
            }))
        );
        assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Search);
        assert_eq!(app.terminal.query, "界");
        assert!(app.selected_terminal_is_writer());
    }
    assert_eq!(update(&mut app, Action::TerminalClearSearch), None);
    assert!(app.terminal.query.is_empty());
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Search);
    update(&mut app, Action::TerminalAppendSearch('/'));
    assert_eq!(app.terminal.query, "/");
    update(&mut app, Action::TerminalFinishSearch);
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
    assert!(app.selected_terminal_is_writer());
}
