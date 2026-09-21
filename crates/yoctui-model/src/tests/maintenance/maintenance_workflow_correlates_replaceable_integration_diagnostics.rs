use super::*;

#[test]
fn maintenance_workflow_correlates_replaceable_integration_diagnostics() {
    let mut state = MaintenanceState::default();
    update_maintenance(&mut state, MaintenanceAction::InspectCapability);
    update_maintenance(&mut state, MaintenanceAction::InspectCapability);
    let unavailable = MaintenanceIntegrationsSnapshot::new(MaintenanceIntegrationsSnapshot {
        pull_request: OptionalPullRequestIntegration {
            state: OptionalIntegrationState::Unavailable,
            create_helper: None,
            send_helper: None,
            worktree: None,
            limitations: vec!["pull-request integration unavailable".into()],
        },
        error_report: OptionalErrorReportIntegration {
            state: OptionalIntegrationState::Unavailable,
            helper: None,
            candidate_report: None,
            limitations: Vec::new(),
        },
        repo_manifest: OptionalRepoManifestIntegration {
            state: OptionalIntegrationState::Unavailable,
            repo_executable: None,
            workspace: None,
            manifest: None,
            limitations: Vec::new(),
        },
        toaster: OptionalToasterIntegration {
            state: OptionalIntegrationState::Unavailable,
            executable: None,
            configurations: Vec::new(),
            observed_processes: Vec::new(),
            limitations: Vec::new(),
        },
        limitations: vec!["optional integrations are partial".into()],
    })
    .unwrap();

    update_maintenance(
        &mut state,
        MaintenanceAction::IntegrationsLoaded {
            request: 1,
            snapshot: Box::new(unavailable.clone()),
            partial: true,
        },
    );
    assert_eq!(
        state.integrations,
        MaintenanceIntegrationDiagnostics::Loading(2)
    );

    update_maintenance(
        &mut state,
        MaintenanceAction::IntegrationsLoaded {
            request: 2,
            snapshot: Box::new(unavailable.clone()),
            partial: true,
        },
    );
    assert!(matches!(
        state.integrations,
        MaintenanceIntegrationDiagnostics::Partial { request: 2, .. }
    ));

    update_maintenance(
        &mut state,
        MaintenanceAction::IntegrationsFailed {
            request: 1,
            message: "stale failure".into(),
        },
    );
    assert!(matches!(
        state.integrations,
        MaintenanceIntegrationDiagnostics::Partial { request: 2, .. }
    ));
}
