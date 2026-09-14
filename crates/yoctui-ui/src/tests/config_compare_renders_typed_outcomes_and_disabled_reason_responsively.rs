//! Regression tests grouped around config_compare_renders_typed_outcomes_and_disabled_reason_responsively.
use super::*;

#[test]
fn config_compare_renders_typed_outcomes_and_disabled_reason_responsively() {
    for (width, height) in [(140, 32), (100, 28), (90, 24)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
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

#[test]
fn next_generation_tasks_table_adapts_only_authoritative_columns() {
    let mut app = App::new(20, 2_000);
    app.screen = Screen::Tasks;
    app.reduced_motion = true;
    let mut task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.worker = Some("worker-7".into());
    task.pid = Some(4242);
    app.tasks.insert(task.id.clone(), task);
    let now = UNIX_EPOCH + Duration::from_secs(100);
    let rows = app.visible_task_row_refs_at(now);

    assert_eq!(
        task_table_columns(70, &rows),
        [
            TaskTableColumn::Task,
            TaskTableColumn::State,
            TaskTableColumn::Progress,
        ]
    );
    assert_eq!(
        task_table_columns(89, &rows),
        [
            TaskTableColumn::Task,
            TaskTableColumn::Recipe,
            TaskTableColumn::State,
            TaskTableColumn::Elapsed,
            TaskTableColumn::Progress,
        ]
    );
    assert_eq!(task_table_columns(120, &rows), TaskTableColumn::ALL);

    let render_table = |width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 12)).unwrap();
        terminal
            .draw(|frame| render_task_table(frame, &app, frame.area(), &rows, now))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    let wide = render_table(120);
    for expected in ["Worker", "PID", "worker-7", "4242", "progress unknown"] {
        assert!(wide.contains(expected), "missing {expected}: {wide}");
    }
    assert!(!wide.contains("CPU"), "{wide}");
    assert!(
        TaskTableColumn::ALL
            .into_iter()
            .all(|column| !matches!(column.header(), "CPU" | "ETA"))
    );

    let medium = render_table(89);
    assert!(medium.contains("Recipe"), "{medium}");
    assert!(medium.contains("Time"), "{medium}");
    assert!(!medium.contains("Worker"), "{medium}");
    assert!(!medium.contains("PID"), "{medium}");

    let narrow = render_table(70);
    assert!(narrow.contains("Task"), "{narrow}");
    assert!(narrow.contains("Status"), "{narrow}");
    assert!(narrow.contains("Progress"), "{narrow}");
    assert!(!narrow.contains("Recipe"), "{narrow}");
    assert!(!narrow.contains("Time"), "{narrow}");

    let palette = ThemePalette::for_app(&app);
    let running = task_table_row_style(&app, TaskState::Active, false);
    assert_eq!(running.fg, Some(palette.running));
    assert!(running.add_modifier.contains(Modifier::BOLD));
    assert_eq!(
        task_table_row_style(&app, TaskState::Active, true),
        selected_style(&app, true),
        "selection remains distinct from the running-row treatment"
    );
}

