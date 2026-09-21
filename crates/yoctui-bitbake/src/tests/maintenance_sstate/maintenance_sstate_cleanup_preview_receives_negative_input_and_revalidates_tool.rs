use super::*;

#[tokio::test]
async fn maintenance_sstate_cleanup_preview_receives_negative_input_and_revalidates_tool() {
    let (root, snapshot) = fixture(
        "preview-runner",
        "sstate-cache-management.py",
        "#!/bin/sh\nread answer\nprintf '%s\\n' \"$answer\"\nexit 0\n",
    );
    let command = MaintenanceSstateCommandSpec::cleanup_preview(
        MaintenanceSessionId(40),
        &snapshot,
        cleanup_request(&root),
    )
    .unwrap();
    let mut runner = MaintenanceSstateJobRunner::new();
    runner.start(command).await.unwrap();
    runner.next_event().await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Output {
            stream: MaintenanceOutputStream::Stdout,
            line,
            ..
        } if line == "n"
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Completed {
            exit_code: Some(0),
            ..
        }
    ));

    let command = MaintenanceSstateCommandSpec::cleanup_preview(
        MaintenanceSessionId(41),
        &snapshot,
        cleanup_request(&root),
    )
    .unwrap();
    write_executable(
        &root.0.join("bin/sstate-cache-management.py"),
        "#!/bin/sh\nprintf 'tampered\\n'\nexit 0\n",
    );
    assert!(matches!(
        MaintenanceSstateJobRunner::new().start(command).await,
        Err(MaintenanceSstateAdapterError::StaleIdentity(_))
    ));
}
