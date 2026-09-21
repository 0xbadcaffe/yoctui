use super::*;

#[tokio::test]
async fn qa_layer_runner_rejects_duplicate_and_cancels_gracefully_or_forcibly() {
    let (_root, snapshot) = fixture(
        "cancel",
        "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
    );
    let command =
        QaLayerCommandSpec::from_preview(QaLayerSessionId(11), &preview(&snapshot)).unwrap();
    let mut runner = QaLayerJobRunner::new();
    runner.start(command.clone()).await.unwrap();
    assert!(matches!(
        runner.start(command).await,
        Err(QaLayerAdapterError::Busy)
    ));
    assert!(runner.cancel(QaLayerSessionId(11)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::CancellationRequested { .. }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::Cancelled { forced: false, .. }
    ));

    let (_root, snapshot) = fixture(
        "forced",
        "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
    );
    let command =
        QaLayerCommandSpec::from_preview(QaLayerSessionId(12), &preview(&snapshot)).unwrap();
    let mut runner = QaLayerJobRunner::new().with_cancellation_timeout(Duration::from_millis(10));
    runner.start(command).await.unwrap();
    runner.next_event().await.unwrap();
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(runner.cancel(QaLayerSessionId(12)).await.unwrap());
    runner.next_event().await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::Cancelled { forced: true, .. }
    ));
}
