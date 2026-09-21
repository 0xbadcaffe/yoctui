use super::*;

#[tokio::test]
async fn qemu_adapter_cancels_gracefully_and_escalates_process_groups() {
    for (name, trap, expected_forced) in [
        ("cancel-graceful", "trap 'exit 0' TERM", false),
        ("cancel-forced", "trap '' TERM", true),
    ] {
        let (directory, _, command) = fixture_preview(
            name,
            &format!("{trap}; printf 'ready\\n'; while :; do :; done"),
        );
        let mut runner = QemuJobRunner::new(directory.clone())
            .with_cancellation_timeout(Duration::from_millis(250));
        runner.start(command).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                QemuRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        assert!(!runner.cancel().await.unwrap());
        loop {
            if let QemuRunnerEvent::Cancelled { forced, .. } = runner.next_event().await.unwrap() {
                assert_eq!(forced, expected_forced);
                break;
            }
        }
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::CancellationRejected { message }
                if message.contains("no cancellable")
        ));
        fs::remove_dir_all(directory).unwrap();
    }
}
