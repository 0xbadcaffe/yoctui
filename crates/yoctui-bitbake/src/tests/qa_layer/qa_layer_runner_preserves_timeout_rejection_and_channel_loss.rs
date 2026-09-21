use super::*;

#[tokio::test]
async fn qa_layer_runner_preserves_timeout_rejection_and_channel_loss() {
    let (_root, snapshot) = fixture(
        "terminal",
        "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
    );
    let command =
        QaLayerCommandSpec::from_preview(QaLayerSessionId(13), &preview(&snapshot)).unwrap();
    let mut runner = QaLayerJobRunner::new()
        .with_operation_timeout(Duration::from_millis(1))
        .with_cancellation_timeout(Duration::from_millis(1));
    runner.start(command).await.unwrap();
    runner.next_event().await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::TimedOut { .. }
    ));
    assert!(!runner.cancel(QaLayerSessionId(99)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::CancellationRejected { .. }
    ));

    let (_root, snapshot) = fixture("loss", "#!/bin/sh\nsleep 30\n");
    let command =
        QaLayerCommandSpec::from_preview(QaLayerSessionId(14), &preview(&snapshot)).unwrap();
    let mut runner = QaLayerJobRunner::new();
    runner.start(command).await.unwrap();
    runner.next_event().await.unwrap();
    runner.lose_output_channel();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::Lost { .. }
    ));
}
