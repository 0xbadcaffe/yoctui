use super::*;

#[test]
fn log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Logs;
    app.logs.wrap = true;
    app.logs.searching = true;
    app.logs.query = "needle".into();
    app.logs.recipe_filter = Some("busybox".into());
    app.logs.task_filter = Some("do_compile".into());
    app.logs.build_filter = Some("core-image-minimal".into());
    app.logs.coalesced = 7;
    app.logs.dropped = 3;
    app.logs.dropped_warnings = 0;
    app.logs.dropped_errors = 0;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "needle in a long wrapped line".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: false,
        diagnostic: None,
    });
    let output = rendered_text(&app, 220, 24);
    assert!(output.contains("7 coalesced"), "{output}");
    assert!(
        output.contains("[EDITING] Query: needle▏ · 1/1"),
        "{output}"
    );
    assert!(output.contains("B:core-image-minimal"), "{output}");
    let _ = rendered_text(&app, 50, 16);
}

#[test]
fn next_generation_log_viewer_exposes_context_positions_hits_and_real_actions() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Logs;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Warning,
        message: "first needle warning".into(),
        recipe: Some("busybox".into()),
        task: Some("do_patch".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "xx compile NEEDLE context".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/tmp/log.do_compile".into()),
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    app.logs.query = "needle".into();
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.selection = 1;
    app.logs.horizontal_offset = 2;

    let output = rendered_text(&app, 180, 30);
    for expected in [
        "Log Viewer — busybox:do_compile · paused · V 2/2 · H 2/",
        "Ⅱ Paused",
        "! Warning",
        "✕ Error",
        "C Copy",
        "o Open source log",
        "[FILTERED] Query: needle · 2/2",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }

    let spans = log_search_spans(&app, "prefix NeEdLe suffix");
    assert_eq!(
        spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>(),
        "prefix NeEdLe suffix"
    );
    assert_eq!(spans.len(), 3);
    assert_eq!(
        spans[1].style.fg,
        Some(ThemePalette::for_app(&app).accent),
        "the exact normalized search hit uses the semantic accent role"
    );
    app.color_enabled = false;
    let spans = log_search_spans(&app, "prefix needle suffix");
    assert!(
        spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD | Modifier::UNDERLINED)
    );

    app.logs.entries.back_mut().unwrap().path = None;
    let no_source = rendered_text(&app, 180, 30);
    assert!(!no_source.contains("o Open source log"), "{no_source}");
    assert!(no_source.contains("C Copy"), "{no_source}");

    let mut empty = App::new(20, 4_000);
    empty.screen = Screen::Logs;
    let output = rendered_text(&empty, 100, 24);
    assert!(output.contains("No retained log entries."), "{output}");
    empty.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "ordinary output".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: false,
        diagnostic: None,
    });
    empty.logs.query = "absent".into();
    let filtered_empty = rendered_text(&empty, 100, 24);
    assert!(
        filtered_empty.contains("No log entries match the active filters or search."),
        "{filtered_empty}"
    );
}

#[test]
fn next_generation_log_activity_is_compact_complete_and_embedded() {
    let mut app = App::new(20, 4_000);
    assert_eq!(compact_log_activity(&app, 100), "▶ Following");

    app.logs.follow = false;
    app.logs.paused_len = Some(0);
    app.logs.filter = Some(Severity::Warning);
    app.logs.recipe_filter = Some("busybox".into());
    app.logs.query = "compile".into();
    app.logs.searching = true;
    app.logs.dropped = 4;
    app.logs.dropped_warnings = 1;
    app.logs.dropped_errors = 2;
    app.logs.coalesced = 7;

    let wide = compact_log_activity(&app, 100);
    for expected in [
        "Ⅱ Paused",
        "◆ Filtered",
        "/ Search compile",
        "! Evicted 4 [W 1 E 2]",
        "↺ 7 coalesced",
    ] {
        assert!(wide.contains(expected), "missing {expected}: {wide}");
    }
    let compact = compact_log_activity(&app, 40);
    for expected in ["Ⅱ Paused", "◆ Filtered", "/ Search", "! Evicted 4"] {
        assert!(compact.contains(expected), "missing {expected}: {compact}");
    }
    assert!(!compact.contains("[W 1 E 2]"), "{compact}");
    assert!(!compact.contains("compile"), "{compact}");
    assert!(!compact.contains("coalesced"), "{compact}");

    app.screen = Screen::Tasks;
    let task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    let row = TaskRowRef::Task {
        task: &task,
        state: TaskState::Active,
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 8)).unwrap();
    terminal
        .draw(|frame| render_task_log(frame, &app, frame.area(), Some(&row)))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        output.contains("Log Viewer — do_compile (busybox)"),
        "{output}"
    );
    assert!(output.contains("Ⅱ Paused"), "{output}");
    assert!(output.contains("◆ Filtered"), "{output}");
}

