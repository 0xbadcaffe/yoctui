use super::*;

#[tokio::test]
async fn sdk_tool_runner_times_out_and_cancels_gracefully_or_forcibly() {
    let (directory, adapter) = fixture("runner-control");
    let preview = publish_preview(&adapter, &directory);
    executable(
        &preview.request.executable,
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
    );
    let preview = SdkPublishPreview::new(
        preview.request.executable,
        preview.request.artifact,
        preview.request.destination,
    )
    .unwrap();
    let command = adapter.publication_command(&preview).unwrap();
    let mut runner = SdkToolJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
    runner.start(command.clone()).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Output { line, .. } if line == "ready"
    ));
    assert!(runner.cancel().await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Cancelled { forced: false, .. }
    ));
    assert!(!runner.cancel().await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::CancellationRejected { .. }
    ));

    executable(
        &preview.request.executable,
        "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let command = adapter.publication_command(&preview).unwrap();
    let mut forced_cancel =
        SdkToolJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
    forced_cancel.start(command.clone()).await.unwrap();
    assert_eq!(
        forced_cancel.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    assert!(matches!(
        forced_cancel.next_event().await.unwrap(),
        SdkToolRunnerEvent::Output { .. }
    ));
    assert!(forced_cancel.cancel().await.unwrap());
    assert!(matches!(
        forced_cancel.next_event().await.unwrap(),
        SdkToolRunnerEvent::Cancelled { forced: true, .. }
    ));

    let mut timed_out = SdkToolJobRunner::new()
        .with_cancellation_timeout(Duration::from_millis(20))
        .with_operation_timeout(Duration::from_millis(20));
    timed_out.start(command).await.unwrap();
    assert_eq!(
        timed_out.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    assert!(matches!(
        timed_out.next_event().await.unwrap(),
        SdkToolRunnerEvent::Output { .. }
    ));
    assert!(matches!(
        timed_out.next_event().await.unwrap(),
        SdkToolRunnerEvent::TimedOut { forced: true, .. }
    ));
}
