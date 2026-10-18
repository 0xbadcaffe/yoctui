use super::*;

fn screen(id: u64, text: &str) -> PtyScreenSnapshot {
    PtyScreenSnapshot {
        session_id: PtySessionId(id),
        dimensions: TerminalDimensions {
            columns: 80,
            rows: 24,
        },
        cursor_column: 0,
        cursor_row: 0,
        cursor_hidden: false,
        application_cursor: true,
        scrollback_offset: 0,
        scrollback_lines: 0,
        dropped_line_feeds_lower_bound: 0,
        cells: vec![PtyScreenCell {
            index: 0,
            contents: text.into(),
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
    }
}

#[test]
fn terminal_screen_budget_tracks_replacement_commas_escaping_and_counter_growth_exactly() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot
        .recovery_warnings
        .push("retained build context".repeat(8192));
    snapshot.pty_screens.push(screen(1, "initial"));
    let mut journal =
        DaemonSnapshotJournal::new(snapshot, DaemonSnapshotLimits::default()).unwrap();
    for index in 0..120 {
        let contents = "雪\\\"".repeat(index % 20);
        journal
            .publish(DaemonEvent::PtyScreen(screen(
                1 + index as u64 % 3,
                &contents,
            )))
            .unwrap();
        journal
            .publish(DaemonEvent::PtyOutput {
                session_id: PtySessionId(1),
                bytes: vec![65],
            })
            .unwrap();
        assert_eq!(
            journal.ipc_metrics().snapshot_bytes_upper_bound,
            serde_json::to_vec(journal.snapshot()).unwrap().len()
        );
    }
    journal
        .publish(DaemonEvent::PtyRemoved {
            session_id: PtySessionId(2),
        })
        .unwrap();
    journal
        .publish(DaemonEvent::PtyScreen(screen(2, "reinserted")))
        .unwrap();
    assert!(
        journal.ipc_metrics().snapshot_bytes_upper_bound
            >= serde_json::to_vec(journal.snapshot()).unwrap().len()
    );
    assert_eq!(
        journal.ipc_metrics().snapshot_serializations,
        1,
        "a terminal redraw must not serialize the retained build snapshot"
    );
}

#[test]
fn terminal_screen_budget_validates_transactionally_and_evicts_reconstructible_screens() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot.pty_screens.push(screen(1, "a"));
    let limit = serde_json::to_vec(&snapshot).unwrap().len();
    let mut journal = DaemonSnapshotJournal::new(
        snapshot.clone(),
        DaemonSnapshotLimits {
            snapshot_bytes: limit,
            ..DaemonSnapshotLimits::default()
        },
    )
    .unwrap();
    let initial_cache = journal.pty_screen_bytes.clone();
    let mut invalid = screen(1, "a");
    invalid.cursor_row = 24;
    assert!(matches!(
        journal.publish(DaemonEvent::PtyScreen(invalid)),
        Err(DaemonSnapshotError::InvalidPtyScreen(_))
    ));
    assert_eq!(journal.snapshot(), &snapshot);
    assert_eq!(journal.pty_screen_bytes, initial_cache);
    journal
        .publish(DaemonEvent::PtyScreen(screen(
            1,
            "too large for this limit",
        )))
        .unwrap();
    assert!(journal.snapshot().pty_screens.is_empty());
    assert!(journal.pty_screen_bytes.is_empty());
    journal
        .publish(DaemonEvent::PtyScreen(screen(1, "a")))
        .unwrap();
    assert_eq!(
        journal.ipc_metrics().snapshot_bytes_upper_bound,
        serde_json::to_vec(journal.snapshot()).unwrap().len()
    );
}

#[test]
fn terminal_screen_budget_rebuilds_leaf_cache_after_bounded_screen_eviction() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot.pty_screens = vec![screen(1, &"a".repeat(500)), screen(2, &"b".repeat(500))];
    let limit = serde_json::to_vec(&snapshot).unwrap().len();
    let mut journal = DaemonSnapshotJournal::new(
        snapshot,
        DaemonSnapshotLimits {
            snapshot_bytes: limit,
            ..DaemonSnapshotLimits::default()
        },
    )
    .unwrap();
    journal
        .publish(DaemonEvent::PtyScreen(screen(3, &"c".repeat(500))))
        .unwrap();
    assert_eq!(journal.snapshot().pty_screens.len(), 2);
    assert!(!journal.pty_screen_bytes.contains_key(&PtySessionId(1)));
    journal
        .publish(DaemonEvent::PtyScreen(screen(2, "short")))
        .unwrap();
    assert_eq!(
        journal.ipc_metrics().snapshot_bytes_upper_bound,
        serde_json::to_vec(journal.snapshot()).unwrap().len()
    );
}
