use super::*;

#[test]
fn devtool_job_lifecycle_maps_start_failure_cancel_failure_cancel_and_loss() {
    let now = SystemTime::UNIX_EPOCH;
    let operation = DevtoolOperation::Reset {
        recipe: "busybox".into(),
    };

    let mut coordinator = DevtoolJobCoordinator::default();
    let id = {
        let _ = coordinator.queue(operation.clone(), now);
        coordinator.active_job_id().unwrap()
    };
    assert!(matches!(
        coordinator.start_failed("missing".into(), now).as_slice(),
        [Action::FailBackgroundJob { id: failed, .. }] if *failed == id
    ));

    let mut coordinator = DevtoolJobCoordinator::default();
    let _ = coordinator.queue(operation.clone(), now);
    assert!(matches!(
        coordinator.request_cancellation(),
        Some(Action::RequestBackgroundJobCancellation { .. })
    ));
    assert!(coordinator.request_cancellation().is_none());
    let rejected = coordinator.cancellation_failed("signal".into(), now);
    assert!(matches!(
        rejected.last(),
        Some(Action::RejectBackgroundJobCancellation { .. })
    ));
    assert!(matches!(
        coordinator
            .actions_for_event(
                DevtoolRunnerEvent::Cancelled {
                    forced: true,
                    exit_code: None,
                },
                now,
            )
            .last(),
        Some(Action::CancelBackgroundJob { .. })
    ));

    let mut coordinator = DevtoolJobCoordinator::default();
    let _ = coordinator.queue(operation, now);
    assert!(matches!(
        coordinator
            .actions_for_event(
                DevtoolRunnerEvent::Lost {
                    message: "channel".into(),
                },
                now,
            )
            .as_slice(),
        [Action::LoseBackgroundJob { .. }]
    ));
}
