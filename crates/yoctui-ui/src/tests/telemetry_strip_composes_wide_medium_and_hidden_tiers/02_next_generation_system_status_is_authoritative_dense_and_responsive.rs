#[test]
fn next_generation_system_status_is_authoritative_dense_and_responsive() {
    let mut app = compatibility_ui_inspector_app();
    app.screen = Screen::Dashboard;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.daemon.connected_clients = 3;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 4,
            name: "devshell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon.telemetry = Some(yoctui_model::ClientDaemonTelemetry {
        uptime_seconds: 125,
        active_jobs: 2,
        pty_sessions: 1,
        queue_depth: 3,
        pressure: yoctui_model::ClientDaemonPressureCounters::default(),
        memory_bytes: None,
        recovery: yoctui_model::DaemonRecoveryState::CleanStart,
    });
    app.workspace.build_dir = Some("/work/poky/build".into());
    app.workspace.bitbake_version = Some("2.18.0".into());
    app.host_telemetry.disk_total_bytes = Some(100 * 1024_u64.pow(3));
    app.host_telemetry.disk_available_bytes = Some(40 * 1024_u64.pow(3));

    let wide = system_status_text(&app, 72);
    for expected in [
        "✓ Daemon Connected · Local · version unavailable · up 00:02:05",
        "✓ BitBake Running · v2.18.0 · Jobs 2",
        "! Compat Degraded g7 · A1 L1 U1 ?2 · PTY 1 · Clients 3",
        "✓ Build FS 60% · 40.0/100.0 GiB free · Workspace /work/poky/build",
    ] {
        assert!(wide.contains(expected), "missing {expected}: {wide}");
    }
    assert!(!wide.contains("PID"), "{wide}");

    app.daemon.telemetry.as_mut().unwrap().pressure = yoctui_model::ClientDaemonPressureCounters {
        current_queue_depth: 3,
        maximum_queue_depth: 9,
        cosmetic_coalesced: 2,
        cosmetic_dropped: 4,
        reliable_waits: 1,
        forced_resynchronizations: 2,
        slow_client_disconnects: 1,
    };
    let pressure = system_status_text(&app, 160);
    assert!(pressure.contains("IPC Q 3/9 C2 D4 W1 R2 S1"), "{pressure}");

    app.client_access_origin = yoctui_model::ClientAccessOrigin::Ssh {
        client_ip: "192.0.2.44".into(),
    };
    let ssh = system_status_text(&app, 72);
    assert!(ssh.contains("Daemon Connected · SSH 192.0.2.44"), "{ssh}");

    let compact = system_status_text(&app, 32);
    assert_eq!(compact.lines().count(), 4, "{compact}");
    assert!(compact.lines().all(|line| line.chars().count() <= 32));
    assert!(compact.contains("Daemon Connected"), "{compact}");
    assert!(compact.contains("BitBake Running"), "{compact}");
    assert!(compact.contains("Compat Degraded g7"), "{compact}");
    assert!(
        compact
            .lines()
            .all(|line| matches!(line.chars().next(), Some('✓' | '!' | '✕' | '…' | '–')))
    );

    app.daemon.status = yoctui_model::ClientReplicaStatus::Stale;
    app.daemon.connected_clients = 99;
    app.daemon.telemetry.as_mut().unwrap().active_jobs = 99;
    let stale = system_status_text(&app, 72);
    assert!(stale.contains("Daemon Stale"), "{stale}");
    assert!(stale.contains("up unavailable"), "{stale}");
    assert!(
        stale.contains("BitBake unavailable · vunavailable · Jobs unavailable"),
        "{stale}"
    );
    assert!(
        stale.contains("Compat Unavailable · PTY unavailable · Clients unavailable"),
        "{stale}"
    );
    assert!(
        !stale.contains("99"),
        "stale counts must not render: {stale}"
    );

    app.theme = Theme::HighContrast;
    app.color_enabled = false;
    app.screen = Screen::Tasks;
    let rendered = rendered_text(&app, 180, 44);
    assert!(rendered.contains("System Status"), "{rendered}");
    assert!(rendered.contains("Daemon Stale"), "{rendered}");
}
#[test]
fn next_generation_health_indicators_cover_backend_disk_compat_logs_and_workspace() {
    let mut app = compatibility_ui_inspector_app();
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.daemon.telemetry = Some(yoctui_model::ClientDaemonTelemetry {
        uptime_seconds: 60,
        active_jobs: 1,
        pty_sessions: 0,
        queue_depth: 1,
        pressure: yoctui_model::ClientDaemonPressureCounters::default(),
        memory_bytes: None,
        recovery: yoctui_model::DaemonRecoveryState::CleanStart,
    });
    app.workspace.build_dir = Some("/work/build".into());
    app.host_telemetry.disk_total_bytes = Some(100);
    app.host_telemetry.disk_available_bytes = Some(31);

    let healthy = system_status_projection(&app, 120);
    assert_eq!(healthy[0].tone, StatusTone::Success);
    assert!(healthy[0].text.starts_with("✓ Daemon Connected"));
    assert_eq!(healthy[1].tone, StatusTone::Success);
    assert!(healthy[1].text.starts_with("✓ BitBake Running"));
    assert_eq!(healthy[2].tone, StatusTone::Warning);
    assert!(healthy[2].text.starts_with("! Compat Degraded"));
    assert_eq!(healthy[3].tone, StatusTone::Success);
    assert!(healthy[3].text.starts_with("✓ Build FS 69%"));

    app.host_telemetry.disk_available_bytes = Some(30);
    let warning = system_status_projection(&app, 120);
    assert_eq!(warning[3].tone, StatusTone::Warning);
    assert!(warning[3].text.starts_with("! Build FS 70%"));
    app.host_telemetry.disk_available_bytes = Some(10);
    let error = system_status_projection(&app, 120);
    assert_eq!(error[3].tone, StatusTone::Error);
    assert!(error[3].text.starts_with("✕ Build FS 90%"));

    app.host_telemetry.disk_available_bytes = Some(31);
    app.logs.dropped = 4;
    app.logs.dropped_warnings = 2;
    app.logs.dropped_errors = 1;
    let pressure = system_status_projection(&app, 120);
    assert_eq!(pressure[3].tone, StatusTone::Error);
    assert!(
        pressure[3]
            .text
            .contains("Logs 4 evicted (2 warning/1 error)"),
        "{}",
        pressure[3].text
    );

    app.logs.dropped = 0;
    app.logs.dropped_warnings = 0;
    app.logs.dropped_errors = 0;
    app.workspace.build_dir = None;
    app.workspace.source_dir = None;
    let unknown = system_status_projection(&app, 120);
    assert_eq!(unknown[3].tone, StatusTone::Warning);
    assert!(unknown[3].text.contains("Workspace unknown"));

    for (status, marker, tone) in [
        (
            yoctui_model::ClientReplicaStatus::Disconnected,
            "✕ Daemon Disconnected",
            StatusTone::Error,
        ),
        (
            yoctui_model::ClientReplicaStatus::Synchronizing,
            "… Daemon Synchronizing",
            StatusTone::Pending,
        ),
        (
            yoctui_model::ClientReplicaStatus::Stale,
            "! Daemon Stale",
            StatusTone::Warning,
        ),
    ] {
        app.daemon.status = status;
        let projection = system_status_projection(&app, 120);
        assert_eq!(projection[0].tone, tone);
        assert!(projection[0].text.starts_with(marker));
        assert!(projection[1].text.contains("BitBake unavailable"));
    }

    let render_header = |app: &App| {
        let mut terminal = Terminal::new(TestBackend::new(180, 2)).unwrap();
        terminal
            .draw(|frame| workbench_header(frame, app, frame.area(), literal_now()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    let stale_header = render_header(&app);
    assert!(stale_header.contains("Daemon: ! Stale"), "{stale_header}");
    assert!(
        stale_header.contains("BitBake: – Unavailable"),
        "{stale_header}"
    );
    assert!(!stale_header.contains("BitBake: ✓ Running"));

    app.color_enabled = false;
    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    let mut terminal = Terminal::new(TestBackend::new(50, 6)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new(system_status_document(&app, 48))
                    .block(Block::default().borders(Borders::ALL)),
                frame.area(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| { cell.symbol() == "!" && cell.modifier.contains(Modifier::BOLD) })
    );
}
#[test]
fn dashboard_renders_parse_progress() {
    let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
    let mut app = App::new(10, 1_000);
    app.build.parse_current = Some(8);
    app.build.parse_total = Some(20);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Parse progress: 8/20"));
}
#[test]
fn dashboard_renders_build_exit_code() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.build.exit_code = Some(1);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Exit code: 1"));
}
#[test]
fn build_history_renders_completed_builds() {
    let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::BuildHistory;
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(std::time::Duration::from_secs(65)),
        completed_tasks: 42,
        warnings: 1,
        errors: 0,
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Build history"));
    assert!(output.contains("core-image-minimal"));
    assert!(output.contains("Completed package tasks: 42"));
}
#[test]
fn next_generation_job_history_is_responsive_pinned_and_exact() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::BuildHistory;
    let states = [
        BackgroundJobStatus::Queued,
        BackgroundJobStatus::Starting,
        BackgroundJobStatus::Running,
        BackgroundJobStatus::Cancelling,
        BackgroundJobStatus::Succeeded,
        BackgroundJobStatus::Failed,
        BackgroundJobStatus::Cancelled,
        BackgroundJobStatus::Lost,
    ];
    for (index, status) in states.into_iter().enumerate() {
        let terminal = status.is_terminal();
        app.background_jobs
            .jobs
            .push_back(yoctui_model::BackgroundJob {
                id: yoctui_model::BackgroundJobId(index as u64 + 1),
                kind: BackgroundJobKind::Build,
                title: format!("operation-{index}"),
                status,
                context: yoctui_model::BackgroundJobContext {
                    target: Some("core-image-minimal".into()),
                    recipe: Some("busybox".into()),
                    task: Some("do_compile".into()),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                progress: yoctui_model::BackgroundJobProgress::Indeterminate,
                output: std::collections::VecDeque::new(),
                retained_output_bytes: 0,
                dropped_output_entries: 0,
                warnings: 1,
                errors: usize::from(matches!(
                    status,
                    BackgroundJobStatus::Failed | BackgroundJobStatus::Lost
                )),
                queued_at: UNIX_EPOCH + Duration::from_secs(index as u64),
                started_at: (!matches!(status, BackgroundJobStatus::Queued))
                    .then_some(UNIX_EPOCH + Duration::from_secs(index as u64 + 1)),
                finished_at: terminal.then_some(UNIX_EPOCH + Duration::from_secs(index as u64 + 5)),
                result: (status == BackgroundJobStatus::Succeeded).then_some(
                    yoctui_model::BackgroundJobResult {
                        summary: "completed result".into(),
                        artifacts: Vec::new(),
                    },
                ),
                error: matches!(
                    status,
                    BackgroundJobStatus::Failed | BackgroundJobStatus::Lost
                )
                .then_some(yoctui_model::BackgroundJobError {
                    summary: format!("{status:?} outcome"),
                    detail: Some("exact terminal detail".into()),
                }),
            });
    }

    assert_eq!(
        job_history_columns(70),
        [
            JobHistoryColumn::Status,
            JobHistoryColumn::Operation,
            JobHistoryColumn::Context,
            JobHistoryColumn::Elapsed,
        ]
    );
    assert!(job_history_columns(90).contains(&JobHistoryColumn::Started));
    assert!(!job_history_columns(90).contains(&JobHistoryColumn::Finished));
    assert_eq!(job_history_columns(120).len(), 8);

    let failed = app
            .job_history_rows()
            .iter()
            .position(|row| {
                matches!(row, JobHistoryRowRef::Background(job) if job.status == BackgroundJobStatus::Failed)
            })
            .unwrap();
    app.build_history_selection = failed;
    let output = rendered_text_at(&app, 180, 34, UNIX_EPOCH + Duration::from_secs(100));
    for expected in [
        "4 jobs pinned",
        "· Queued",
        "… Starting",
        "▶ Running",
        "! Cancelling",
        "✓ Succeeded",
        "✕ Failed",
        "■ Cancelled",
        "? Lost",
        "Target / Context",
        "busybox:do_compile",
        "Outcome: Failed outcome — exact terminal detail",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    let first = app.job_history_rows()[0];
    assert!(
        matches!(first, JobHistoryRowRef::Background(job) if !job.status.is_terminal()),
        "active work stays pinned ahead of terminal history"
    );
}
