use super::*;

#[tokio::test]
async fn maintenance_service_revalidates_tool_file_parent_and_preview_before_spawn() {
    let fixture = TestDirectory::new("revalidate");
    prepare_fixture(&fixture, Some("#!/bin/sh\nexit 0\n"));
    let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
        &fixture,
        Some("localhost:0".into()),
        None,
        None,
    ))
    .unwrap();
    let import_file = fixture.join("locked.conf");
    fs::write(&import_file, "one\n").unwrap();
    let import = PrServiceRequest::new(
        yoctui_model::PrServiceOperation::Import,
        import_file.clone(),
        fixture.join("build"),
        "localhost:0".into(),
    )
    .unwrap();
    let (_, command) = pr_service_command(
        MaintenanceSessionId(1),
        1,
        &inspection.capability,
        1,
        import,
    )
    .unwrap();
    fs::write(&import_file, "changed identity\n").unwrap();
    assert!(matches!(
        MaintenanceSstateJobRunner::new().start(command).await,
        Err(MaintenanceSstateAdapterError::StaleIdentity(path)) if path == import_file
    ));

    let export = export_request(&inspection, &fixture);
    let (_, command) = pr_service_command(
        MaintenanceSessionId(2),
        1,
        &inspection.capability,
        2,
        export,
    )
    .unwrap();
    executable(
        &fixture.join("tools/bitbake-prserv-tool"),
        "#!/bin/sh\necho tampered\nexit 0\n",
    );
    assert!(matches!(
        MaintenanceSstateJobRunner::new().start(command).await,
        Err(MaintenanceSstateAdapterError::StaleIdentity(_))
    ));
}
