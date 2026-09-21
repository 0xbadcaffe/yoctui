use super::*;

#[test]
fn recipe_inventory_oversized_workspace_reports_error_and_keeps_journal_usable() {
    use yoctui_model::{DaemonGlobalState, DaemonModelInstanceId, DaemonStateLimits};
    use yoctui_protocol::daemon::{DaemonSnapshotJournal, DaemonSnapshotLimits, MAX_FRAME_BYTES};
    let state = DaemonGlobalState::new(
        DaemonModelInstanceId([1; 16]),
        1,
        "fixture-boot".into(),
        DaemonStateLimits::default(),
    )
    .unwrap();
    let mut journal = DaemonSnapshotJournal::new(
        crate::daemon_protocol_snapshot(&state),
        DaemonSnapshotLimits::default(),
    )
    .unwrap();
    let mut workspace = Workspace::default();
    workspace
        .variables
        .insert("oversized".into(), "x".repeat(MAX_FRAME_BYTES));
    assert!(!publish_workspace(&mut journal, workspace).unwrap());
    assert!(journal.snapshot().build_events.is_empty());
    assert!(
        journal
            .snapshot()
            .recent_logs
            .last()
            .unwrap()
            .message
            .contains("snapshot bounds")
    );
    assert!(publish_workspace(&mut journal, Workspace::default()).unwrap());
    assert_eq!(journal.snapshot().build_events.len(), 1);
}
