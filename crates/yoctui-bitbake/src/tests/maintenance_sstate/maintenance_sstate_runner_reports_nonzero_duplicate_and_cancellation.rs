use super::*;

#[tokio::test]
async fn maintenance_sstate_runner_reports_nonzero_duplicate_and_cancellation() {
    let (_root, snapshot) = fixture(
        "cancel",
        "sstate-cache-management.py",
        "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
    );
    let (_, command) = MaintenanceSstateCommandSpec::readiness(
        MaintenanceSessionId(5),
        1,
        &snapshot,
        5,
        SstateReadinessRequest::new(
            vec!["busybox".into()],
            SstateReadinessMode::IsolatedTmpdir,
            None,
            None,
            60,
        )
        .unwrap(),
    )
    .unwrap();
    let mut runner = MaintenanceSstateJobRunner::new();
    runner.start(command.clone()).await.unwrap();
    assert!(matches!(
        runner.start(command).await,
        Err(MaintenanceSstateAdapterError::Busy)
    ));
    assert!(runner.cancel(MaintenanceSessionId(5)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRequested { .. }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Cancelled { forced: false, .. }
    ));

    let (_root, snapshot) = fixture(
        "nonzero",
        "sstate-cache-management.py",
        "#!/bin/sh\nexit 7\n",
    );
    let (_, command) = MaintenanceSstateCommandSpec::readiness(
        MaintenanceSessionId(6),
        1,
        &snapshot,
        6,
        SstateReadinessRequest::new(
            vec!["busybox".into()],
            SstateReadinessMode::IsolatedTmpdir,
            None,
            None,
            60,
        )
        .unwrap(),
    )
    .unwrap();
    let mut runner = MaintenanceSstateJobRunner::new();
    runner.start(command).await.unwrap();
    runner.next_event().await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Failed {
            exit_code: Some(7),
            ..
        }
    ));
}
