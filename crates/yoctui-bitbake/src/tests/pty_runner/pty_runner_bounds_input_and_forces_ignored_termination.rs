use super::*;

#[tokio::test]
async fn pty_runner_bounds_input_and_forces_ignored_termination() {
    let (root, spec) = fixture("trap '' TERM; echo ready; while :; do sleep 1; done");
    let mut runner = PtyRunner::default().with_termination_grace(Duration::from_millis(20));
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
    assert!(matches!(
        runner
            .input(PtyClientId([2; 16]), 0, &vec![0; MAX_PTY_INPUT_BYTES + 1])
            .await,
        Err(PtyRunnerError::InputTooLarge)
    ));
    assert!(runner.terminate().await.unwrap());
    assert_eq!(
        runner.session().unwrap().lifecycle,
        PtySessionLifecycle::Exited
    );
    assert_eq!(runner.session().unwrap().process_group, None);
    fs::remove_dir_all(root).unwrap();
}
