use super::*;

fn build(id: &str, saved: u64, outcome: SavedBuildOutcome, machine: &str) -> SavedBuild {
    SavedBuild {
        id: id.into(),
        target: "obmc-phosphor-image".into(),
        machine: Some(machine.into()),
        source: None,
        build_dir: None,
        outcome,
        saved_unix_ms: saved,
        started_unix_ms: Some(saved.saturating_sub(10)),
        finished_unix_ms: Some(saved),
        logs: vec![SavedBuildLog {
            unix_ms: saved,
            severity: Severity::Error,
            message: "compile failed".into(),
            recipe: Some("phosphor-host".into()),
            task: Some("do_compile".into()),
            path: Some("/tmp/log.do_compile".into()),
            build: Some("obmc-phosphor-image".into()),
        }],
        tasks: Vec::new(),
        limitations: Vec::new(),
    }
}

#[test]
fn errors_history_marks_only_a_newer_matching_success_as_resolved() {
    let failed = build("failed", 10, SavedBuildOutcome::Failed, "romulus");
    let other_machine = build("other", 30, SavedBuildOutcome::Succeeded, "qemuarm");
    assert!(!saved_build_is_resolved(
        &[other_machine, failed.clone()],
        &failed
    ));
    let success = build("success", 20, SavedBuildOutcome::Succeeded, "romulus");
    let records = vec![success, failed.clone()];
    assert!(saved_build_is_resolved(&records, &failed));
    let errors = historical_errors(&records);
    assert!(
        errors
            .iter()
            .any(|entry| entry.build.id == "failed" && entry.resolved)
    );
    let mut unknown = failed.clone();
    unknown.machine = None;
    let mut unknown_success = unknown.clone();
    unknown_success.id = "unknown-success".into();
    unknown_success.saved_unix_ms = 30;
    unknown_success.outcome = SavedBuildOutcome::Succeeded;
    assert!(!saved_build_is_resolved(
        &[unknown_success, unknown.clone()],
        &unknown
    ));
}

#[test]
fn errors_history_opens_retained_path_in_bounded_viewer() {
    let mut app = App::new(32, 4096);
    app.saved_builds.records = std::sync::Arc::new(vec![build(
        "failed",
        10,
        SavedBuildOutcome::Failed,
        "romulus",
    )]);
    app.error_workspace.view = ErrorWorkspaceView::History;
    assert_eq!(
        update(&mut app, Action::OpenSelectedErrorLog),
        Some(Effect::LoadErrorLog("/tmp/log.do_compile".into()))
    );
    update(
        &mut app,
        Action::ErrorLogLoaded {
            path: "/tmp/log.do_compile".into(),
            content: "one\ntwo\nthree".into(),
        },
    );
    update(&mut app, Action::ScrollErrorLog { delta: 2 });
    let viewer = app.error_workspace.viewer.as_ref().unwrap();
    assert_eq!(viewer.scroll, 2);
    assert_eq!(viewer.content, "one\ntwo\nthree");
}

#[test]
fn errors_resolved_cleanup_requires_proof_and_confirmation() {
    let mut app = App::new(32, 4096);
    let failed = build("failed", 10, SavedBuildOutcome::Failed, "romulus");
    app.saved_builds.records = std::sync::Arc::new(vec![failed.clone()]);
    app.error_workspace.view = ErrorWorkspaceView::History;
    assert!(update(&mut app, Action::RequestResolvedBuildRemoval).is_none());
    assert!(app.active_dialog().is_none());

    let success = build("success", 20, SavedBuildOutcome::Succeeded, "romulus");
    app.saved_builds.records = std::sync::Arc::new(vec![success, failed]);
    app.error_workspace.history_selection = 1;
    app.notification = None;
    update(&mut app, Action::RequestResolvedBuildRemoval);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ResolvedBuildRemovalConfirmation { id, .. }) if id == "failed"
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmResolvedBuildRemoval),
        Some(Effect::RemoveSavedBuild("failed".into()))
    );
}
