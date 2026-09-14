//! Regression tests grouped around next_generation_telemetry_strip_composes_wide_medium_and_hidden_tiers.
use super::*;

#[test]
fn next_generation_telemetry_strip_composes_wide_medium_and_hidden_tiers() {
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some("/work/build".into());
    for cpu in [12, 38, 71, 42] {
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::HostTelemetryUpdated(yoctui_model::HostTelemetry {
                cpu_utilization_percent: Some(cpu),
                logical_cpu_count: Some(16),
                memory_total_bytes: Some(16 * 1024 * 1024 * 1024),
                memory_available_bytes: Some(4 * 1024 * 1024 * 1024),
                disk_total_bytes: Some(100 * 1024 * 1024 * 1024),
                disk_available_bytes: Some(40 * 1024 * 1024 * 1024),
                disk_read_bytes_per_second: Some(u64::from(cpu) * 1024),
                disk_write_bytes_per_second: Some(u64::from(cpu) * 2048),
                network_receive_bytes_per_second: Some(u64::from(cpu) * 3072),
                network_transmit_bytes_per_second: Some(u64::from(cpu) * 4096),
                load_average_milli: Some([1_250, 2_500, 3_750]),
            }),
        );
    }
    let render_strip = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 8)).unwrap();
        terminal
            .draw(|frame| render_telemetry_strip(frame, app, frame.area()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };

    assert_eq!(
        telemetry_strip_mode(Rect::new(0, 0, 112, 8)),
        TelemetryStripMode::Wide
    );
    assert_eq!(
        telemetry_strip_mode(Rect::new(0, 0, 111, 8)),
        TelemetryStripMode::Medium
    );
    assert_eq!(
        telemetry_strip_mode(Rect::new(0, 0, 63, 8)),
        TelemetryStripMode::Hidden
    );

    let wide = render_strip(&app, 210);
    for expected in [
        "Telemetry · bounded 60-sample histories",
        "CPU Usage",
        "42%",
        "16 cores",
        "RAM Usage",
        "12.0/16.0 GiB",
        "Build FS Usage",
        "40.0 GiB free",
        "Read 42.0 KiB/s",
        "Write 84.0 KiB/s",
        "RX 126.0 KiB/s",
        "TX 168.0 KiB/s",
    ] {
        assert!(wide.contains(expected), "missing {expected}: {wide}");
    }
    assert!(wide.chars().any(|character| "▁▂▃▄▅▆▇█".contains(character)));
    assert!(
        wide.contains('▪'),
        "wide telemetry lost its filled meter: {wide}"
    );
    assert!(
        wide.contains('▫'),
        "wide telemetry lost its remaining meter track: {wide}"
    );
    assert!(
        !wide.contains('╱') && !wide.contains('╲'),
        "pointed dial geometry must not return: {wide}"
    );

    let medium = render_strip(&app, 100);
    for expected in [
        "CPU Usage",
        "42%",
        "RAM Usage",
        "75%",
        "Build FS Usage",
        "60%",
        "R 42.0 KiB/s",
        "W 84.0 KiB/s",
    ] {
        assert!(medium.contains(expected), "missing {expected}: {medium}");
    }
    assert!(!medium.contains("RX 126.0 KiB/s"), "{medium}");
    assert!(!medium.contains("TX 168.0 KiB/s"), "{medium}");

    let narrow = render_strip(&app, 63);
    assert!(!narrow.contains("Telemetry"), "{narrow}");
    assert!(!narrow.contains("CPU 42%"), "{narrow}");

    let mut unsupported_optional = app.clone();
    unsupported_optional
        .host_telemetry
        .disk_read_bytes_per_second = None;
    unsupported_optional
        .host_telemetry
        .disk_write_bytes_per_second = None;
    unsupported_optional
        .host_telemetry
        .network_receive_bytes_per_second = None;
    unsupported_optional
        .host_telemetry
        .network_transmit_bytes_per_second = None;
    unsupported_optional.host_telemetry_history = Default::default();
    let unsupported = render_strip(&unsupported_optional, 210);
    assert!(!unsupported.contains("Read"), "{unsupported}");
    assert!(!unsupported.contains("Write"), "{unsupported}");
    assert!(!unsupported.contains("RX"), "{unsupported}");
    assert!(!unsupported.contains("TX"), "{unsupported}");

    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    let high_contrast = render_strip(&app, 112);
    assert!(high_contrast.contains("CPU Usage"), "{high_contrast}");
    assert!(high_contrast.contains("42%"), "{high_contrast}");
    app.color_enabled = false;
    assert!(render_strip(&app, 100).contains("R 42.0 KiB/s"));
    app.preferences.symbols = SymbolPreference::Ascii;
    let ascii = render_strip(&app, 100);
    assert!(
        ascii.contains('#'),
        "ASCII telemetry lost its filled meter: {ascii}"
    );
    assert!(
        ascii.contains('.'),
        "ASCII telemetry lost its remaining meter track: {ascii}"
    );
    assert!(
        !ascii.contains('╱') && !ascii.contains('╲'),
        "ASCII telemetry leaked Unicode: {ascii}"
    );
    app.preferences.symbols = SymbolPreference::Unicode;

    let dashboard = rendered_text(&app, 300, 60);
    assert!(dashboard.contains("Telemetry · bounded 60-sample histories"));

    app.screen = Screen::Tasks;
    let task_rows = app.visible_task_row_refs_at(UNIX_EPOCH);
    let mut tasks = Terminal::new(TestBackend::new(140, 50)).unwrap();
    tasks
        .draw(|frame| {
            tasks_workspace(frame, &app, frame.area(), UNIX_EPOCH, &task_rows);
        })
        .unwrap();
    let tasks = tasks
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(tasks.contains("Telemetry · bounded 60-sample histories"));
    assert!(tasks.contains("Tasks:"), "{tasks}");
    assert!(tasks.contains("Log Viewer"), "{tasks}");
    assert!(tasks.contains("Job History"), "{tasks}");
}

