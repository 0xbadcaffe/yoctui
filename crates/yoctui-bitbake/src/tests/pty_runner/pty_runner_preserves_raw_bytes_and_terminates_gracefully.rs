use super::*;

#[tokio::test]
async fn pty_runner_preserves_raw_bytes_and_terminates_gracefully() {
    let (root, spec) = fixture("stty raw -echo; printf R; dd bs=1 count=3 2>/dev/null");
    let mut runner = PtyRunner::default();
    runner
        .start(
            spec,
            BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
        )
        .await
        .unwrap();
    let client = PtyClientId([3; 16]);
    runner
        .apply_session_action(PtySessionAction::Attach(client))
        .unwrap();
    runner
        .apply_session_action(PtySessionAction::TakeControl {
            client,
            expected_epoch: 0,
        })
        .unwrap();
    loop {
        if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap()
            && bytes.contains(&b'R')
        {
            break;
        }
    }
    let raw = [0xff, 0xfe, 0x80];
    runner.input(client, 1, &raw).await.unwrap();
    let output = collect_until_exit(&mut runner).await;
    assert!(output.windows(raw.len()).any(|window| window == raw));
    fs::remove_dir_all(root).unwrap();

    let (root, spec) = fixture("trap 'exit 0' TERM; echo ready; while :; do sleep 1; done");
    // Keep the fixture above the one-second process/scheduler boundary used
    // by heavily parallel workspace runs; a responsive TERM trap still
    // completes immediately, while the separate forced-kill test owns the
    // escalation deadline assertion.
    let mut runner = PtyRunner::default().with_termination_grace(Duration::from_secs(3));
    runner
        .start(
            spec,
            BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
        )
        .await
        .unwrap();
    loop {
        if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap()
            && String::from_utf8_lossy(&bytes).contains("ready")
        {
            break;
        }
    }
    assert!(!runner.terminate().await.unwrap());
    assert_eq!(
        runner.session().unwrap().exit_status,
        Some(PtyExitStatus::Code(0))
    );
    fs::remove_dir_all(root).unwrap();
}
