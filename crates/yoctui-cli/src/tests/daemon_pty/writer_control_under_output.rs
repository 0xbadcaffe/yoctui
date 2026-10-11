use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pty_writer_control_is_confirmed_during_continuous_output() {
    let mut supervisor = DaemonPtySupervisor::default();
    let id = supervisor
        .start_new(
            "console flood".into(),
            PtyKind::Utility,
            std::env::temp_dir().display().to_string(),
            PtyCommand {
                program: "/bin/sh".into(),
                arguments: vec![
                    "-c".into(),
                    "while :; do printf 'console output\\n'; done".into(),
                ],
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if matches!(supervisor.try_event(), Some(DaemonPtyEvent::Started { .. })) {
            break;
        }
        assert!(Instant::now() < deadline, "test console did not start");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let client = PtyClientId([42; 16]);
    supervisor.attach(id, client).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let started = Instant::now();
    let result = supervisor.take_with_summary(id, client, 0);
    let elapsed = started.elapsed();
    // Stop only this test-owned console even if the assertion fails.
    let stopped = supervisor.terminate(id);
    let (epoch, summary) = result.unwrap();
    stopped.unwrap();
    assert!(
        elapsed < Duration::from_secs(2),
        "writer handoff waited for output: {elapsed:?}"
    );
    assert_eq!(summary.writer.unwrap().0, client.0);
    assert_eq!(summary.writer_epoch, epoch);
    assert!(epoch > 0);
}
