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
    assert!(dashboard.contains("Resource Telemetry"));
    assert!(dashboard.contains("Downloads:"));
    assert!(dashboard.contains("offline readiness: unverified"));

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
