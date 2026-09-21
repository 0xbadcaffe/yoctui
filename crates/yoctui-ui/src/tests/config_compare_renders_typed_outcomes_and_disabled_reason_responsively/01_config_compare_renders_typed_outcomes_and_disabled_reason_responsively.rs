use super::*;

#[test]
fn config_compare_renders_typed_outcomes_and_disabled_reason_responsively() {
    for (width, height) in [(140, 32), (100, 28), (90, 24)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "base-files".into(),
            ..yoctui_model::Recipe::default()
        });
        app.config_scope = Some("base-files".into());
        let unloaded = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(unloaded.contains("c compare: disabled"), "{unloaded}");
        }
        app.dialogs
            .push_back(Dialog::ConfigComparison(yoctui_model::ConfigComparison {
                variable: "MACHINE".into(),
                recipe: "base-files".into(),
                effective: yoctui_model::ConfigComparisonField {
                    global: Some("qemux86-64".into()),
                    recipe: Some("qemux86-64".into()),
                    outcome: yoctui_model::ConfigComparisonOutcome::Equal,
                },
                unexpanded: yoctui_model::ConfigComparisonField {
                    global: Some("${DEFAULT_MACHINE}".into()),
                    recipe: None,
                    outcome: yoctui_model::ConfigComparisonOutcome::Unavailable,
                },
            }));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Configuration comparison"), "{output}");
        assert!(output.contains("Effective: Equal"), "{output}");
        assert!(output.contains("Unexpanded: Unavailable"), "{output}");
        assert!(output.contains("base-files"), "{output}");
    }
}

#[test]
fn config_edit_preview_renders_availability_editor_and_exact_confirmation_responsively() {
    for (width, height) in [(140, 32), (100, 28), (90, 24)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.focus = FocusTarget::Workspace;
        app.workspace.build_dir = Some("/build".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            yoctui_model::VariableDetail {
                identity: identity.clone(),
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );

        let output = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(output.contains("E edit:"), "{output}");
            assert!(output.contains("enabled"), "{output}");
        }

        let mut editor =
            yoctui_model::PopupEditor::new("# MACHINE\nvalue = \"qemux86-64\"\n".into());
        editor.select_toml_value("value").unwrap();
        app.dialogs.push_back(Dialog::ConfigEdit {
            identity: identity.clone(),
            editor,
        });
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Configuration.toml"), "{output}");
        assert!(output.contains("⟦qemux86-64⟧▏"), "{output}");
        assert!(output.contains("Ctrl+V paste"), "{output}");

        app.dialogs.pop_back();
        app.dialogs.push_back(Dialog::ConfigEditConfirmation(
            yoctui_model::ConfigEditRequest {
                identity,
                value: "qemux86-64".into(),
                destination: "/build/conf/local.conf".into(),
                assignment: "MACHINE = \"qemux86-64\"".into(),
            },
        ));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Preview configuration edit"), "{output}");
        assert!(output.contains("/build/conf/local.conf"), "{output}");
        assert!(output.contains("MACHINE = \"qemux86-64\""), "{output}");
    }

    let mut app = App::new(10, 1_000);
    app.screen = Screen::Configuration;
    app.workspace
        .variables
        .insert("BB_NUMBER_THREADS".into(), "8".into());
    let output = rendered_text(&app, 120, 28);
    assert!(output.contains("E edit: disabled"), "{output}");
    assert!(output.contains("read-only"), "{output}");
}

#[test]
fn bbmask_renders_effective_patterns_and_provenance() {
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Bbmask;
    app.workspace
        .variables
        .insert("BBMASK".into(), "meta-broken/.* meta-old/.*".into());
    app.workspace
        .variable_provenance
        .insert("BBMASK".into(), "conf/local.conf:42".into());
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Effective BBMASK"));
    assert!(output.contains("meta-broken/.*"));
    assert!(output.contains("conf/local.conf:42"));
}
#[test]
fn bbmask_edit_preview_shows_the_exact_assignment() {
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("BBMASK".into(), "meta-broken/.*".into());
    let _ = update(&mut app, Action::BeginBbmaskEdit);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("⟦meta-broken/.*⟧▏"), "{output}");
    assert!(output.contains("Home/End line"), "{output}");
    app.dialogs.clear();
    app.dialogs
        .push_back(Dialog::BbmaskConfirmation("meta-broken/.*".into()));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Confirm BBMASK change"));
    assert!(output.contains("BBMASK = \"meta-broken/.*\""));
}

#[test]
fn live_tasks_renders_summary_states_filters_and_selected_inspector() {
    let mut app = App::new(20, 2_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    app.build.completed = 2;
    app.build.total = Some(5);
    app.build.errors = 1;
    let mut active = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    active.progress = Some(42);
    active.worker = Some("worker-1".into());
    active.pid = Some(4242);
    active.log_path = Some("/build/tmp/work/busybox/temp/log.do_compile".into());
    app.tasks.insert(active.id.clone(), active);
    let mut failed = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("openssl:do_install".into()),
        "openssl".into(),
        "do_install".into(),
    );
    failed.state = TaskState::Failed;
    failed.progress = Some(100);
    app.completed_tasks.push_back(yoctui_model::CompletedTask {
        task: failed,
        success: false,
    });
    let output = rendered_text(&app, 180, 34);
    assert!(output.contains("Overall  40%  2/5"), "{output}");
    assert!(output.contains("Active 1"), "{output}");
    assert!(output.contains("Waiting 2"), "{output}");
    assert!(output.contains("Errors 1"), "{output}");
    assert!(output.contains("✕ Failed"), "{output}");
    assert!(output.contains("· All"), "{output}");
    assert!(output.contains("PID         4242"), "{output}");
    assert!(output.contains("log.do_compile"), "{output}");
}

