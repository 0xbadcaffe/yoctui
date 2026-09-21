use super::*;

#[test]
fn archive_legacy_summaries_do_not_invent_logs_or_liveness() {
    let mut snapshot = snapshot();
    snapshot.jobs[0].lifecycle = LifecycleState::Running;
    let persisted = yoctui_protocol::daemon_persist::DaemonPersistedState::capture(
        &snapshot,
        1000,
        "boot".into(),
        Vec::new(),
        Default::default(),
    );
    let records = legacy_saved_builds(&persisted);
    assert_eq!(records[0].outcome, SavedBuildOutcome::Incomplete);
    assert!(records[0].logs.is_empty());
    assert!(records[0].started_unix_ms.is_none());
    assert!(records[0].limitations[0].contains("Legacy summary only"));
}
