use super::*;
use yoctui_protocol::daemon::*;

fn fixture() -> (DaemonClientSnapshot, App) {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([77; 16]),
        1,
        "terminal".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    for id in 1..=2 {
        snapshot.pty_sessions.push(PtySessionSummary {
            id: PtySessionId(id),
            name: format!("shell {id}"),
            kind: PtyKind::BuildShell,
            cwd: "/tmp".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 80,
                rows: 24,
            },
            writer: None,
            writer_epoch: 0,
            viewers: 0,
            exit_code: None,
            restartable: true,
        });
        snapshot.pty_screens.push(screen(id, "before"));
    }
    let mut client = DaemonClientSnapshot::default();
    let mut app = App::new(16, 4096);
    client.replace_app(&mut app, snapshot);
    (client, app)
}

fn screen(id: u64, text: &str) -> PtyScreenSnapshot {
    PtyScreenSnapshot {
        session_id: PtySessionId(id),
        dimensions: TerminalDimensions {
            columns: 80,
            rows: 24,
        },
        cursor_column: 4,
        cursor_row: 0,
        cursor_hidden: false,
        application_cursor: true,
        scrollback_offset: 0,
        cells: text
            .chars()
            .enumerate()
            .map(|(index, ch)| PtyScreenCell {
                index: index as u32,
                contents: ch.to_string(),
                foreground: PtyTerminalColor::Rgb(10, 20, 30),
                background: PtyTerminalColor::Default,
                bold: true,
                dim: false,
                italic: false,
                underline: false,
                inverse: false,
                wide: false,
                wide_continuation: false,
            })
            .collect(),
        scrollback_lines: 7,
        dropped_line_feeds_lower_bound: 3,
    }
}

fn apply(client: &mut DaemonClientSnapshot, app: &mut App, event: DaemonEvent) {
    let snapshot = client.snapshot.as_ref().unwrap();
    let event = SequencedEvent {
        sequence: snapshot.sequence + 1,
        generation: snapshot.generation + 1,
        event,
    };
    client.apply_event_to_app(app, &event).unwrap();
    assert_eq!(
        app.daemon,
        crate::daemon_view::daemon_client_view(
            client.status,
            client.snapshot.as_ref(),
            client.telemetry
        )
    );
}

#[test]
fn terminal_events_match_full_projection_without_rebuilding_other_terminal_cells() {
    let (mut client, mut app) = fixture();
    let cells = app.daemon.pty_screens[0].cells.as_ptr();
    let other = app.daemon.pty_screens[1].cells.as_ptr();
    apply(
        &mut client,
        &mut app,
        DaemonEvent::PtyOutput {
            session_id: PtySessionId(2),
            bytes: b"\x1b[31mbytes".to_vec(),
        },
    );
    assert_eq!(app.daemon.pty_screens[0].cells.as_ptr(), cells);
    assert_eq!(app.daemon.pty_screens[1].cells.as_ptr(), other);
    apply(
        &mut client,
        &mut app,
        DaemonEvent::PtyScreen(screen(2, "echo")),
    );
    assert_eq!(app.daemon.pty_screens[0].cells.as_ptr(), cells);
    let mut summary = client.snapshot.as_ref().unwrap().pty_sessions[1].clone();
    summary.writer = Some(ClientId([9; 16]));
    summary.writer_epoch = 4;
    summary.kind = PtyKind::QemuConsole;
    apply(&mut client, &mut app, DaemonEvent::PtyChanged(summary));
    assert_eq!(app.daemon.pty_screens[0].cells.as_ptr(), cells);
    apply(
        &mut client,
        &mut app,
        DaemonEvent::PtyRemoved {
            session_id: PtySessionId(2),
        },
    );
    assert_eq!(app.daemon.pty_screens[0].cells.as_ptr(), cells);
}

#[test]
fn terminal_fast_projection_falls_back_after_a_skipped_view_update_and_rejects_gaps() {
    let (mut client, mut app) = fixture();
    client
        .apply_event(&SequencedEvent {
            sequence: 1,
            generation: 1,
            event: DaemonEvent::PtyRemoved {
                session_id: PtySessionId(1),
            },
        })
        .unwrap();
    apply(
        &mut client,
        &mut app,
        DaemonEvent::PtyOutput {
            session_id: PtySessionId(2),
            bytes: vec![65],
        },
    );
    assert_eq!(app.daemon.pty_sessions.len(), 1);
    assert!(
        client
            .apply_event_to_app(
                &mut app,
                &SequencedEvent {
                    sequence: 4,
                    generation: 4,
                    event: DaemonEvent::PtyScreen(screen(2, "invalid gap")),
                }
            )
            .is_err()
    );
    assert_eq!(app.daemon.sequence, 2);
}
