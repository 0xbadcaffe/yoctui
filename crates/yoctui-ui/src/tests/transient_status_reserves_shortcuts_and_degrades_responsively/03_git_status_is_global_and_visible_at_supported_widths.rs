#[test]
fn git_status_is_global_and_visible_at_supported_widths() {
    let mut app = App::new(10, 1024);
    app.source_git_status = yoctui_model::SourceGitStatus::Ready(yoctui_model::SourceGitSummary {
        branch: "master".into(),
        unstaged: 2,
        upstream: Some("origin/master".into()),
        ..Default::default()
    });
    for width in [80, 100, 160] {
        let mut terminal = Terminal::new(TestBackend::new(width, 5)).unwrap();
        terminal
            .draw(|frame| workbench_header(frame, &app, frame.area(), UNIX_EPOCH))
            .unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("Git: master ~2 synced*"), "{width}: {text}");
    }
}

#[test]
fn gitui_launch_dialog_exposes_source_and_preserves_focus() {
    let mut app = App::new(10, 1024);
    app.gitui_program = Some("/usr/bin/gitui".into());
    app.workspace.source_dir = Some("/workspace/source".into());
    app.source_git_status = yoctui_model::SourceGitStatus::Ready(Default::default());
    update(&mut app, Action::OpenGitUi);
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("gitui"), "{text}");
        assert!(text.contains("/workspace/source"), "{text}");
        assert_eq!(app.focus, FocusTarget::Dialog);
    }
}

#[test]
fn focus_navigation_renders_workspace_then_navigator_at_each_breakpoint() {
    let mut app = App::new(10, 1024);
    yoctui_model::update(&mut app, Action::Open(Screen::Recipes));
    update(&mut app, Action::ActivateNavigator);
    assert_eq!(app.focus, FocusTarget::Workspace);
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Recipes"));
        assert!(text.contains("Navigator"));
    }
    update(&mut app, Action::Focus(FocusTarget::Navigator));
    assert_eq!(app.screen, Screen::Recipes);
}

#[test]
fn offline_screens_retain_navigation_and_explain_connection() {
    let mut app = App::new_unconfigured(32, 4096);
    app.require_daemon = true;
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        for screen in [
            Screen::Dashboard,
            Screen::Logs,
            Screen::Tasks,
            Screen::BuildHistory,
        ] {
            update(&mut app, Action::Open(screen));
            app.focus = FocusTarget::Workspace;
            let text = rendered_text(&app, width, height);
            assert!(text.contains("No build environment"), "{text}");
            assert!(!text.contains(" LIVE"), "{text}");
        }
    }
}

#[test]
fn archive_history_renders_saved_summary_logs_tasks_and_missing_evidence() {
    use yoctui_model::{
        SavedBuild, SavedBuildAction as A, SavedBuildLog, SavedBuildOutcome, SavedBuildTask,
        SavedBuildView,
    };
    let mut app = App::new_unconfigured(32, 4096);
    app.require_daemon = true;
    let record = SavedBuild {
        id: "one".into(),
        target: "core-image-minimal".into(),
        machine: Some("qemuarm64".into()),
        source: Some("/source".into()),
        build_dir: Some("/build".into()),
        outcome: SavedBuildOutcome::Succeeded,
        saved_unix_ms: 3000,
        started_unix_ms: Some(1000),
        finished_unix_ms: Some(3000),
        logs: vec![SavedBuildLog {
            unix_ms: 2000,
            severity: Severity::Error,
            message: "retained compiler diagnostic".into(),
        }],
        tasks: vec![SavedBuildTask {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            status: "Succeeded".into(),
        }],
        limitations: vec!["Bounded saved excerpt".into()],
    };
    update(
        &mut app,
        Action::SavedBuild(A::Loaded {
            records: vec![record],
            notice: None,
        }),
    );
    update(&mut app, Action::Open(Screen::BuildHistory));
    app.saved_builds.loading = false;
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        let list = rendered_text(&app, width, height);
        assert!(list.contains("core-image-minimal"), "{list}");
        for (view, anchor) in [
            (SavedBuildView::Summary, "Duration: 2 s"),
            (SavedBuildView::Logs, "retained compiler diagnostic"),
            (SavedBuildView::Tasks, "busybox:do_compile"),
            (SavedBuildView::Errors, "retained compiler diagnostic"),
        ] {
            app.saved_builds.view = Some(view);
            let text = rendered_text(&app, width, height);
            assert!(text.contains(anchor), "{text}");
            assert!(!text.contains(" LIVE"), "{text}");
        }
        app.saved_builds.view = None;
    }
    assert!(app.logs.entries.is_empty());
    assert!(app.tasks.is_empty());
    std::sync::Arc::make_mut(&mut app.saved_builds.records)[0]
        .logs
        .clear();
    app.saved_builds.view = Some(SavedBuildView::Logs);
    assert!(rendered_text(&app, 100, 30).contains("Saved logs/errors unavailable"));
}
