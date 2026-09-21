use super::*;

#[tokio::test]
async fn maintenance_release_locked_signature_revalidates_every_input_before_spawn() {
    let fixture = TestDirectory::new("locked-stale");
    prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
    let snapshot = inspect(&fixture);
    let (_, command, _) = locked_signature_command(
        MaintenanceSessionId(1),
        1,
        &snapshot,
        1,
        locked_request(&fixture),
    )
    .unwrap();
    fs::write(fixture.join("locked.inc"), "changed input identity\n").unwrap();
    assert!(matches!(
        MaintenanceSstateJobRunner::new().start(command).await,
        Err(MaintenanceSstateAdapterError::StaleIdentity(path))
            if path == fixture.join("locked.inc")
    ));
}
