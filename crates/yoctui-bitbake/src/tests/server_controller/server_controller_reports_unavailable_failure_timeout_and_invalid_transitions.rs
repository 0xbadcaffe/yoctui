use super::*;

#[tokio::test]
async fn server_controller_reports_unavailable_failure_timeout_and_invalid_transitions() {
    let mut unavailable = FakeAdapter::default();
    unavailable.detections.push_back(None);
    let mut controller =
        BitBakeServerController::new(unavailable, context(), Duration::from_secs(1)).unwrap();
    assert_eq!(
        controller.detect().await.unwrap(),
        BitBakeDetection::Unavailable
    );
    assert!(matches!(
        controller.connect().await,
        Err(BitBakeServerControllerError::InvalidTransition { .. })
    ));

    let failed = FakeAdapter {
        fail: Some("start"),
        ..FakeAdapter::default()
    };
    let mut controller =
        BitBakeServerController::new(failed, context(), Duration::from_secs(1)).unwrap();
    assert!(matches!(
        controller.start().await,
        Err(BitBakeServerControllerError::Adapter {
            operation: BitBakeServerOperation::Start,
            ..
        })
    ));
    assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);

    let slow = FakeAdapter {
        delay: Duration::from_millis(50),
        ..FakeAdapter::default()
    };
    let mut controller =
        BitBakeServerController::new(slow, context(), Duration::from_millis(5)).unwrap();
    assert_eq!(
        controller.detect().await,
        Err(BitBakeServerControllerError::Timeout(
            BitBakeServerOperation::Detect
        ))
    );
    assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);
}