#[test]
fn next_generation_task_states_remain_distinct_without_color() {
    let mut app = App::new(20, 2_000);
    app.screen = Screen::Tasks;
    app.color_enabled = false;
    app.reduced_motion = true;
    let states = [
        TaskState::Queued,
        TaskState::Waiting,
        TaskState::Active,
        TaskState::Completed,
        TaskState::Failed,
        TaskState::Cancelled,
        TaskState::Lost,
    ];
    let tasks = states
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let mut task = yoctui_model::TaskInfo::active(
                yoctui_model::TaskId(format!("recipe-{index}:do_state")),
                format!("recipe-{index}"),
                "do_state".into(),
            );
            task.state = *state;
            task
        })
        .collect::<Vec<_>>();
    let mut rows = tasks
        .iter()
        .zip(states)
        .map(|(task, state)| TaskRowRef::Task { task, state })
        .collect::<Vec<_>>();
    rows.push(TaskRowRef::WaitingSummary(2));

    let mut terminal = Terminal::new(TestBackend::new(120, 14)).unwrap();
    terminal
        .draw(|frame| {
            render_task_table(
                frame,
                &app,
                frame.area(),
                &rows,
                UNIX_EPOCH + Duration::from_secs(100),
            )
        })
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "· Queued",
        "▫ Waiting",
        "▶ Running",
        "✓ Succeeded",
        "✕ Failed",
        "■ Cancelled",
        "? Lost",
        "2 waiting tasks",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    assert_eq!(
        states
            .iter()
            .map(|state| task_state_label(*state))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        states.len(),
        "every lifecycle state has a unique text-and-marker label"
    );
    for state in states {
        let style = task_state_style(&app, state);
        assert_eq!(style.fg, Some(Color::Reset));
        assert_eq!(style.bg, None);
        assert_eq!(
            task_table_row_style(&app, state, true),
            selected_style(&app, true),
            "selection must remain visible for {state:?}"
        );
    }
    assert_ne!(
        task_state_style(&app, TaskState::Queued).add_modifier,
        task_state_style(&app, TaskState::Waiting).add_modifier,
        "queued and aggregate waiting retain different non-color treatment"
    );
}

#[test]
fn next_generation_build_summary_is_determinate_only_with_a_real_total() {
    let now = UNIX_EPOCH + Duration::from_secs(100);
    let mut app = App::new(20, 2_000);
    app.screen = Screen::Tasks;
    app.build.status = BuildStatus::Running;
    app.build.started = Some(UNIX_EPOCH + Duration::from_secs(40));
    app.build.completed = 3;
    app.build.total = Some(10);
    app.build.warnings = 2;
    app.build.errors = 1;
    let active = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    app.tasks.insert(active.id.clone(), active);

    let render_table = |app: &App, width| {
        let rows = app.visible_task_row_refs_at(now);
        let mut terminal = Terminal::new(TestBackend::new(width, 12)).unwrap();
        terminal
            .draw(|frame| render_task_table(frame, app, frame.area(), &rows, now))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        (output, terminal)
    };

    let (known, terminal) = render_table(&app, 100);
    for expected in [
        "Overall  30%  3/10",
        "Active 1",
        "Waiting 6",
        "Warnings 2",
        "Errors 1",
        "Elapsed 00:01:00",
    ] {
        assert!(known.contains(expected), "missing {expected}: {known}");
    }
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.symbol() == "▪"),
        "determinate progress must have a visible filled bar"
    );
    assert!(!known.contains("Sstate"), "{known}");

    app.build.total = None;
    app.reduced_motion = true;
    let (unknown, _) = render_table(&app, 70);
    assert!(unknown.contains("progress unknown ⣿  3/—"), "{unknown}");
    assert!(!unknown.contains("30%"), "{unknown}");
    assert!(unknown.contains("A1 W0 !2 ✕1 00:01:00"), "{unknown}");
}

#[test]
fn workbench_tasks_renders_table_log_history_and_structured_inspector() {
    let mut app = App::new(32, 8_192);
    app.screen = Screen::Tasks;
    app.build.target = Some("core-image-minimal".into());
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    let mut task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.progress = Some(72);
    task.worker = Some("worker-3".into());
    task.log_path = Some("/build/tmp/work/busybox/temp/log.do_compile".into());
    app.tasks.insert(task.id.clone(), task);
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "CC builtins/execute_cmd.o".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/build/tmp/work/busybox/temp/log.do_compile".into()),
        timestamp: UNIX_EPOCH + Duration::from_secs(3_600),
        build: Some("core-image-minimal".into()),
        protected: false,
        diagnostic: None,
    });
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
            id: yoctui_model::BackgroundJobId(857),
            kind: BackgroundJobKind::Build,
            title: "core-image-minimal".into(),
            context: yoctui_model::BackgroundJobContext::default(),
            cancellation_supported: true,
            queued_at: UNIX_EPOCH + Duration::from_secs(3_500),
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id: yoctui_model::BackgroundJobId(857),
            started_at: UNIX_EPOCH + Duration::from_secs(3_550),
        },
    );

    let output = rendered_text_at(&app, 180, 44, UNIX_EPOCH + Duration::from_secs(3_600));
    for expected in [
        "Tasks: core-image-minimal",
        "do_compile",
        "busybox",
        "▶ Running",
        "72%",
        "Log Viewer — do_compile (busybox)",
        "CC builtins/execute_cmd.o",
        "Job History",
        "… Starting",
        "Inspector: Task",
        "Recent Log (tail)",
        "Actions",
        "System Status",
        "Daemon: ✓ Connected",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
}

