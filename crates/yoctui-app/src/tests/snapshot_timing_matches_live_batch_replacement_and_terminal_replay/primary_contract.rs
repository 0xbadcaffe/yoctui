use super::*;

#[test]
fn snapshot_timing_matches_live_batch_replacement_and_terminal_replay() {
    use std::time::{Duration, UNIX_EPOCH};
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
    let mut live = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut live, journal.snapshot().clone());
    for event in [
        B::Reset {
            targets: vec!["image".into()],
        },
        B::Started {
            started_unix_ms: Some(1000),
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(event)).unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
    }
    let mut batch = yoctui_model::App::new(64, 64 * 1024);
    let mut batch_replica = DaemonClientSnapshot::default();
    batch_replica.replace_app(&mut batch, journal.snapshot().clone());
    let mut events = Vec::new();
    for recipe in ["llvm-native", "cli11"] {
        for event in [
            B::TaskStarted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: Some(2000),
            },
            B::TaskCompleted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: Some(5000),
            },
            B::TaskCompleted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: Some(9000),
            },
        ] {
            let event = journal.publish(DaemonEvent::Build(event)).unwrap();
            replica.apply_event_to_app(&mut live, &event).unwrap();
            events.push(event);
        }
    }
    batch_replica
        .apply_task_events_to_app(&mut batch, &events)
        .unwrap();
    let mut fresh = yoctui_model::App::new(64, 64 * 1024);
    fresh.screen = Screen::Recipes;
    fresh.focus = FocusTarget::Inspector;
    DaemonClientSnapshot::default().replace_app(&mut fresh, journal.snapshot().clone());
    let now = UNIX_EPOCH + Duration::from_secs(100);
    for app in [&live, &batch, &fresh] {
        assert_eq!(
            app.build_summary_at(now).elapsed,
            Some(Duration::from_secs(99))
        );
        assert_eq!(app.completed_tasks.len(), 2);
        for row in &app.completed_tasks {
            assert_eq!(row.task.elapsed_at(now), Some(Duration::from_secs(3)));
        }
    }
    assert_eq!(fresh.screen, Screen::Recipes);
    assert_eq!(fresh.focus, FocusTarget::Inspector);
    let previous_record = yoctui_model::BuildRecord {
        target: Some("previous-image".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(12)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    };
    fresh.build_history.push_back(previous_record.clone());
    for finished in [6000, 99000] {
        let event = journal
            .publish(DaemonEvent::Build(B::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: Some(finished),
            }))
            .unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
    }
    DaemonClientSnapshot::default().replace_app(&mut fresh, journal.snapshot().clone());
    DaemonClientSnapshot::default().replace_app(&mut fresh, journal.snapshot().clone());
    assert_eq!(fresh.build_history.len(), 2);
    assert_eq!(fresh.build_history[0], previous_record);
    replica.replace_app(&mut live, journal.snapshot().clone());
    for app in [&live, &fresh] {
        assert_eq!(
            app.build_summary_at(now).elapsed,
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            app.build_summary_at(now + Duration::from_secs(900)).elapsed,
            Some(Duration::from_secs(5))
        );
    }
    let event = journal
        .publish(DaemonEvent::Build(B::Reset {
            targets: vec!["next".into()],
        }))
        .unwrap();
    replica.apply_event_to_app(&mut live, &event).unwrap();
    assert_eq!(live.build.started, None);
    assert!(live.completed_tasks.is_empty());
}
