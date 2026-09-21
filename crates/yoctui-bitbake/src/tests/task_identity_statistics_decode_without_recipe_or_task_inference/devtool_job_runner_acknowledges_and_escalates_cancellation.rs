use super::*;

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_acknowledges_and_escalates_cancellation() {
    for (name, trap, expected_forced) in [
        ("devtool-runner-graceful", "trap 'exit 0' TERM", false),
        ("devtool-runner-forced", "trap '' TERM", true),
    ] {
        let (script, command) = fake_devtool_command(
            name,
            &format!("{trap}\nprintf 'ready\\n'\nwhile :; do :; done"),
        );
        let mut runner = DevtoolJobRunner::new(std::env::temp_dir())
            .with_cancellation_timeout(Duration::from_millis(250));
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            DevtoolRunnerEvent::Started
        );
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                DevtoolRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        assert!(!runner.cancel().await.unwrap());
        loop {
            if let DevtoolRunnerEvent::Cancelled { forced, .. } = runner.next_event().await.unwrap()
            {
                assert_eq!(forced, expected_forced);
                break;
            }
        }
        fs::remove_file(script).unwrap();
    }
}