#[test]
fn workbench_tasks_reduced_height_prioritizes_the_task_table() {
    let mut app = App::new(16, 4_096);
    app.screen = Screen::Tasks;
    let task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("bash:do_compile".into()),
        "bash".into(),
        "do_compile".into(),
    );
    app.tasks.insert(task.id.clone(), task);
    let output = rendered_text(&app, 130, 24);
    assert!(output.contains("Tasks: not selected"), "{output}");
    assert!(output.contains("do_compile"), "{output}");
    assert!(output.contains("Log Viewer"), "{output}");
    assert!(!output.contains("Job History"), "{output}");
}

#[test]
fn workbench_responsive_preserves_task_priority_at_every_breakpoint() {
    let mut app = App::new(16, 4_096);
    app.screen = Screen::Tasks;
    app.build.target = Some("core-image-minimal".into());
    let task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    app.tasks.insert(task.id.clone(), task);

    let wide = rendered_text(&app, 180, 44);
    assert!(wide.contains("CONTENT"), "{wide}");
    assert!(wide.contains("Job History"), "{wide}");
    assert!(wide.contains("System Status"), "{wide}");

    let medium = rendered_text(&app, 100, 30);
    assert!(medium.contains("Navigator"), "{medium}");
    assert!(medium.contains("Log Viewer"), "{medium}");
    assert!(!medium.contains("System Status"), "{medium}");

    let narrow = rendered_text(&app, 80, 24);
    assert!(
        narrow.contains("Panes: Navigator  [Workspace]  Inspector"),
        "{narrow}"
    );
    assert!(narrow.contains("do_compile"), "{narrow}");

    let too_small = rendered_text(&app, 79, 23);
    assert!(too_small.contains("Yoctui needs at least 80x24"));
}

#[test]
fn workbench_responsive_keeps_semantics_in_every_theme_and_no_color() {
    let mut app = App::new(16, 4_096);
    app.screen = Screen::Tasks;
    app.build.status = BuildStatus::Running;
    let mut task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("bash:do_compile".into()),
        "bash".into(),
        "do_compile".into(),
    );
    task.progress = Some(50);
    app.tasks.insert(task.id.clone(), task);
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::VscodeDark,
        Theme::VscodeLight,
        Theme::AccessibleDark,
        Theme::SoftLight,
        Theme::HighContrast,
    ] {
        app.theme = theme;
        app.color_enabled = true;
        let output = rendered_text(&app, 160, 36);
        assert!(output.contains("▶ Running"), "{theme:?}: {output}");
        assert!(output.contains("50%"), "{theme:?}: {output}");
        assert!(output.contains("▾ Tasks"), "{theme:?}: {output}");
    }

    app.color_enabled = false;
    app.focus = FocusTarget::Navigator;
    let mut terminal = Terminal::new(TestBackend::new(160, 36)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.modifier.contains(Modifier::REVERSED))
    );
}

#[test]
fn log_workspace_selection_drives_full_multiline_inspector_details() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Logs;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "compile failed\ncompiler context line".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/tmp/log.do_compile".into()),
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "later output".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: false,
        diagnostic: None,
    });
    app.logs.follow = false;
    app.logs.selection = 0;
    let output = rendered_text(&app, 180, 34);
    assert!(output.contains("do_compile"), "{output}");
    assert!(output.contains("Source: /tmp/log.do_compile"), "{output}");
    assert!(output.contains("compiler context line"), "{output}");
    assert!(output.contains("Build: core-image-minimal"), "{output}");
}
