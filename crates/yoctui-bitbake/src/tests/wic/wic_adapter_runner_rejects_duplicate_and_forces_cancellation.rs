use super::*;

#[tokio::test]
async fn wic_adapter_runner_rejects_duplicate_and_forces_cancellation() {
    let (directory, output, command) = runner_fixture(
        "runner-cancel",
        "trap '' TERM; printf 'ready\\n'; while :; do :; done",
    )
    .await;
    let mut runner =
        WicJobRunner::new(directory.clone()).with_cancellation_timeout(Duration::from_millis(50));
    runner.start(command.clone(), output.clone()).await.unwrap();
    assert_eq!(
        runner.start(command, output).await.unwrap_err(),
        WicAdapterError::Busy
    );
    assert!(matches!(
        runner.next_event().await.unwrap(),
        WicRunnerEvent::Starting
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        WicRunnerEvent::Started
    ));
    loop {
        if matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Output { ref line, .. } if line == "ready"
        ) {
            break;
        }
    }
    assert!(runner.cancel().await.unwrap());
    let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = runner.next_event().await.unwrap();
            if matches!(event, WicRunnerEvent::Cancelled { .. }) {
                break event;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        cancelled,
        WicRunnerEvent::Cancelled { forced: true, .. }
    ));
    assert!(!runner.cancel().await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        WicRunnerEvent::CancellationRejected { .. }
    ));
    fs::remove_dir_all(directory).unwrap();
}
