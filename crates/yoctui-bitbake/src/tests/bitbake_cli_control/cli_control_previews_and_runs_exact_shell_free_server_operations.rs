use super::*;

#[tokio::test]
async fn cli_control_previews_and_runs_exact_shell_free_server_operations() {
    let (root, executable) = fixture("printf '%s:%s' \"$1\" \"$MARKER\"");
    for (operation, expected) in [
        (BitBakeCliOperation::Status, "--status-only:captured"),
        (BitBakeCliOperation::StartServer, "--server-only:captured"),
        (BitBakeCliOperation::StopServer, "--kill-server:captured"),
    ] {
        let command = command(
            executable.clone(),
            root.clone(),
            operation,
            Duration::from_secs(2),
            1024,
        );
        assert_eq!(
            command.preview().argv,
            vec![
                executable.clone().into_os_string(),
                OsString::from(operation_parts(operation).1)
            ]
        );
        assert_eq!(command.preview().cwd, root);
        let mut runner = BitBakeCliRunner::default();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.complete().await.unwrap(),
            BitBakeCliOutcome::Succeeded {
                exit_code: 0,
                stdout: expected.into(),
                stderr: String::new(),
                truncated: false,
            }
        );
    }
    fs::remove_dir_all(root).unwrap();
}
