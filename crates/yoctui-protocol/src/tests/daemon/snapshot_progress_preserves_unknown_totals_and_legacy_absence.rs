use super::*;

#[test]
fn snapshot_progress_preserves_unknown_totals_and_legacy_absence() {
    let mut snapshot = daemon_snapshot_fixture();
    let encoded = serde_json::to_value(&snapshot).unwrap();
    assert!(encoded.get("build_progress").is_none());
    let legacy: DaemonSnapshot = serde_json::from_value(encoded).unwrap();
    assert_eq!(legacy.build_progress, None);
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::TaskCompleted {
            recipe: "legacy".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: None,
        },
    );
    assert_eq!(snapshot.build_progress, None);
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
    );
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::TaskQueued {
            recipe: "current".into(),
            task: "do_compile".into(),
            worker: None,
            stats: Some(TaskStatsData {
                completed: 42,
                total: 0,
                active: 1,
                failed: 0,
            }),
        },
    );
    apply_build_event(
        &mut snapshot,
        DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: None,
        },
    );
    assert_eq!(
        snapshot.build_progress,
        Some(DaemonBuildProgress {
            completed: 42,
            total: None,
            ..Default::default()
        })
    );
    for invalid in [
        r#"{"completed":-1,"total":6812}"#,
        r#"{"completed":1,"total":-1}"#,
        r#"{"completed":1,"total":"unknown"}"#,
        r#"{"completed":1.5,"total":6812}"#,
    ] {
        assert!(
            serde_json::from_str::<DaemonBuildProgress>(invalid).is_err(),
            "{invalid}"
        );
    }
}