#[test]
fn next_generation_task_inspector_is_authoritative_bounded_and_responsive() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Inspector;
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        version: Some("1.36.1".into()),
        ..Recipe::default()
    });
    let mut task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.progress = Some(72);
    task.worker = Some("worker-3".into());
    task.pid = Some(85873);
    task.started = Some(UNIX_EPOCH + Duration::from_secs(10));
    task.log_path = Some("/build/tmp/work/busybox/temp/log.do_compile".into());
    task.dependencies = vec![
        yoctui_model::TaskId("busybox:do_configure".into()),
        yoctui_model::TaskId("virtual/libc:do_populate_sysroot".into()),
    ];
    app.tasks.insert(task.id.clone(), task);
    for id in 0..10 {
        app.logs.insert(yoctui_model::LogEntry {
            id,
            severity: if id == 9 {
                Severity::Warning
            } else {
                Severity::Info
            },
            message: format!("compile-line-{id}"),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/build/tmp/work/busybox/temp/log.do_compile".into()),
            timestamp: UNIX_EPOCH + Duration::from_secs(id),
            build: Some("core-image-minimal".into()),
            protected: id == 9,
            diagnostic: None,
        });
    }
    let now = UNIX_EPOCH + Duration::from_secs(70);
    let rows = app.visible_task_row_refs_at(now);
    let mut terminal = Terminal::new(TestBackend::new(60, 44)).unwrap();
    terminal
        .draw(|frame| tasks_inspector(frame, &app, frame.area(), now, &rows))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "Inspector: Task",
        "Task        do_compile",
        "PN          busybox",
        "PV          1.36.1",
        "PR          unavailable",
        "State       ▶ Running",
        "Progress    72%",
        "Worker      worker-3",
        "PID         85873",
        "Elapsed     00:01:00",
        "Workdir     unavailable",
        "log.do_compile",
        "busybox:do_configure",
        "virtual/libc:do_populate_sysroot",
        "compile-line-9",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    assert!(!output.contains("compile-line-0"), "{output}");

    let waiting = [TaskRowRef::WaitingSummary(7)];
    let mut compact = Terminal::new(TestBackend::new(78, 19)).unwrap();
    compact
        .draw(|frame| tasks_inspector(frame, &app, frame.area(), now, &waiting))
        .unwrap();
    let compact = compact
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(compact.contains("7 waiting tasks"), "{compact}");
    assert!(compact.contains("PV          unavailable"), "{compact}");
    assert!(!compact.contains("72%"), "{compact}");
}

#[test]
fn next_generation_inspector_actions_are_aligned_typed_and_accessible() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Inspector;
    app.color_enabled = false;
    app.build.status = BuildStatus::Idle;

    let actions = task_inspector_actions(&app);
    let action = |label: &str| {
        actions
            .iter()
            .find(|action| action.label == label)
            .unwrap_or_else(|| panic!("missing action {label}"))
    };
    assert_eq!(action("Build options").shortcut, "B");
    assert_eq!(action("Open Logs").shortcut, "l");
    assert_eq!(action("Build History").shortcut, "h");
    let cancel = action("Cancel active build");
    assert_eq!(cancel.shortcut, "c");
    assert!(!cancel.enabled);
    assert_eq!(cancel.marker, "×");
    assert_eq!(cancel.state, "Disabled");
    assert!(
        cancel
            .details
            .iter()
            .any(|detail| detail == "Reason: No active build can be cancelled.")
    );
    assert!(action("Open Logs").enabled);
    assert_eq!(action("Open Logs").marker, "✓");

    let plain = action_list_plain(&actions, 52);
    let shortcut_columns = plain
        .lines()
        .filter(|line| !line.starts_with("  "))
        .filter_map(|line| line.chars().position(|character| character == '['))
        .collect::<Vec<_>>();
    assert!(
        shortcut_columns.windows(2).all(|pair| pair[0] == pair[1]),
        "{plain}"
    );
    assert!(plain.contains("[B] — Unknown"), "{plain}");
    assert!(
        plain.contains("No current environment capability snapshot"),
        "{plain}"
    );

    let mut terminal = Terminal::new(TestBackend::new(34, 28)).unwrap();
    terminal
        .draw(|frame| tasks_inspector(frame, &app, frame.area(), UNIX_EPOCH, &[]))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let output = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Contextual Actions"), "{output}");
    assert!(output.contains("× Cancel"), "{output}");
    assert!(output.contains("✓ Open Logs"), "{output}");
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.symbol() == "×" && cell.modifier.contains(Modifier::DIM)),
        "disabled action must retain non-color emphasis"
    );
}

#[test]
fn live_tasks_unknown_progress_and_narrow_layout_are_honest_and_safe() {
    let mut app = App::new(20, 2_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("linux-yocto:do_compile".into()),
        "linux-yocto".into(),
        "do_compile".into(),
    );
    app.tasks.insert(task.id.clone(), task);
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("progress unknown"), "{output}");
    assert!(!output.contains("0%"), "{output}");
    let _ = rendered_text(&app, 50, 16);
}
