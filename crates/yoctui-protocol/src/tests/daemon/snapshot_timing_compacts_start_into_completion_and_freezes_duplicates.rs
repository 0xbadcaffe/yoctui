use super::*;

#[test]
fn snapshot_timing_compacts_start_into_completion_and_freezes_duplicates() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    let publish = |journal: &mut DaemonSnapshotJournal, event| {
        journal.publish(DaemonEvent::Build(event)).unwrap()
    };
    publish(
        &mut journal,
        DaemonBuildEvent::Started {
            started_unix_ms: Some(1000),
        },
    );
    publish(
        &mut journal,
        DaemonBuildEvent::Started {
            started_unix_ms: Some(9000),
        },
    );
    let started = DaemonBuildEvent::TaskStarted {
        recipe: "recipe".into(),
        task: "do_compile".into(),
        started_unix_ms: Some(2000),
        pid: None,
        worker: None,
        log_path: None,
        stats: None,
    };
    publish(&mut journal, started);
    let completed = |finished| DaemonBuildEvent::TaskCompleted {
        recipe: "recipe".into(),
        task: "do_compile".into(),
        success: true,
        started_unix_ms: None,
        finished_unix_ms: Some(finished),
    };
    let first = publish(&mut journal, completed(2500));
    let repeated = publish(&mut journal, completed(9900));
    assert_eq!(first.event, repeated.event);
    assert!(matches!(
        first.event,
        DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
            started_unix_ms: Some(2000),
            finished_unix_ms: Some(2500),
            ..
        })
    ));
    assert!(
        !journal
            .snapshot()
            .build_events
            .iter()
            .any(|event| matches!(event, DaemonBuildEvent::TaskStarted { .. }))
    );
    let terminal = |finished| DaemonBuildEvent::Completed {
        success: true,
        exit_code: Some(0),
        finished_unix_ms: Some(finished),
    };
    let first = publish(&mut journal, terminal(5000));
    let repeated = publish(&mut journal, terminal(10000));
    assert_eq!(first.event, repeated.event);
    assert!(
        journal
            .snapshot()
            .build_events
            .iter()
            .filter(|event| matches!(event, DaemonBuildEvent::Started { .. }))
            .all(|event| matches!(
                event,
                DaemonBuildEvent::Started {
                    started_unix_ms: Some(1000)
                }
            ))
    );
    publish(
        &mut journal,
        DaemonBuildEvent::Reset {
            targets: vec!["next".into()],
        },
    );
    let next = publish(
        &mut journal,
        DaemonBuildEvent::Started {
            started_unix_ms: Some(20000),
        },
    );
    assert!(matches!(
        next.event,
        DaemonEvent::Build(DaemonBuildEvent::Started {
            started_unix_ms: Some(20000)
        })
    ));
}
