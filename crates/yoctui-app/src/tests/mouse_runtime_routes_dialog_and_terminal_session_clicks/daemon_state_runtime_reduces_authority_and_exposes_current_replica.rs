use super::*;

#[test]
fn daemon_state_runtime_reduces_authority_and_exposes_current_replica() {
    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([7; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let jobs = daemon_job_state_from_app(&yoctui_model::App::new(16, 4096));
    let revision = reduce_daemon_state(
        &mut state,
        yoctui_model::DaemonStateAction::ReplaceJobs(Box::new(jobs)),
    )
    .unwrap();

    assert_eq!(revision.sequence, 1);
    assert_eq!(revision.generation, 1);
    assert!(state.jobs.is_some());
    let replica = client_replica_from_daemon(&state);
    assert_eq!(replica.status, yoctui_model::ClientReplicaStatus::Current);
    assert_eq!(replica.state.as_ref(), Some(&state));
    let snapshot = daemon_protocol_snapshot(&state);
    assert_eq!(snapshot.daemon_instance_id.0, [7; 16]);
    assert_eq!(snapshot.sequence, 1);
    assert_eq!(snapshot.generation, 1);
    assert!(matches!(
        snapshot.bitbake.lifecycle,
        yoctui_protocol::daemon::LifecycleState::Disconnected
    ));
}
