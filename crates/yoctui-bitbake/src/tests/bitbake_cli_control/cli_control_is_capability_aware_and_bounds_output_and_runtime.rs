use super::*;

#[tokio::test]
async fn cli_control_is_capability_aware_and_bounds_output_and_runtime() {
    let (root, executable) = fixture("printf '123456789'; printf 'abcdefghi' >&2; exit 7");
    let status_only = compatibility(&root, &executable, BitBakeCliOperation::Status);
    let unsupported = BitBakeCliCommand::new(
        executable.clone(),
        root.clone(),
        BTreeMap::new(),
        &status_only,
        status_only.snapshot.generation,
        BitBakeCliOperation::StartServer,
    );
    assert!(matches!(
        unsupported,
        Err(BitBakeCliControlError::Authorization(
            crate::BitBakeCommandAuthorizationError::CapabilityMissing { .. }
        ))
    ));
    let mut runner = BitBakeCliRunner::default();
    runner
        .start(command(
            executable.clone(),
            root.clone(),
            BitBakeCliOperation::Status,
            Duration::from_secs(2),
            5,
        ))
        .await
        .unwrap();
    assert_eq!(
        runner.complete().await.unwrap(),
        BitBakeCliOutcome::NonZero {
            exit_code: Some(7),
            stdout: "12345".into(),
            stderr: "abcde".into(),
            truncated: true,
        }
    );

    fs::write(
        &executable,
        "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
    )
    .unwrap();
    let mut runner = BitBakeCliRunner::default().with_cancellation_grace(Duration::from_millis(20));
    runner
        .start(command(
            executable,
            root.clone(),
            BitBakeCliOperation::Status,
            Duration::from_millis(20),
            64,
        ))
        .await
        .unwrap();
    assert!(matches!(
        runner.complete().await.unwrap(),
        BitBakeCliOutcome::TimedOut { forced: true, .. }
    ));
    fs::remove_dir_all(root).unwrap();
}
