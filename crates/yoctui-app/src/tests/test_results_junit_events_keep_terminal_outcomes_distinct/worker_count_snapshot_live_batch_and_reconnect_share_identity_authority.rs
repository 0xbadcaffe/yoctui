use super::*;

#[test]
fn worker_count_snapshot_live_batch_and_reconnect_share_identity_authority() {
    use yoctui_protocol::daemon::{
        DaemonBuildEvent as B, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut journal = DaemonSnapshotJournal::new(
        daemon_protocol_snapshot(&state),
        DaemonSnapshotLimits::default(),
    )
    .unwrap();
    for event in [
        B::Reset {
            targets: vec!["image".into()],
        },
        B::Started {
            started_unix_ms: None,
        },
    ] {
        journal.publish(DaemonEvent::Build(event)).unwrap();
    }
    let mut live = yoctui_model::App::new(64, 64 * 1024);
    let mut batch = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    let mut batch_replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut live, journal.snapshot().clone());
    batch_replica.replace_app(&mut batch, journal.snapshot().clone());
    let mut events = Vec::new();
    for (recipe, pid) in [("rust-native", 41), ("tar", 42)] {
        let event = journal
            .publish(DaemonEvent::Build(B::TaskStarted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                pid: Some(pid),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: None,
            }))
            .unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
        events.push(event);
    }
    batch_replica
        .apply_task_events_to_app(&mut batch, &events)
        .unwrap();
    let mut attached = yoctui_model::App::new(64, 64 * 1024);
    DaemonClientSnapshot::default().replace_app(&mut attached, journal.snapshot().clone());
    for app in [&live, &batch, &attached] {
        assert_eq!(app.active_worker_count(), Some(2));
    }
    let event = journal
        .publish(DaemonEvent::Build(B::TaskCompleted {
            recipe: "tar".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: None,
        }))
        .unwrap();
    replica.apply_event_to_app(&mut live, &event).unwrap();
    assert_eq!(live.active_worker_count(), Some(1));
    replica.disconnect_app(&mut live);
    assert_eq!(live.active_worker_count(), None);
    replica.replace_app(&mut live, journal.snapshot().clone());
    assert_eq!(live.active_worker_count(), Some(1));
    let event = journal
        .publish(DaemonEvent::Build(B::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: None,
        }))
        .unwrap();
    replica.apply_event_to_app(&mut live, &event).unwrap();
    assert_eq!(live.active_worker_count(), Some(0));
    DaemonClientSnapshot::default().replace_app(&mut attached, journal.snapshot().clone());
    assert_eq!(attached.active_worker_count(), Some(0));
    for event in [
        B::Reset {
            targets: vec!["next".into()],
        },
        B::Started {
            started_unix_ms: None,
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(event)).unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
    }
    assert_eq!(live.active_worker_count(), None);
}
