use super::*;

#[tokio::test]
async fn devtool_status_inspection_runs_in_the_daemon_worker() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-daemon-devtool-status-{}",
        std::process::id()
    ));
    let source = root.join("workspace/busybox");
    fs::create_dir_all(&source).unwrap();
    let executable = root.join("devtool");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nif [ \"$1\" = status ]; then printf 'busybox: {} (/layers/busybox.bb)\\n'; else exit 9; fi\n",
            source.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let executable = executable.canonicalize().unwrap();
    let build = root.canonicalize().unwrap();
    let mut supervisor = DaemonDevtoolSupervisor::new(Default::default());
    supervisor
        .replace_compatibility(Some(compatibility(&build, &executable)))
        .unwrap();
    supervisor
        .inspect_status(
            yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/busybox.bb".into(),
            },
            build,
        )
        .unwrap();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(DaemonDevtoolEvent::Status(status)) = supervisor.try_event() {
            break status;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    };
    assert_eq!(status.identity.name, "busybox");
    assert!(matches!(
        status.workspace,
        yoctui_model::DevtoolWorkspace::Present { ref source_path, .. }
            if source_path == &source
    ));
    fs::remove_dir_all(root).unwrap();
}
