use super::*;

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_reports_unexpected_event_channel_loss() {
    let (script, command) =
        fake_devtool_command("devtool-runner-channel-loss", "printf 'ready\\n'\nsleep 30");
    let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        DevtoolRunnerEvent::Started
    );
    runner.output = None;
    assert!(matches!(
        runner.next_event().await.unwrap(),
        DevtoolRunnerEvent::Lost { message } if message.contains("channel")
    ));
    assert!(!runner.is_active());
    fs::remove_file(script).unwrap();
}
