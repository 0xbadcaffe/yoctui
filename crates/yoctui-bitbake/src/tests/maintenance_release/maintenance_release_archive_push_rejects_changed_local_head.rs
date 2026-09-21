use super::*;

#[tokio::test]
async fn maintenance_release_archive_push_rejects_changed_local_head() {
    let fixture = TestDirectory::new("archive-stale");
    prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
    let snapshot = inspect(&fixture);
    let request = archive_request(&fixture, Some("origin"));
    git_repository(&request.git_dir);
    let local_result = GitArchiveLocalResult::capture(&request).unwrap();
    let (_, command) = git_archive_push_command(
        MaintenanceSessionId(1),
        1,
        &snapshot,
        1,
        request.clone(),
        &local_result,
    )
    .unwrap();
    fs::write(request.git_dir.join(".git/HEAD"), "changed\n").unwrap();
    assert!(matches!(
        MaintenanceSstateJobRunner::new().start(command).await,
        Err(MaintenanceSstateAdapterError::StaleIdentity(_))
    ));
}
