use super::*;

#[tokio::test]
async fn compatibility_devtool_daemon_runner_uses_owned_snapshot_and_survives_client_scope() {
    let root = std::env::temp_dir().join(format!("yoctui-daemon-devtool-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("devtool");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf 'started:%s\\n' \"$*\"\nsleep 0.05\nprintf 'finished\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let executable = executable.canonicalize().unwrap();
    let build = root.canonicalize().unwrap();
    let mut supervisor = DaemonDevtoolSupervisor::new(Default::default());
    supervisor
        .replace_compatibility(Some(compatibility(&build, &executable)))
        .unwrap();
    let job = supervisor
        .start(
            DaemonDevtoolOperation::Modify {
                recipe: "busybox".into(),
            },
            build,
        )
        .unwrap();
    let client_connection = String::from("detached-client");
    drop(client_connection);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut saw_output = false;
    let terminal = loop {
        if let Some(event) = supervisor.try_event() {
            saw_output |= matches!(event, DaemonDevtoolEvent::Output { .. });
            if matches!(event, DaemonDevtoolEvent::Completed { .. }) {
                break event;
            }
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    };
    assert!(saw_output);
    assert_eq!(terminal.job_id(), job);
    fs::remove_dir_all(root).unwrap();
}
