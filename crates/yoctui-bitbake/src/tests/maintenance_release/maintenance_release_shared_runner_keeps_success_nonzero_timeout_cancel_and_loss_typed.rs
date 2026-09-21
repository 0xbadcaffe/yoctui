use super::*;

#[tokio::test]
async fn maintenance_release_shared_runner_keeps_success_nonzero_timeout_cancel_and_loss_typed() {
    for (name, body, expected_success) in [
        ("runner-success", "#!/bin/sh\necho release\nexit 0\n", true),
        (
            "runner-failure",
            "#!/bin/sh\necho failed >&2\nexit 8\n",
            false,
        ),
    ] {
        let fixture = TestDirectory::new(name);
        prepare_fixture(&fixture, body);
        let snapshot = inspect(&fixture);
        let (_, command) = buildhistory_command(
            MaintenanceSessionId(8),
            1,
            &snapshot,
            1,
            comparison_request(&fixture),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));
        let terminal = terminal_event(&mut runner).await;
        assert_eq!(
            matches!(terminal, MaintenanceSstateRunnerEvent::Completed { .. }),
            expected_success
        );
        if !expected_success {
            assert!(matches!(
                terminal,
                MaintenanceSstateRunnerEvent::Failed {
                    exit_code: Some(8),
                    ..
                }
            ));
        }
    }

    let forced = TestDirectory::new("runner-timeout");
    prepare_fixture(
        &forced,
        "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
    );
    let forced_snapshot = inspect(&forced);
    let (_, forced_command) = buildhistory_command(
        MaintenanceSessionId(9),
        1,
        &forced_snapshot,
        1,
        comparison_request(&forced),
    )
    .unwrap();
    let mut runner = MaintenanceSstateJobRunner::new()
        .with_operation_timeout(Duration::from_millis(200))
        .with_cancellation_timeout(Duration::from_millis(10));
    runner.start(forced_command).await.unwrap();
    assert!(matches!(
        terminal_event(&mut runner).await,
        MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
    ));

    let graceful = TestDirectory::new("runner-cancel");
    prepare_fixture(
        &graceful,
        "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
    );
    let graceful_snapshot = inspect(&graceful);
    let make_command = || {
        buildhistory_command(
            MaintenanceSessionId(10),
            1,
            &graceful_snapshot,
            1,
            comparison_request(&graceful),
        )
        .unwrap()
        .1
    };
    let mut cancelled =
        MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(100));
    cancelled.start(make_command()).await.unwrap();
    cancelled.next_event().await.unwrap();
    assert!(cancelled.cancel(MaintenanceSessionId(10)).await.unwrap());
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRequested { .. }
    ));
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Cancelled { .. }
    ));
    assert!(!cancelled.cancel(MaintenanceSessionId(10)).await.unwrap());
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRejected { .. }
    ));

    let mut lost = MaintenanceSstateJobRunner::new();
    lost.start(make_command()).await.unwrap();
    lost.next_event().await.unwrap();
    lost.lose_output_channel();
    assert!(matches!(
        lost.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Lost { .. }
    ));
}
