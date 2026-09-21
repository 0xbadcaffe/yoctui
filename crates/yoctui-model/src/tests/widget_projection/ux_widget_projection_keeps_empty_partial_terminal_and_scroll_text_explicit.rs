use super::*;

#[test]
fn ux_widget_projection_keeps_empty_partial_terminal_and_scroll_text_explicit() {
    let empty = HistoryProjection::bounded(
        "I/O",
        WidgetState::Available,
        WidgetRole::DiskRead,
        None,
        [],
        60,
        "",
    );
    assert_eq!(empty.state, WidgetState::Empty);
    assert!(empty.text(false, true).contains("empty"));

    let terminal = GaugeProjection::terminal(
        "Build",
        8,
        10,
        WidgetTerminalState::Failure,
        "two tasks failed",
    );
    assert!(terminal.text(false, true).contains("x Build · failed"));

    let terminal_unknown = GaugeProjection::terminal(
        "Parse",
        12,
        0,
        WidgetTerminalState::Cancelled,
        "backend stopped",
    );
    assert_eq!(terminal_unknown.state, WidgetState::TerminalCancelled);
    assert!(terminal_unknown.text(false, true).contains("12/?"));

    let scroll = ScrollbarProjection::new(999, 999, 4, 12);
    assert_eq!(scroll.label(), "9-12/12");
    assert_eq!(
        ScrollbarProjection::new(0, 0, 5, 0).state,
        WidgetState::Empty
    );
}
