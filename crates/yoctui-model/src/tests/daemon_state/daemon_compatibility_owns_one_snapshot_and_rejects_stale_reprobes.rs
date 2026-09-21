use super::*;

#[test]
fn daemon_compatibility_owns_one_snapshot_and_rejects_stale_reprobes() {
    let mut daemon = DaemonGlobalState::new(
        DaemonModelInstanceId([9; 16]),
        10,
        "boot-a".into(),
        DaemonStateLimits::default(),
    )
    .unwrap();
    update_daemon_state(
        &mut daemon,
        DaemonStateAction::ReplaceCompatibility(Box::new(daemon_compatibility_snapshot(2))),
    )
    .unwrap();
    assert_eq!(
        daemon.compatibility.as_ref().unwrap().snapshot.generation,
        2
    );

    assert_eq!(
        update_daemon_state(
            &mut daemon,
            DaemonStateAction::ReplaceCompatibility(Box::new(daemon_compatibility_snapshot(1))),
        ),
        Err(DaemonStateError::StaleCompatibilityGeneration {
            current: 2,
            received: 1,
        })
    );
    assert_eq!(daemon.revision.sequence, 1);

    update_daemon_state(&mut daemon, DaemonStateAction::InvalidateCompatibility).unwrap();
    assert!(daemon.compatibility.is_none());
}
