use super::*;

#[tokio::test]
async fn wic_adapter_runner_times_out_without_blocking_forever() {
    let (directory, output, command) = runner_fixture("runner-timeout", "sleep 30").await;
    let mut runner =
        WicJobRunner::new(directory.clone()).with_execution_timeout(Duration::from_millis(20));
    runner.start(command, output).await.unwrap();
    let _ = runner.next_event().await.unwrap();
    let _ = runner.next_event().await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        WicRunnerEvent::Failed {
            ref message,
            exit_code: None
        } if message.contains("timed out")
    ));
    fs::remove_dir_all(directory).unwrap();
}
