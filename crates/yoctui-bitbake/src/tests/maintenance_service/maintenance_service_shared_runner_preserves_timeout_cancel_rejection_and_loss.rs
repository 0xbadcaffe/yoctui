use super::*;

#[tokio::test]
async fn maintenance_service_shared_runner_preserves_timeout_cancel_rejection_and_loss() {
    let forced_fixture = TestDirectory::new("runner-forced-timeout");
    prepare_fixture(
        &forced_fixture,
        Some("#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n"),
    );
    let forced_inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
        &forced_fixture,
        Some("localhost:0".into()),
        None,
        None,
    ))
    .unwrap();
    let mut timed = MaintenanceSstateJobRunner::new()
        .with_operation_timeout(Duration::from_millis(200))
        .with_cancellation_timeout(Duration::from_millis(10));
    timed
        .start(
            pr_service_command(
                MaintenanceSessionId(4),
                1,
                &forced_inspection.capability,
                1,
                export_request(&forced_inspection, &forced_fixture),
            )
            .unwrap()
            .1,
        )
        .await
        .unwrap();
    assert!(matches!(
        terminal_event(&mut timed).await,
        MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
    ));

    let fixture = TestDirectory::new("runner-terminal");
    prepare_fixture(
        &fixture,
        Some("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n"),
    );
    let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
        &fixture,
        Some("localhost:0".into()),
        None,
        None,
    ))
    .unwrap();
    let command = || {
        pr_service_command(
            MaintenanceSessionId(4),
            1,
            &inspection.capability,
            1,
            export_request(&inspection, &fixture),
        )
        .unwrap()
        .1
    };

    let mut cancelled =
        MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(100));
    cancelled.start(command()).await.unwrap();
    cancelled.next_event().await.unwrap();
    assert!(cancelled.cancel(MaintenanceSessionId(4)).await.unwrap());
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRequested { .. }
    ));
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Cancelled { .. }
    ));
    assert!(!cancelled.cancel(MaintenanceSessionId(4)).await.unwrap());
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRejected { .. }
    ));

    let mut lost = MaintenanceSstateJobRunner::new();
    lost.start(command()).await.unwrap();
    lost.next_event().await.unwrap();
    lost.lose_output_channel();
    assert!(matches!(
        lost.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Lost { .. }
    ));
}
