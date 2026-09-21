use super::*;

#[test]
fn maintenance_integration_snapshot_validates_state_and_bounds_typed_evidence() {
    let file = |path: &str| MaintenanceFileIdentity::new(path.into(), 1, UNIX_EPOCH).unwrap();
    let directory =
        |path: &str| MaintenanceDirectoryIdentity::new(path.into(), UNIX_EPOCH).unwrap();
    let mut snapshot = MaintenanceIntegrationsSnapshot {
        pull_request: OptionalPullRequestIntegration {
            state: OptionalIntegrationState::Available,
            create_helper: Some(file("/tools/create-pull-request")),
            send_helper: Some(file("/tools/send-pull-request")),
            worktree: Some(MaintenanceGitWorktreeIdentity {
                root: directory("/sources/poky"),
                head: file("/sources/poky/.git/HEAD"),
            }),
            limitations: Vec::new(),
        },
        error_report: OptionalErrorReportIntegration {
            state: OptionalIntegrationState::Partial,
            helper: Some(file("/tools/send-error-report")),
            candidate_report: None,
            limitations: vec!["report unavailable".into()],
        },
        repo_manifest: OptionalRepoManifestIntegration {
            state: OptionalIntegrationState::Unavailable,
            repo_executable: None,
            workspace: None,
            manifest: None,
            limitations: vec!["repo unavailable".into()],
        },
        toaster: OptionalToasterIntegration {
            state: OptionalIntegrationState::Available,
            executable: Some(file("/tools/toaster")),
            configurations: vec![file("/config/toaster.conf"), file("/config/toaster.conf")],
            observed_processes: vec![
                ServiceProcessEvidence::new(42, "toaster".into()).unwrap(),
                ServiceProcessEvidence::new(42, "toaster".into()).unwrap(),
            ],
            limitations: vec!["observational only".into()],
        },
        limitations: Vec::new(),
    };
    let normalized = MaintenanceIntegrationsSnapshot::new(snapshot.clone()).unwrap();
    assert_eq!(normalized.toaster.configurations.len(), 1);
    assert_eq!(normalized.toaster.observed_processes.len(), 1);

    snapshot.pull_request.state = OptionalIntegrationState::Partial;
    assert!(MaintenanceIntegrationsSnapshot::new(snapshot).is_err());
}
