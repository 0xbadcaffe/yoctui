use super::*;

#[tokio::test]
async fn maintenance_sstate_runner_streams_bounded_output_and_terminal_status() {
    let (_root, snapshot) = fixture(
        "runner",
        "sstate-cache-management.py",
        &format!(
            "#!/bin/sh\nprintf 'out\\n'\nprintf '%*s\\n' {} x >&2\nexit 0\n",
            MAX_MAINTENANCE_TEXT_BYTES + 64
        ),
    );
    let (_, command) = MaintenanceSstateCommandSpec::readiness(
        MaintenanceSessionId(4),
        1,
        &snapshot,
        4,
        SstateReadinessRequest::new(
            vec!["busybox".into()],
            SstateReadinessMode::IsolatedTmpdir,
            None,
            None,
            60,
        )
        .unwrap(),
    )
    .unwrap();
    let mut runner = MaintenanceSstateJobRunner::new();
    runner.start(command).await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Started { .. }
    ));
    let mut truncated = false;
    loop {
        match runner.next_event().await.unwrap() {
            MaintenanceSstateRunnerEvent::Output {
                truncated: value, ..
            } => truncated |= value,
            MaintenanceSstateRunnerEvent::Completed {
                exit_code: Some(0), ..
            } => break,
            event => panic!("unexpected event {event:?}"),
        }
    }
    assert!(truncated);
}