#[test]
fn navigator_and_tasks_titles_render_once_after_resize() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Navigator;

    let mut navigator_terminal = Terminal::new(TestBackend::new(30, 55)).unwrap();
    navigator_terminal
        .draw(|frame| navigator(frame, &app, frame.area(), None))
        .unwrap();
    navigator_terminal.backend_mut().resize(28, 50);
    navigator_terminal.autoresize().unwrap();
    navigator_terminal
        .draw(|frame| navigator(frame, &app, frame.area(), None))
        .unwrap();
    let navigator_lines = navigator_terminal
        .backend()
        .buffer()
        .content
        .chunks(28)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .collect::<Vec<_>>();
    for group in [
        "▾ OVERVIEW",
        "▾ CONTENT",
        "▾ BUILD",
        "▾ VALIDATE",
        "▾ TOOLS",
    ] {
        assert_eq!(
            navigator_lines
                .iter()
                .filter(|line| line.contains(group))
                .count(),
            1,
            "group {group:?} was duplicated after resize: {navigator_lines:#?}"
        );
    }
    for destination in ["Dashboard", "Layers", "Tasks"] {
        assert_eq!(
            navigator_lines
                .iter()
                .filter(|line| line.contains(destination))
                .count(),
            1,
            "destination {destination:?} was duplicated after resize: {navigator_lines:#?}"
        );
    }

    app.focus = FocusTarget::Workspace;
    let rows = app.visible_task_row_refs_at(UNIX_EPOCH);
    let mut task_terminal = Terminal::new(TestBackend::new(110, 18)).unwrap();
    task_terminal
        .draw(|frame| render_task_table(frame, &app, frame.area(), &rows, UNIX_EPOCH))
        .unwrap();
    task_terminal.backend_mut().resize(120, 20);
    task_terminal.autoresize().unwrap();
    task_terminal
        .draw(|frame| render_task_table(frame, &app, frame.area(), &rows, UNIX_EPOCH))
        .unwrap();
    let task_lines = task_terminal
        .backend()
        .buffer()
        .content
        .chunks(120)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    assert_eq!(
        task_lines
            .iter()
            .filter(|line| line.contains("Tasks: not selected · All"))
            .count(),
        1,
        "Tasks title was duplicated after resize: {task_lines:#?}"
    );
}

#[test]
fn ux_telemetry_expanded_context_uses_exact_units_and_retained_valid_history() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    app.workspace_subfocus = yoctui_model::WorkspaceSubfocus::Context;
    app.zoomed_pane = Some(FocusTarget::Workspace);
    app.workspace.build_dir = Some("/work/构建/build".into());
    for sample in [
        yoctui_model::HostTelemetry {
            cpu_utilization_percent: Some(25),
            memory_total_bytes: Some(16 * 1024_u64.pow(3)),
            memory_available_bytes: Some(8 * 1024_u64.pow(3)),
            disk_total_bytes: Some(100 * 1024_u64.pow(3)),
            disk_available_bytes: Some(40 * 1024_u64.pow(3)),
            disk_read_bytes_per_second: Some(2_048),
            disk_write_bytes_per_second: Some(4_096),
            network_receive_bytes_per_second: Some(6_144),
            network_transmit_bytes_per_second: Some(8_192),
            ..yoctui_model::HostTelemetry::default()
        },
        yoctui_model::HostTelemetry {
            cpu_utilization_percent: Some(42),
            memory_total_bytes: Some(16 * 1024_u64.pow(3)),
            memory_available_bytes: Some(4 * 1024_u64.pow(3)),
            disk_total_bytes: Some(100 * 1024_u64.pow(3)),
            disk_available_bytes: Some(40 * 1024_u64.pow(3)),
            disk_read_bytes_per_second: None,
            disk_write_bytes_per_second: Some(0),
            network_receive_bytes_per_second: None,
            network_transmit_bytes_per_second: Some(16_384),
            ..yoctui_model::HostTelemetry::default()
        },
    ] {
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::HostTelemetryUpdated(sample));
    }

    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("ZOOM · Tasks · Workspace/Context"),
            "{output}"
        );
        assert!(output.contains("CPU 42%"), "{output}");
        assert!(output.contains("RAM 75%"), "{output}");
        assert!(output.contains("Disk read partial"), "{output}");
        assert!(!output.contains("Disk read 0 B/s"), "{output}");
        assert!(output.contains("Disk write 0 B/s"), "{output}");
        assert!(output.contains("Network RX partial"), "{output}");
        assert!(output.contains("Network TX 16384 B/s"), "{output}");
        assert!(!output.contains('\u{fffd}'), "{output}");
    }

    let wide = rendered_text(&app, 160, 50);
    assert!(wide.contains("BUILD FS  60%"), "{wide}");
    assert!(wide.chars().any(|character| "▁▂▃▄▅▆▇█".contains(character)));
}

