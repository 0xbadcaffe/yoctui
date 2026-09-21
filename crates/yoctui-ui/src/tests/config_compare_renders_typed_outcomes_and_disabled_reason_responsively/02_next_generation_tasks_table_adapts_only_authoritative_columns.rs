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
    assert_eq!(
        terminal.backend().buffer()[(1, 1)].symbol(),
        "█",
        "determinate progress must have a visible filled bar"
    );
    assert_eq!(terminal.backend().buffer()[(98, 1)].symbol(), " ");
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
    app.focus = FocusTarget::Workspace;
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
    app.focus = FocusTarget::Workspace;
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
    assert!(narrow.contains("Panes: Navigator  [Workspace]"), "{narrow}");
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
