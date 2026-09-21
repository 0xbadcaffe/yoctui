use super::*;

#[test]
fn snapshot_progress_keeps_counts_after_eviction_and_duplicate_completion() {
    let mut snapshot = daemon_snapshot_fixture();
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::Reset {
            targets: vec!["image".into()],
        },
    );
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::TaskQueued {
            recipe: "seed".into(),
            task: "do_compile".into(),
            worker: None,
            stats: Some(TaskStatsData {
                completed: 2_339,
                total: 6_812,
                active: 1,
                failed: 0,
            }),
        },
    );
    let count = MAX_DAEMON_BUILD_EVENTS + 8;
    for index in 0..count {
        let event = DaemonBuildEvent::TaskCompleted {
            recipe: format!("recipe-{index}"),
            task: "do_compile".into(),
            success: index % 2 == 0,
            started_unix_ms: None,
            finished_unix_ms: None,
        };
        apply_build_event(&mut snapshot, event.clone());
        apply_build_event(&mut snapshot, event);
    }
    assert_eq!(snapshot.build_events.len(), MAX_DAEMON_BUILD_EVENTS);
    assert_eq!(
        snapshot.build_progress,
        Some(DaemonBuildProgress {
            completed: 2_339 + count,
            total: Some(6_812),
            ..Default::default()
        })
    );
    assert!(!snapshot.build_events.iter().any(|event| matches!(event,
            DaemonBuildEvent::TaskCompleted { recipe, .. } if recipe == "recipe-0")));
    let encoded = encode_frame(&ServerMessage::Snapshot(snapshot.clone())).unwrap();
    assert!(encoded.len() < MAX_FRAME_BYTES);
    assert_eq!(
        decode_frame::<ServerMessage>(&encoded).unwrap(),
        ServerMessage::Snapshot(snapshot.clone())
    );
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::Completed {
            success: false,
            exit_code: Some(1),
            finished_unix_ms: None,
        },
    );
    assert_eq!(snapshot.build_progress.unwrap().completed, 2_339 + count);
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: None,
        },
    );
    assert_eq!(snapshot.build_progress.unwrap().completed, 6_812);
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::Reset {
            targets: vec!["next".into()],
        },
    );
    assert_eq!(
        snapshot.build_progress,
        Some(DaemonBuildProgress::default())
    );
}
