use super::*;

#[test]
fn ux_dashboard_projects_environment_build_and_artifact_next_actions_honestly() {
    let mut app = App::new(16, 4_096);
    app.daemon.status = ClientReplicaStatus::Current;
    app.build_environment = BuildEnvironmentState::Unconfigured;
    let unconfigured = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
    assert_eq!(
        unconfigured.next_action.kind,
        DashboardNextActionKind::ConfigureEnvironment
    );
    assert_eq!(
        unconfigured.health.environment,
        DashboardEnvironmentState::NeedsConfiguration
    );

    app.workspace.source_dir = Some("/work/poky".into());
    app.workspace.build_dir = Some("/work/poky/build".into());
    let build = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
    assert_eq!(build.next_action.kind, DashboardNextActionKind::StartBuild);
    assert_eq!(build.next_action.state, WorkspaceAvailabilityState::Unknown);
    assert!(build.next_action.reason.is_some());

    let id = BackgroundJobId(7);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::Build,
            title: "image build".into(),
            context: BackgroundJobContext::default(),
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id,
            started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "built".into(),
                artifacts: vec!["/deploy/core-image-minimal.wic".into()],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        },
    );
    app.build.status = BuildStatus::Completed;
    let completed = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
    assert_eq!(
        completed.next_action.kind,
        DashboardNextActionKind::InspectArtifacts
    );
    assert_eq!(completed.artifacts.len(), 1);
    assert_eq!(
        completed.artifacts[0].path,
        Path::new("/deploy/core-image-minimal.wic")
    );
    assert_eq!(completed.recent_work.len(), 1);
}
