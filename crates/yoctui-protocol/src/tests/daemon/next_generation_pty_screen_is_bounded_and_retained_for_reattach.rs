use super::*;

#[test]
fn next_generation_pty_screen_is_bounded_and_retained_for_reattach() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    let screen = PtyScreenSnapshot {
        session_id: PtySessionId(9),
        dimensions: TerminalDimensions {
            columns: 20,
            rows: 4,
        },
        cursor_column: 3,
        cursor_row: 1,
        cursor_hidden: false,
        scrollback_offset: 0,
        cells: Vec::new(),
        scrollback_lines: 7,
        dropped_line_feeds_lower_bound: 0,
    };
    journal
        .publish(DaemonEvent::PtyScreen(screen.clone()))
        .unwrap();
    assert_eq!(journal.snapshot().pty_screens, vec![screen.clone()]);
    let mut replacement = screen;
    replacement.cells = vec![PtyScreenCell {
        index: 0,
        contents: "updated".into(),
        foreground: PtyTerminalColor::Default,
        background: PtyTerminalColor::Default,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        inverse: false,
        wide: false,
        wide_continuation: false,
    }];
    journal
        .publish(DaemonEvent::PtyScreen(replacement.clone()))
        .unwrap();
    assert_eq!(journal.snapshot().pty_screens, vec![replacement]);

    let invalid = PtyScreenSnapshot {
        session_id: PtySessionId(10),
        dimensions: TerminalDimensions {
            columns: 2,
            rows: 1,
        },
        cursor_column: 0,
        cursor_row: 0,
        cursor_hidden: false,
        scrollback_offset: 0,
        cells: vec![PtyScreenCell {
            index: 2,
            contents: "outside".into(),
            foreground: PtyTerminalColor::Default,
            background: PtyTerminalColor::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            inverse: false,
            wide: false,
            wide_continuation: false,
        }],
        scrollback_lines: 0,
        dropped_line_feeds_lower_bound: 0,
    };
    assert!(matches!(
        journal.publish(DaemonEvent::PtyScreen(invalid)),
        Err(DaemonSnapshotError::InvalidPtyScreen(PtySessionId(10)))
    ));
}
