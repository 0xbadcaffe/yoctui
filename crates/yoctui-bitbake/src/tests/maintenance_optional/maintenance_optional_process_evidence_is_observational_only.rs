use super::*;

#[test]
fn maintenance_optional_process_evidence_is_observational_only() {
    let fixture = TestDirectory::new("process");
    let mut input = complete_fixture(&fixture);
    fs::remove_file(fixture.join("tools/toaster")).unwrap();
    fs::remove_file(fixture.join("config/toaster.conf")).unwrap();
    input.toaster_configuration_candidates.clear();
    let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
    assert_eq!(
        inspection.toaster.state,
        OptionalIntegrationState::Unavailable
    );
    assert!(!inspection.toaster.observed_processes.is_empty());
    assert!(!inspection.capability.supports(MaintenanceTool::Toaster));
}
