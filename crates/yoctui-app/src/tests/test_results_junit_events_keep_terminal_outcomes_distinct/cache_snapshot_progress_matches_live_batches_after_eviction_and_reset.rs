use super::*;

#[test]
fn cache_snapshot_progress_matches_live_batches_after_eviction_and_reset() {
    use yoctui_protocol::daemon::{
        DaemonBuildEvent, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
        MAX_DAEMON_BUILD_EVENTS,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut journal =
        DaemonSnapshotJournal::new(initial.clone(), DaemonSnapshotLimits::default()).unwrap();
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut app, initial);
    for build in [
        DaemonBuildEvent::Reset {
            targets: vec!["image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::SstateSummary {
            summary: yoctui_model::SstateSummary {
                wanted: 10,
                local: 3,
                mirrors: 2,
                missed: 5,
                current: 8,
            },
        },
        DaemonBuildEvent::TaskQueued {
            recipe: "seed".into(),
            task: "do_compile".into(),
            worker: None,
            stats: Some(yoctui_protocol::TaskStatsData {
                completed: 2_339,
                total: 6_812,
                active: 1,
                failed: 0,
            }),
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(build)).unwrap();
        replica.apply_event_to_app(&mut app, &event).unwrap();
    }
    let count = MAX_DAEMON_BUILD_EVENTS + 8;
    for index in 0..count {
        let event = journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
                recipe: format!("recipe-{index}"),
                task: "do_fetch".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: None,
            }))
            .unwrap();
        replica
            .apply_task_events_to_app(&mut app, &[event])
            .unwrap();
    }
    let expected = (2_339 + count, Some(6_812));
    assert_eq!(app.build.cache.fetch_completed, count);
    let cache = app.build.cache;
    assert_eq!((app.build.completed, app.build.total), expected);
    replica.replace_app(&mut app, journal.snapshot().clone());
    assert_eq!(app.build.cache, cache);
    assert_eq!(app.build.cache.summary.unwrap().match_percent(), Some(50));
    assert_eq!((app.build.completed, app.build.total), expected);
    let event = journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Reset {
            targets: vec!["next".into()],
        }))
        .unwrap();
    replica.apply_event_to_app(&mut app, &event).unwrap();
    assert_eq!((app.build.completed, app.build.total), (0, None));
    assert!(app.completed_tasks.is_empty());
    assert_eq!(app.build.cache, yoctui_model::BuildCacheState::default());
}
