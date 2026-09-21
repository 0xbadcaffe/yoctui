use super::*;

#[tokio::test]
async fn pty_runner_uses_real_pty_raw_io_and_resize() {
    let (root, spec) = fixture("stty size; IFS= read -r line; printf 'got:%s\\n' \"$line\"");
    let mut runner = PtyRunner::default();
    runner
        .start(
            spec,
            BTreeMap::from([
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("TERM".into(), "xterm-256color".into()),
            ]),
        )
        .await
        .unwrap();
    let client = PtyClientId([1; 16]);
    runner
        .apply_session_action(PtySessionAction::Attach(client))
        .unwrap();
    runner
        .apply_session_action(PtySessionAction::TakeControl {
            client,
            expected_epoch: 0,
        })
        .unwrap();
    runner
        .resize(
            client,
            1,
            PtyDimensions {
                columns: 100,
                rows: 30,
            },
        )
        .unwrap();
    runner.input(client, 1, b"hello\n").await.unwrap();
    let output = collect_until_exit(&mut runner).await;
    let text = String::from_utf8_lossy(&output).replace('\r', "");
    assert!(text.contains("30 100"), "{text:?}");
    assert!(text.contains("got:hello"), "{text:?}");
    assert_eq!(
        runner.session().unwrap().lifecycle,
        PtySessionLifecycle::Exited
    );
    fs::remove_dir_all(root).unwrap();
}
