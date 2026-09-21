use super::*;

#[test]
fn snapshot_timing_retained_rows_stay_bounded_after_eviction() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Started {
            started_unix_ms: Some(0),
        }))
        .unwrap();
    let count = MAX_DAEMON_BUILD_EVENTS + 4;
    for index in 0..count {
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
                recipe: format!("recipe-{index}"),
                task: "do_compile".into(),
                started_unix_ms: Some(index as u64),
                pid: None,
                worker: None,
                log_path: None,
                stats: None,
            }))
            .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
                recipe: format!("recipe-{index}"),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: Some(index as u64 + 100),
            }))
            .unwrap();
    }
    assert_eq!(
        journal.snapshot().build_events.len(),
        MAX_DAEMON_BUILD_EVENTS
    );
    assert!(
        matches!(journal.snapshot().build_events.last(), Some(DaemonBuildEvent::TaskCompleted {
            started_unix_ms: Some(start), finished_unix_ms: Some(end), ..
        }) if *start == (count - 1) as u64 && *end == (count - 1) as u64 + 100)
    );
    let message = ServerMessage::Snapshot(journal.snapshot().clone());
    let frame = encode_frame(&message).unwrap();
    assert!(frame.len() < MAX_FRAME_BYTES);
    assert_eq!(decode_frame::<ServerMessage>(&frame).unwrap(), message);
}