#[test]
fn ux_telemetry_expanded_context_handles_empty_partial_large_and_no_color_inputs() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    app.workspace_subfocus = yoctui_model::WorkspaceSubfocus::Context;
    app.zoomed_pane = Some(FocusTarget::Workspace);
    app.color_enabled = false;
    app.reduced_motion = true;

    let empty = rendered_text(&app, 160, 50);
    for label in [
        "CPU unavailable",
        "RAM unavailable",
        "Disk read unavailable",
        "Disk write unavailable",
        "Network RX unavailable",
        "Network TX unavailable",
    ] {
        assert!(empty.contains(label), "missing {label}: {empty}");
    }
    assert!(!empty.contains("0 B/s"), "{empty}");

    app.host_telemetry.disk_write_bytes_per_second = Some(u64::MAX);
    app.host_telemetry_history
        .disk_write_bytes_per_second
        .extend(0..10_000);
    app.workspace.build_dir = Some("/非常に/長い/構建目录/".repeat(200).into());
    app.host_telemetry.disk_total_bytes = Some(u64::MAX);
    app.host_telemetry.disk_available_bytes = Some(1);
    for (width, height) in [(200, 60), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("18446744073709551615 B/s"), "{output}");
        assert!(!output.contains('\u{fffd}'), "{output}");
    }
}
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

#[test]
fn next_generation_job_summary_is_shared_compact_and_authoritative() {
    let mut app = App::new(10, 1_000);
    for (index, status) in [
        BackgroundJobStatus::Queued,
        BackgroundJobStatus::Running,
        BackgroundJobStatus::Failed,
        BackgroundJobStatus::Succeeded,
    ]
    .into_iter()
    .enumerate()
    {
        app.background_jobs
            .jobs
            .push_back(yoctui_model::BackgroundJob {
                id: yoctui_model::BackgroundJobId(index as u64 + 1),
                kind: BackgroundJobKind::Build,
                title: format!("job-{index}"),
                status,
                context: yoctui_model::BackgroundJobContext::default(),
                cancellation_supported: true,
                progress: yoctui_model::BackgroundJobProgress::Indeterminate,
                output: std::collections::VecDeque::new(),
                retained_output_bytes: 0,
                dropped_output_entries: 0,
                warnings: 0,
                errors: usize::from(status == BackgroundJobStatus::Failed),
                queued_at: UNIX_EPOCH,
                started_at: (status != BackgroundJobStatus::Queued)
                    .then_some(UNIX_EPOCH + Duration::from_secs(1)),
                finished_at: status
                    .is_terminal()
                    .then_some(UNIX_EPOCH + Duration::from_secs(2)),
                result: None,
                error: None,
            });
    }
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    for id in 10..12 {
        app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
            id,
            kind: yoctui_model::ClientDaemonJobKind::Utility,
            label: format!("daemon-{id}"),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        });
    }

    let wide = "Active 3 · Queued 1 · Failed 1 · Recent complete 2 · Daemon-owned 2";
    assert_eq!(job_summary_label(&app, 100), wide);
    assert_eq!(job_summary_label(&app, 70), "A3 Q1 F1 Done2 D2");

    app.screen = Screen::Tasks;
    let embedded = rendered_text_at(&app, 180, 44, UNIX_EPOCH + Duration::from_secs(10));
    assert!(embedded.contains(wide), "{embedded}");

    app.screen = Screen::BuildHistory;
    let standalone = rendered_text_at(&app, 180, 34, UNIX_EPOCH + Duration::from_secs(10));
    assert!(standalone.contains("A3 Q1 F1 Done2 D2"), "{standalone}");

    app.daemon.status = yoctui_model::ClientReplicaStatus::Stale;
    assert_eq!(job_summary_label(&app, 70), "A1 Q1 F1 Done2");
    assert!(!job_summary_label(&app, 100).contains("Daemon-owned"));
}
