use super::*;

#[tokio::test]
async fn sdk_tool_runner_rejects_duplicate_and_reports_nonzero_and_loss() {
    let (directory, adapter) = fixture("runner-outcomes");
    let preview = publish_preview(&adapter, &directory);
    executable(&preview.request.executable, "#!/bin/sh\nexit 7\n");
    let preview = SdkPublishPreview::new(
        preview.request.executable,
        preview.request.artifact,
        preview.request.destination,
    )
    .unwrap();
    let command = adapter.publication_command(&preview).unwrap();
    let mut runner = SdkToolJobRunner::new();
    runner.start(command.clone()).await.unwrap();
    assert_eq!(
        runner.start(command.clone()).await,
        Err(SdkToolAdapterError::Busy)
    );
    assert_eq!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    assert_eq!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Failed { exit_code: Some(7) }
    );

    executable(&preview.request.executable, "#!/bin/sh\nsleep 2\n");
    let command = adapter.publication_command(&preview).unwrap();
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    runner.lose_output_channel();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Lost { .. }
    ));
}
