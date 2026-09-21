use super::*;

#[test]
fn maintenance_release_archive_separates_local_result_from_network_push() {
    let fixture = TestDirectory::new("archive");
    prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
    let snapshot = inspect(&fixture);
    let request = archive_request(&fixture, Some("origin"));
    let (local_preview, local_command) =
        git_archive_local_command(MaintenanceSessionId(4), 1, &snapshot, 6, &request).unwrap();
    assert_eq!(
        local_command.kind(),
        MaintenanceSstateCommandKind::GitArchiveLocal
    );
    assert!(
        !local_command
            .arguments()
            .iter()
            .any(|argument| argument == "--push")
    );
    assert!(!local_preview.operation.network_side_effect());

    git_repository(&request.git_dir);
    let local_result = GitArchiveLocalResult::capture(&request).unwrap();
    let (push_preview, push_command) = git_archive_push_command(
        MaintenanceSessionId(5),
        1,
        &snapshot,
        7,
        request.clone(),
        &local_result,
    )
    .unwrap();
    assert_eq!(
        push_command.kind(),
        MaintenanceSstateCommandKind::GitArchivePush
    );
    let push_index = push_command
        .arguments()
        .iter()
        .position(|argument| argument == "--push")
        .unwrap();
    assert_eq!(push_command.arguments()[push_index + 1], "origin");
    assert!(push_preview.operation.network_side_effect());
    assert!(
        push_preview
            .limitations
            .iter()
            .any(|line| line.contains("after the retained local archive result"))
    );
}
