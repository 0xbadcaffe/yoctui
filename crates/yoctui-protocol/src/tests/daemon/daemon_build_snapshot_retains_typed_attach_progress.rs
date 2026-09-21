use super::*;

#[test]
fn daemon_build_snapshot_retains_typed_attach_progress() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Reset {
            targets: vec!["core-image-minimal".into()],
        }))
        .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: Some("worker-1".into()),
            log_path: Some("/build/temp/log.do_compile".into()),
            stats: Some(TaskStatsData {
                completed: 102,
                total: 4090,
                active: 8,
                failed: 0,
            }),
            started_unix_ms: None,
        }))
        .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(77),
        }))
        .unwrap();
    assert_eq!(journal.snapshot().build_events.len(), 3);
    assert!(matches!(
        journal.snapshot().build_events.last(),
        Some(DaemonBuildEvent::TaskProgress {
            progress: Some(77),
            ..
        })
    ));
    let encoded = encode_frame(&ServerMessage::Snapshot(journal.snapshot().clone())).unwrap();
    assert!(encoded.len() < MAX_FRAME_BYTES);
}
