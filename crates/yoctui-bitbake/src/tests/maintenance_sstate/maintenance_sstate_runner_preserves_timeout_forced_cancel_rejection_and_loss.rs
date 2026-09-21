use super::*;

#[tokio::test]
async fn maintenance_sstate_runner_preserves_timeout_forced_cancel_rejection_and_loss() {
    let (_root, snapshot) = fixture(
        "timeout",
        "sstate-cache-management.py",
        "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let make_command = |id| {
        MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(id),
            1,
            &snapshot,
            id,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap()
        .1
    };
    let mut runner = MaintenanceSstateJobRunner::new()
        .with_operation_timeout(Duration::from_secs(2))
        .with_cancellation_timeout(Duration::from_millis(20));
    runner.start(make_command(7)).await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Started { .. }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Output { ref line, .. } if line == "ready"
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
    ));
    assert!(!runner.cancel(MaintenanceSessionId(99)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRejected { .. }
    ));

    let mut runner = MaintenanceSstateJobRunner::new();
    runner.start(make_command(8)).await.unwrap();
    runner.next_event().await.unwrap();
    runner.lose_output_channel();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Lost { .. }
    ));
}
