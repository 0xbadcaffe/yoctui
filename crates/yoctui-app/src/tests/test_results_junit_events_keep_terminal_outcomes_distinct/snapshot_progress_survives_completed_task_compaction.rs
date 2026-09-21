use super::*;

#[test]
fn snapshot_progress_survives_completed_task_compaction() {
    use yoctui_protocol::daemon::{
        DaemonBuildEvent, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let snapshot = daemon_protocol_snapshot(&state);
    let mut journal =
        DaemonSnapshotJournal::new(snapshot.clone(), DaemonSnapshotLimits::default()).unwrap();
    let mut uninterrupted = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut uninterrupted, snapshot);
    for event in [
        DaemonBuildEvent::Reset {
            targets: vec!["obmc-phosphor-image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskQueued {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            worker: None,
            stats: Some(yoctui_protocol::TaskStatsData {
                completed: 2_339,
                total: 6_812,
                active: 2,
                failed: 0,
            }),
        },
        DaemonBuildEvent::TaskStarted {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskCompleted {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: None,
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(event)).unwrap();
        replica
            .apply_event_to_app(&mut uninterrupted, &event)
            .unwrap();
    }
    assert_eq!(
        (uninterrupted.build.completed, uninterrupted.build.total),
        (2_340, Some(6_812))
    );
    let mut attached = yoctui_model::App::new(64, 64 * 1024);
    attached.screen = Screen::Recipes;
    DaemonClientSnapshot::default().replace_app(&mut attached, journal.snapshot().clone());
    assert_eq!(
        (attached.build.completed, attached.build.total),
        (2_340, Some(6_812))
    );
    assert_eq!(attached.screen, Screen::Recipes);
    assert_eq!(attached.completed_tasks.len(), 1);
    replica.replace_app(&mut uninterrupted, journal.snapshot().clone());
    assert_eq!(
        (uninterrupted.build.completed, uninterrupted.build.total),
        (2_340, Some(6_812))
    );
}
