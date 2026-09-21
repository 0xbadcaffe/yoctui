use super::*;

#[test]
fn maintenance_optional_detects_complete_capabilities_without_side_effects() {
    let fixture = TestDirectory::new("complete");
    let inspection =
        MaintenanceOptionalCapabilityInspector::inspect(complete_fixture(&fixture)).unwrap();
    assert_eq!(
        inspection.pull_request.state,
        OptionalIntegrationState::Available
    );
    assert_eq!(
        inspection.error_report.state,
        OptionalIntegrationState::Available
    );
    assert_eq!(
        inspection.repo_manifest.state,
        OptionalIntegrationState::Available
    );
    assert_eq!(
        inspection.toaster.state,
        OptionalIntegrationState::Available
    );
    assert_eq!(inspection.toaster.observed_processes.len(), 1);
    assert_eq!(inspection.toaster.observed_processes[0].pid, 42);
    assert!(
        inspection
            .toaster
            .limitations
            .iter()
            .any(|value| value.contains("observational"))
    );
    assert!(
        inspection
            .capability
            .tools
            .iter()
            .all(|capability| matches!(
                capability,
                MaintenanceToolCapability::Available {
                    interface: MaintenanceToolInterface::DetectionOnly,
                    ..
                }
            ))
    );
    inspection.revalidate().unwrap();
}
