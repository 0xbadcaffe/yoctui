use super::*;

#[test]
fn maintenance_service_constructs_only_exact_export_and_import_vectors_with_side_effects() {
    let fixture = TestDirectory::new("vectors");
    prepare_fixture(
        &fixture,
        Some("#!/bin/sh\nprintf '%s:%s\\n' \"$1\" \"$2\"\n"),
    );
    let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
        &fixture,
        Some("localhost:0".into()),
        None,
        None,
    ))
    .unwrap();
    let request = export_request(&inspection, &fixture);
    let (preview, command) = pr_service_command(
        MaintenanceSessionId(1),
        7,
        &inspection.capability,
        9,
        request.clone(),
    )
    .unwrap();
    assert_eq!(
        command.kind(),
        MaintenanceSstateCommandKind::PrServiceExport
    );
    assert_eq!(
        command.arguments(),
        [
            std::ffi::OsString::from("export"),
            request.file.as_os_str().to_owned(),
        ]
    );
    assert_eq!(preview.arguments[1], "1: export");
    assert!(
        preview
            .limitations
            .iter()
            .any(|line| line.contains("memory-resident"))
    );
    assert!(
        preview
            .limitations
            .iter()
            .any(|line| line.contains("configured PR endpoint"))
    );

    let import_file = fixture.join("locked.inc");
    fs::write(&import_file, "PRAUTO$example = 1\n").unwrap();
    let import = PrServiceRequest::new(
        yoctui_model::PrServiceOperation::Import,
        import_file.clone(),
        fixture.join("build"),
        "localhost:0".into(),
    )
    .unwrap();
    let (preview, command) = pr_service_command(
        MaintenanceSessionId(2),
        7,
        &inspection.capability,
        10,
        import,
    )
    .unwrap();
    assert_eq!(
        command.kind(),
        MaintenanceSstateCommandKind::PrServiceImport
    );
    assert_eq!(command.arguments()[0], "import");
    assert_eq!(command.arguments()[1], import_file.as_os_str());
    assert!(
        preview
            .limitations
            .iter()
            .any(|line| line.contains("changes PR service data"))
    );
}
