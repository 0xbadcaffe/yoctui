use super::*;

#[test]
fn runqueue_completion_duplicates_preserve_single_progress_for_fresh_attach() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Reset {
            targets: vec!["obmc-phosphor-image".into()],
        }))
        .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::TaskQueued {
            recipe: "obmc-phosphor-image".into(),
            task: "do_build".into(),
            worker: None,
            stats: Some(crate::TaskStatsData {
                completed: 9,
                total: 10,
                active: 0,
                failed: 0,
            }),
        }))
        .unwrap();
    let completed = |finished| {
        DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
            recipe: "obmc-phosphor-image".into(),
            task: "do_build".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: Some(finished),
        })
    };
    let first = journal.publish(completed(1000)).unwrap();
    let duplicate = journal.publish(completed(2000)).unwrap();
    assert_eq!(first.event, duplicate.event);
    assert_eq!(
        journal
            .snapshot()
            .build_progress
            .as_ref()
            .unwrap()
            .completed,
        10
    );
    // The bounded journal retains duplicate observations, not duplicate UI
    // tasks. Both retain the first end time and neither leaves a queued row.
    assert!(
        !journal.snapshot().build_events.iter().any(|event| matches!(
            event,
            DaemonBuildEvent::TaskQueued { .. } | DaemonBuildEvent::TaskStarted { .. }
        ))
    );
    let terminal = journal
        .snapshot()
        .build_events
        .iter()
        .filter(|event| matches!(event, DaemonBuildEvent::TaskCompleted { .. }))
        .collect::<Vec<_>>();
    assert_eq!(terminal.len(), 2);
    assert!(terminal.iter().all(|event| matches!(
        event,
        DaemonBuildEvent::TaskCompleted {
            success: true,
            started_unix_ms: None,
            finished_unix_ms: Some(1000),
            ..
        }
    )));
    let frame = encode_frame(&ServerMessage::Snapshot(journal.snapshot().clone())).unwrap();
    assert!(frame.len() < MAX_FRAME_BYTES);
}
