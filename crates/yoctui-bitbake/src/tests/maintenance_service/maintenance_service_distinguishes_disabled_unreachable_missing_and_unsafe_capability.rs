use super::*;

#[test]
fn maintenance_service_distinguishes_disabled_unreachable_missing_and_unsafe_capability() {
    let fixture = TestDirectory::new("states");
    prepare_fixture(&fixture, None);
    let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
        &fixture,
        None,
        Some("192.0.2.1:9".into()),
        None,
    ))
    .unwrap();
    assert_eq!(
        service(&inspection, ServiceKind::Pr).state,
        ServiceState::Disabled
    );
    assert_eq!(
        service(&inspection, ServiceKind::Hash).endpoints[0].location,
        ServiceLocation::Remote
    );
    assert_eq!(
        service(&inspection, ServiceKind::Hash).state,
        ServiceState::Unreachable
    );
    assert!(matches!(
        inspection
            .capability
            .capability(MaintenanceTool::PrServiceTool),
        Some(MaintenanceToolCapability::Unavailable { .. })
    ));

    let real = fixture.join("real-tool");
    executable(&real, "#!/bin/sh\nexit 0\n");
    #[cfg(unix)]
    symlink(&real, fixture.join("tools/bitbake-prserv-tool")).unwrap();
    let unsafe_tool = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
        &fixture,
        Some("localhost:0".into()),
        None,
        None,
    ))
    .unwrap();
    assert!(
        unsafe_tool
            .limitations
            .iter()
            .any(|line| line.contains("unsafe executable"))
    );

    let mut missing_process = fixture_input(&fixture, Some("localhost:0".into()), None, None);
    missing_process.process_root = fixture.join("missing-proc");
    let missing_process = MaintenanceServiceCapabilityInspector::inspect(missing_process).unwrap();
    assert_eq!(
        service(&missing_process, ServiceKind::Worker).state,
        ServiceState::Unavailable
    );
    assert_eq!(
        service(&missing_process, ServiceKind::Pr).state,
        ServiceState::Partial
    );
}
