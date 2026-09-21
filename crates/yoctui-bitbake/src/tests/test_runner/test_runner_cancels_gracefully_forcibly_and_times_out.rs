use super::*;

#[tokio::test]
async fn test_runner_cancels_gracefully_forcibly_and_times_out() {
    let (_directory, adapter) = fixture("control");
    let mut request = request(&adapter, TestFamily::OeSelftest);
    executable(
        &request.executable,
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
    );
    request = TestSelftestRequest::new(
        request.executable,
        request.family,
        request.selector,
        request.parallelism,
        request.verbose,
        request.skip_network,
    )
    .unwrap();
    let mut graceful = TestRunnerJob::new().with_cancellation_timeout(Duration::from_secs(1));
    graceful
        .start(adapter.command(&request).unwrap())
        .await
        .unwrap();
    assert_eq!(
        graceful.next_event().await.unwrap(),
        TestRunnerEvent::Started
    );
    assert!(matches!(
        graceful.next_event().await.unwrap(),
        TestRunnerEvent::Output { .. }
    ));
    assert!(graceful.cancel().await.unwrap());
    assert!(matches!(
        graceful.next_event().await.unwrap(),
        TestRunnerEvent::Cancelled { forced: false, .. }
    ));

    executable(
        &request.executable,
        "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let request = TestSelftestRequest::new(
        request.executable,
        request.family,
        request.selector,
        request.parallelism,
        request.verbose,
        request.skip_network,
    )
    .unwrap();
    let command = adapter.command(&request).unwrap();
    let mut forced = TestRunnerJob::new().with_cancellation_timeout(Duration::from_millis(20));
    forced.start(command.clone()).await.unwrap();
    assert_eq!(forced.next_event().await.unwrap(), TestRunnerEvent::Started);
    assert!(matches!(
        forced.next_event().await.unwrap(),
        TestRunnerEvent::Output { .. }
    ));
    assert!(forced.cancel().await.unwrap());
    assert!(matches!(
        forced.next_event().await.unwrap(),
        TestRunnerEvent::Cancelled { forced: true, .. }
    ));

    let mut timed_out = TestRunnerJob::new()
        .with_cancellation_timeout(Duration::from_millis(20))
        .with_operation_timeout(Duration::from_millis(20));
    timed_out.start(command).await.unwrap();
    assert_eq!(
        timed_out.next_event().await.unwrap(),
        TestRunnerEvent::Started
    );
    assert!(matches!(
        timed_out.next_event().await.unwrap(),
        TestRunnerEvent::Output { .. }
    ));
    assert!(matches!(
        timed_out.next_event().await.unwrap(),
        TestRunnerEvent::TimedOut { forced: true, .. }
    ));
}
