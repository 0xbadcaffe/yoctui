use super::*;

#[tokio::test]
async fn hardening_stress_process_tree_cancellation_reaps_descendant() {
    let (root, snapshot) = fixture(
        "process-tree-stress",
        "sstate-cache-management.py",
        "#!/bin/sh\n(\n  trap '' TERM\n  while :; do sleep 1; done\n) &\ndescendant=$!\nprintf '%s\\n' \"$descendant\" > descendant.pid\ntrap 'wait \"$descendant\"' TERM\nwhile :; do sleep 1; done\n",
    );
    let descendant_file = root.0.join("build/descendant.pid");
    let (_, command) = MaintenanceSstateCommandSpec::readiness(
        MaintenanceSessionId(70),
        1,
        &snapshot,
        70,
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
    let mut runner =
        MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(50));
    runner.start(command).await.unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Started { .. }
    ));

    let mut descendant = None;
    for _ in 0..200 {
        descendant = fs::read_to_string(&descendant_file)
            .ok()
            .and_then(|contents| contents.trim().parse::<i32>().ok());
        if descendant.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let descendant = descendant.expect("fixture did not publish a descendant PID");
    // SAFETY: signal zero only probes the exact child PID written by the fixture.
    assert_eq!(unsafe { libc::kill(descendant, 0) }, 0);

    assert!(runner.cancel(MaintenanceSessionId(70)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::CancellationRequested { .. }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        MaintenanceSstateRunnerEvent::Cancelled { forced: true, .. }
    ));
    let mut reaped = false;
    for _ in 0..200 {
        // SAFETY: signal zero only probes the previously observed fixture PID.
        if unsafe { libc::kill(descendant, 0) } != 0 {
            reaped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(reaped, "cancelled process-group descendant survived");
}
