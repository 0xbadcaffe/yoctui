use super::*;

fn saved(
    outcome: yoctui_model::SavedBuildOutcome,
    id: &str,
    when: u64,
) -> yoctui_model::SavedBuild {
    yoctui_model::SavedBuild {
        id: id.into(),
        target: "obmc-phosphor-image".into(),
        machine: Some("romulus".into()),
        source: None,
        build_dir: None,
        outcome,
        saved_unix_ms: when,
        started_unix_ms: None,
        finished_unix_ms: None,
        logs: vec![yoctui_model::SavedBuildLog {
            unix_ms: when,
            severity: Severity::Error,
            message: "compiler failed in phosphor-host".into(),
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
fn errors_history_renders_resolved_state_and_source_log_action() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Errors;
    app.focus = FocusTarget::Workspace;
    app.error_workspace.view = yoctui_model::ErrorWorkspaceView::History;
    app.error_workspace.history_selection = 1;
    app.saved_builds.records = std::sync::Arc::new(vec![
        saved(yoctui_model::SavedBuildOutcome::Succeeded, "success", 20),
        saved(yoctui_model::SavedBuildOutcome::Failed, "failure", 10),
    ]);
    let output = rendered_text(&app, 180, 45);
    for expected in [
        "2 Past builds (2)",
        "Resolved",
        "phosphor-host",
        "log.do_compile",
        "d/Delete remove resolved history",
    ] {
        assert!(output.contains(expected), "missing {expected:?}: {output}");
    }
}

#[test]
fn errors_history_source_viewer_is_read_only_and_scrollable() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Errors;
    app.error_workspace.viewer = Some(yoctui_model::ErrorLogViewer {
        title: "/tmp/log.do_compile".into(),
        path: Some("/tmp/log.do_compile".into()),
        content: "first\nsecond\nthird".into(),
        scroll: 1,
        loading: false,
        error: None,
    });
    let output = rendered_text(&app, 120, 32);
    assert!(output.contains("Read-only source log"), "{output}");
    assert!(output.contains("second"), "{output}");
    assert!(output.contains("Esc back"), "{output}");
}
