use super::*;

#[tokio::test]
async fn qemu_adapter_reports_successful_completion() {
    let (directory, _, command) = fixture_preview("success", "exit 0");
    let mut runner = QemuJobRunner::new(directory.clone());
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        QemuRunnerEvent::Starting
    );
    assert_eq!(runner.next_event().await.unwrap(), QemuRunnerEvent::Started);
    assert_eq!(
        runner.next_event().await.unwrap(),
        QemuRunnerEvent::Completed { exit_code: 0 }
    );
    fs::remove_dir_all(directory).unwrap();
}
