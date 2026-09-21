use super::*;

#[test]
fn daemon_state_partition_keeps_global_authority_and_client_presentation_distinct() {
    let mut daemon = DaemonGlobalState::new(
        DaemonModelInstanceId([1; 16]),
        10,
        "boot-a".into(),
        DaemonStateLimits {
            logs: 2,
            errors: 2,
            history: 2,
        },
    )
    .unwrap();
    let revision = daemon
        .mutate(|state| {
            state.bitbake.lifecycle = DaemonBitBakeLifecycle::Connected;
            state.bitbake.version = Some("2.8.1".into());
            state
                .recent_logs
                .extend(["one".into(), "two".into(), "three".into()]);
        })
        .unwrap();
    assert_eq!(revision.sequence, 1);
    assert_eq!(revision.generation, 1);
    assert_eq!(
        daemon.recent_logs.iter().cloned().collect::<Vec<_>>(),
        ["two", "three"]
    );

    let mut replica = ClientDaemonReplica::default();
    replica.begin_synchronization();
    replica.replace(daemon.clone());
    let presentation = ClientPresentationState {
        screen: Screen::Layers,
        theme: Theme::WhiteClassic,
        ..ClientPresentationState::default()
    };
    assert_eq!(replica.state.as_ref().unwrap(), &daemon);
    assert_eq!(presentation.screen, Screen::Layers);
    assert_eq!(daemon.revision, revision);
    replica.mark_stale();
    assert_eq!(replica.status, ClientReplicaStatus::Stale);
    replica.disconnect();
    assert_eq!(replica.status, ClientReplicaStatus::Disconnected);
    assert_eq!(replica.state.as_ref().unwrap().revision, revision);
}
