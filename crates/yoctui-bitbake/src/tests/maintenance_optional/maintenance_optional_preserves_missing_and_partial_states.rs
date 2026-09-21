use super::*;

#[test]
fn maintenance_optional_preserves_missing_and_partial_states() {
    let fixture = TestDirectory::new("partial");
    let mut input = complete_fixture(&fixture);
    fs::remove_file(fixture.join("tools/send-pull-request")).unwrap();
    fs::remove_file(fixture.join("report.json")).unwrap();
    fs::remove_file(fixture.join("tools/repo")).unwrap();
    fs::remove_file(fixture.join("config/toaster.conf")).unwrap();
    input.toaster_configuration_candidates.clear();
    let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
    assert_eq!(
        inspection.pull_request.state,
        OptionalIntegrationState::Partial
    );
    assert_eq!(
        inspection.error_report.state,
        OptionalIntegrationState::Partial
    );
    assert_eq!(
        inspection.repo_manifest.state,
        OptionalIntegrationState::Partial
    );
    assert_eq!(inspection.toaster.state, OptionalIntegrationState::Partial);
    assert!(
        inspection
            .pull_request
            .limitations
            .iter()
            .any(|value| value.contains("send-pull-request"))
    );
}
