use super::*;

#[test]
fn bitbake_restart_lists_active_work_and_requires_exact_confirmation() {
    let mut app = App::new(128, 1024 * 1024);
    app.build.status = BuildStatus::Running;
    app.build.target = Some("core-image-minimal".into());
    crate::update(
        &mut app,
        crate::Action::QueueBackgroundJob(BackgroundJobSpec {
            id: BackgroundJobId(9),
            kind: BackgroundJobKind::Sdk,
            title: "SDK build".into(),
            context: BackgroundJobContext::default(),
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let jobs = DaemonJobState::capture(&app);
    let affected = bitbake_restart_affected_jobs(&jobs);
    assert_eq!(affected.len(), 2);
    let preview = BitBakeRestartPreview {
        controller_generation: 4,
        server_identity: "server-a".into(),
        affected_jobs: affected,
    };
    assert!(preview.requires_confirmation());
    assert!(preview.validate_confirmation(&preview.confirmation()));
    let mut stale = preview.confirmation();
    stale.controller_generation += 1;
    assert!(!preview.validate_confirmation(&stale));
}
