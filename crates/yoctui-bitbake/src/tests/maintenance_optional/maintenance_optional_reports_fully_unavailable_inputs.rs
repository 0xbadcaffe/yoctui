use super::*;

#[test]
fn maintenance_optional_reports_fully_unavailable_inputs() {
    let fixture = TestDirectory::new("missing");
    for directory in ["build", "tools", "proc"] {
        fs::create_dir_all(fixture.join(directory)).unwrap();
    }
    let inspection =
        MaintenanceOptionalCapabilityInspector::inspect(MaintenanceOptionalCapabilityInput {
            build_dir: fixture.join("build"),
            executable_search_path: vec![fixture.join("tools")],
            git_worktree_candidates: Vec::new(),
            error_report_candidates: Vec::new(),
            repo_workspace_candidates: Vec::new(),
            toaster_configuration_candidates: Vec::new(),
            process_root: fixture.join("proc"),
        })
        .unwrap();
    assert_eq!(
        inspection.pull_request.state,
        OptionalIntegrationState::Unavailable
    );
    assert_eq!(
        inspection.error_report.state,
        OptionalIntegrationState::Unavailable
    );
    assert_eq!(
        inspection.repo_manifest.state,
        OptionalIntegrationState::Unavailable
    );
    assert_eq!(
        inspection.toaster.state,
        OptionalIntegrationState::Unavailable
    );
    assert!(
        inspection
            .capability
            .tools
            .iter()
            .all(|capability| matches!(capability, MaintenanceToolCapability::Unavailable { .. }))
    );
}
