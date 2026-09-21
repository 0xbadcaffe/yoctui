use super::*;

#[test]
fn maintenance_release_buildhistory_vector_uses_only_documented_flags() {
    let fixture = TestDirectory::new("buildhistory");
    prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
    let snapshot = inspect(&fixture);
    let request = comparison_request(&fixture);
    let (preview, command) =
        buildhistory_command(MaintenanceSessionId(2), 3, &snapshot, 4, request).unwrap();
    assert_eq!(
        command.arguments(),
        [
            OsString::from("-p"),
            fixture.join("history").into_os_string(),
            OsString::from("-v"),
            OsString::from("-a"),
            OsString::from("-s"),
            OsString::from("-S"),
            OsString::from("-e"),
            OsString::from("images/*"),
            OsString::from("-c"),
            OsString::from("no"),
            OsString::from("HEAD^"),
            OsString::from("HEAD"),
        ]
    );
    assert!(matches!(
        preview.operation,
        MaintenanceOperation::BuildHistoryComparison(_)
    ));
    let mut invalid = comparison_request(&fixture);
    invalid.from_revision = Some("--help".into());
    assert!(buildhistory_command(MaintenanceSessionId(3), 3, &snapshot, 5, invalid).is_err());
}
