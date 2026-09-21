use super::*;

#[tokio::test]
async fn cli_control_cancels_the_owned_process_group() {
    let (root, executable) = fixture("trap 'exit 0' TERM; : > \"$READY\"; while :; do :; done");
    let ready = root.join("ready");
    let mut command = command(
        executable,
        root.clone(),
        BitBakeCliOperation::StartServer,
        Duration::from_secs(5),
        64,
    );
    command
        .environment
        .insert("READY".into(), ready.to_string_lossy().into_owned());
    let mut runner = BitBakeCliRunner::default().with_cancellation_grace(Duration::from_secs(1));
    runner.start(command).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !ready.is_file() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("fixture did not install its TERM trap");
    let outcome = runner.cancel().await.unwrap();
    assert!(matches!(
        outcome,
        BitBakeCliOutcome::Cancelled { forced: false, .. }
    ));
    assert!(!runner.is_active());
    fs::remove_dir_all(root).unwrap();
}
