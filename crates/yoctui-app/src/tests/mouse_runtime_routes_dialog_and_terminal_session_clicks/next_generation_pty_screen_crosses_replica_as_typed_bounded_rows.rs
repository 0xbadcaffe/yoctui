use super::*;

#[test]
fn next_generation_pty_screen_crosses_replica_as_typed_bounded_rows() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "pty-screen".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut client = DaemonClientSnapshot::default();
    client.replace(daemon_protocol_snapshot(&state));
    let mut app = yoctui_model::App::new(16, 4096);
    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: 1,
                generation: 1,
                event: yoctui_protocol::daemon::DaemonEvent::PtyScreen(
                    yoctui_protocol::daemon::PtyScreenSnapshot {
                        session_id: yoctui_protocol::daemon::PtySessionId(4),
                        dimensions: yoctui_protocol::daemon::TerminalDimensions {
                            columns: 20,
                            rows: 3,
                        },
                        cursor_column: 2,
                        cursor_row: 1,
                        cursor_hidden: false,
                        scrollback_offset: 0,
                        cells: vec![yoctui_protocol::daemon::PtyScreenCell {
                            index: 0,
                            contents: "r".into(),
                            foreground: yoctui_protocol::daemon::PtyTerminalColor::Rgb(1, 2, 3),
                            background: yoctui_protocol::daemon::PtyTerminalColor::Default,
                            bold: true,
                            dim: false,
                            italic: false,
                            underline: false,
                            inverse: false,
                            wide: false,
                            wide_continuation: false,
                        }],
                        scrollback_lines: 8,
                        dropped_line_feeds_lower_bound: 2,
                    },
                ),
            },
        )
        .unwrap();
    let screen = &app.daemon.pty_screens[0];
    assert_eq!(screen.session_id, 4);
    assert_eq!(screen.rows[0], "r");
    assert_eq!(screen.scrollback_lines, 8);
    assert_eq!(screen.cells.len(), 60);
    assert_eq!(screen.cells[0].contents, "r");
    assert!(screen.cells[0].bold);
    assert_eq!(
        screen.cells[0].foreground,
        yoctui_model::ClientDaemonTerminalColor::Rgb(1, 2, 3)
    );
}
