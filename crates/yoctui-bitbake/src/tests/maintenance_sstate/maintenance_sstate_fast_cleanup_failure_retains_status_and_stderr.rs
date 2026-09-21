use super::*;

#[tokio::test]
async fn maintenance_sstate_fast_cleanup_failure_retains_status_and_stderr() {
    let (root, snapshot) = fixture(
        "fast-preview-failure",
        "sstate-cache-management.py",
        "#!/bin/sh\nexec 0<&-\nprintf 'denied\\n' >&2\nexit 9\n",
    );
    let command = MaintenanceSstateCommandSpec::cleanup_preview(
        MaintenanceSessionId(42),
        &snapshot,
        cleanup_request(&root),
    )
    .unwrap();
    let mut runner = MaintenanceSstateJobRunner::new();
    runner.start(command).await.unwrap();

    let mut stderr = None;
    loop {
        match runner.next_event().await.unwrap() {
            MaintenanceSstateRunnerEvent::Started { .. } => {}
            MaintenanceSstateRunnerEvent::Output {
                stream: MaintenanceOutputStream::Stderr,
                line,
                ..
            } => stderr = Some(line),
            MaintenanceSstateRunnerEvent::Failed {
                exit_code: Some(9), ..
            } => break,
            event => panic!("unexpected event {event:?}"),
        }
    }
    assert_eq!(stderr.as_deref(), Some("denied"));
}