#[test]
fn ux_logs_workspace_renders_virtualized_bookmarks_filter_chips_and_bounded_actions() {
    let mut app = App::new(1_000, 500_000);
    app.screen = Screen::Logs;
    app.focus = FocusTarget::Workspace;
    for index in 0..300 {
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: if index == 299 {
                Severity::Warning
            } else {
                Severity::Info
            },
            message: format!("bounded-log-{index:03}-日志"),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/logs/build.log".into()),
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
            build: Some("core-image-minimal".into()),
            protected: index == 299,
            diagnostic: None,
        });
    }
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.selection = 299;
    app.logs.source_filter = Some("/logs/build.log".into());
    app.logs.time_range = yoctui_model::LogTimeRange::LastFiveMinutes;
    assert!(app.logs.toggle_selected_bookmark());

    for (width, height, wrap, color) in [
        (160, 40, false, true),
        (100, 30, true, true),
        (80, 24, false, false),
    ] {
        app.logs.wrap = wrap;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("bounded-log-299"), "{output}");
        assert!(output.contains("★"), "{output}");
        if width >= 120 {
            assert!(output.contains("/logs/build.log"), "{output}");
        } else {
            assert!(output.contains("S:on"), "{output}");
        }
        assert!(output.contains("I:5m"), "{output}");
        assert!(output.contains("E Export"), "{output}");
        assert!(output.contains("m Remove"), "{output}");
        assert!(!output.contains('\u{fffd}'), "{output}");
    }

    let window = app.logs.window(5);
    assert_eq!(window.entries.len(), 5);
    assert_eq!(
        window.entries.last().unwrap().message,
        "bounded-log-299-日志"
    );
}

#[test]
fn ux_internal_log_view_is_separate_bounded_responsive_and_nonvisual() {
    let mut app = App::new(512, 256 * 1024);
    app.screen = Screen::Logs;
    app.focus = FocusTarget::Workspace;
    app.log_workspace_view = LogWorkspaceView::Yoctui;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "BITBAKE-DOMAIN-ONLY".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: true,
        diagnostic: None,
    });
    for index in 0..2_000 {
        app.internal_logs.insert(yoctui_model::InternalLogRecord {
            id: 0,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
            level: if index == 1_999 {
                InternalLogLevel::Error
            } else {
                InternalLogLevel::Debug
            },
            target: if index % 2 == 0 {
                "yoctui::runtime".into()
            } else {
                "yoctui::adapter".into()
            },
            message: format!("d{index:04}-self-diagnostic-日志"),
        });
    }
    app.internal_logs.follow = false;
    app.internal_logs.paused_len = Some(app.internal_logs.entries.len());
    app.internal_logs.selection = app.internal_logs.visible_count().saturating_sub(1);
    app.internal_logs.ingress_dropped = 3;

    for (width, height, color) in [(160, 40, true), (100, 30, true), (80, 24, false)] {
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Yoctui diagnostics"), "{output}");
        assert!(output.contains("local tracing"), "{output}");
        assert!(output.contains("d1999"), "{output}");
        assert!(output.contains("✕ Error"), "{output}");
        assert!(output.contains("ingress dropped 3"), "{output}");
        assert!(output.contains("E Export"), "{output}");
        assert!(!output.contains("BITBAKE-DOMAIN-ONLY"), "{output}");
        assert!(!output.contains('\u{fffd}'), "{output}");
    }

    let window = app.internal_logs.window(7);
    assert_eq!(window.entries.len(), 7);
    app.internal_logs.level_filter = Some(InternalLogLevel::Warning);
    let filtered_empty = rendered_text(&app, 100, 30);
    assert!(
        filtered_empty.contains("No Yoctui diagnostics match"),
        "{filtered_empty}"
    );
}

#[test]
fn error_workspace_renders_structured_columns_inspector_and_related_entries() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Errors;
    app.focus = FocusTarget::Workspace;
    app.build.target = Some("core-image-minimal".into());
    let mut first = yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "compile failed\nfull compiler context".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/tmp/log.do_compile".into()),
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: true,
        diagnostic: None,
    };
    first.build = app.build.target.clone();
    app.logs.insert(first);
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Warning,
        message: "busybox follow-up warning".into(),
        recipe: Some("busybox".into()),
        task: Some("do_package".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    let output = rendered_text(&app, 220, 36);
    assert!(output.contains("Time"), "{output}");
    assert!(output.contains("Severity"), "{output}");
    assert!(output.contains("Summary"), "{output}");
    assert!(output.contains("Category: BitBake error"), "{output}");
    assert!(output.contains("full compiler context"), "{output}");
    assert!(output.contains("Suggested actions"), "{output}");
    assert!(output.contains("busybox follow-up warning"), "{output}");
    assert!(output.contains("/tmp/log.do_compile"), "{output}");
}

#[test]
fn concept_failed_build_composes_summary_filters_correlated_log_and_recovery() {
    let app = concept_failed_errors_app();
    let output = rendered_text_at(&app, 160, 50, literal_now());

    for anchor in [
        "Build Result · Failed",
        "Result: Failed (exit 1)",
        "Diagnostics: 1 error / 1 warning",
        "Correlated-log filters",
        "Errors (checked)",
        "Warnings (checked)",
        "Related task context (indeterminate)",
        "Errors and warnings",
        "Correlated · Paused",
        "match 3/3",
        "loss 2 W1 E1",
        "1-3/3",
        "bash:do_compile failed with exit code 1",
        "Recovery actions",
        "confirmation required",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}: {output}");
    }
}

#[test]
fn error_workspace_and_actionable_failure_completion_are_narrow_safe() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Errors;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "backend connection lost".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: true,
        diagnostic: None,
    });
    let _ = rendered_text(&app, 50, 16);
    app.build.status = yoctui_model::BuildStatus::Failed;
    app.build.errors = 1;
    app.dialogs.push_back(Dialog::BuildCompletion);
    let output = rendered_text(&app, 100, 24);
    assert!(
        output.contains("Press Enter to investigate Errors"),
        "{output}"
    );
}
