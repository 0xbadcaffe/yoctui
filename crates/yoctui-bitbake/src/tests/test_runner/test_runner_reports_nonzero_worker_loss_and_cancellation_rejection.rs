use super::*;

#[tokio::test]
async fn test_runner_reports_nonzero_worker_loss_and_cancellation_rejection() {
    let (_directory, adapter) = fixture("outcomes");
    let mut request = request(&adapter, TestFamily::OeSelftest);
    executable(&request.executable, "#!/bin/sh\nexit 7\n");
    request = TestSelftestRequest::new(
        request.executable,
        request.family,
        request.selector,
        request.parallelism,
        request.verbose,
        request.skip_network,
    )
    .unwrap();
    let command = adapter.command(&request).unwrap();
    let mut runner = TestRunnerJob::new();
    runner.start(command).await.unwrap();
    assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
    assert_eq!(
        runner.next_event().await.unwrap(),
        TestRunnerEvent::Failed { exit_code: Some(7) }
    );
    assert!(!runner.cancel().await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        TestRunnerEvent::CancellationRejected { .. }
    ));

    executable(&request.executable, "#!/bin/sh\nsleep 2\n");
    let request = TestSelftestRequest::new(
        request.executable,
        request.family,
        request.selector,
        request.parallelism,
        request.verbose,
        request.skip_network,
    )
    .unwrap();
    runner
        .start(adapter.command(&request).unwrap())
        .await
        .unwrap();
    assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
    runner.lose_output_channel();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        TestRunnerEvent::Lost { .. }
    ));
}
