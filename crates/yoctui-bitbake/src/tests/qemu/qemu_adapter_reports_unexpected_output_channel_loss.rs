use super::*;

#[tokio::test]
async fn qemu_adapter_reports_unexpected_output_channel_loss() {
    let (directory, _, command) = fixture_preview("channel-loss", "printf 'ready\\n'; sleep 30");
    let mut runner = QemuJobRunner::new(directory.clone());
    runner.start(command).await.unwrap();
    let _ = runner.next_event().await.unwrap();
    let _ = runner.next_event().await.unwrap();
    runner.output = None;
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QemuRunnerEvent::Lost { message } if message.contains("channel")
    ));
    assert!(!runner.is_active());
    fs::remove_dir_all(directory).unwrap();
}
