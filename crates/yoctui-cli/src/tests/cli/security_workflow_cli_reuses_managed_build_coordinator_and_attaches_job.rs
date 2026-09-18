use super::*;

#[tokio::test]
async fn security_workflow_cli_reuses_managed_build_coordinator_and_attaches_job() {
    let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
    let mut app = fixture.app();
    let input =
        security_capability_input(&app, fixture.build.clone(), vec![fixture.bin.clone()]).unwrap();
    let capability = SecurityCapabilityInspector::new(input).inspect().unwrap();
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::CapabilityLoaded(capability)),
    );
    let _ = update(&mut app, Action::Security(SecurityAction::BeginCveCheck));
    let preview = match app.active_dialog().cloned() {
        Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
        other => panic!("unexpected build dialog: {other:?}"),
    };
    let effect = update(
        &mut app,
        Action::Security(SecurityAction::ConfirmOperation(preview)),
    )
    .unwrap();
    let Effect::Security(SecurityEffect::StartBuild { id, request }) = effect else {
        panic!("Security build did not produce its typed effect");
    };
    let started = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut backend: Box<dyn BitBakeBackend> = Box::new(SecurityBuildBackend {
        started: started.clone(),
        fail_start: false,
    });
    let mut jobs = BuildJobCoordinator::default();
    assert!(begin_security_build(&mut backend, &mut app, &mut jobs, id, request.clone()).await);
    assert_eq!(*started.lock().unwrap(), [request]);
    let background_job_id = jobs.active_job_id().unwrap();
    assert_eq!(
        app.security.sessions.last().unwrap().background_job_id,
        Some(background_job_id)
    );
    assert_eq!(
        app.background_jobs.get(background_job_id).unwrap().kind,
        yoctui_model::BackgroundJobKind::CveCheck
    );
}
