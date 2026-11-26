use super::*;
use crate::telemetry_strip::{DenseTelemetryMeter, render_dense_telemetry_meter};
use yoctui_model::{
    BuildPerformanceSettings, BuildResourceTelemetry, ClientDaemonTelemetry, ClientReplicaStatus,
    PressureSample,
};

#[test]
fn compact_dashboard_retains_quantity_and_status_when_color_is_disabled() {
    let mut app = App::new(32, 4096);
    app.color_enabled = false;
    app.workspace.build_dir = Some("/fixture/build".into());
    let render = |app: &App| {
        rendered_region_rows(78, 4, |frame, area| {
            render_compact_telemetry_strip(frame, app, area)
        })
        .join("\n")
    };
    let text = render(&app);
    assert!(text.contains("unavailable") && !text.contains("0%"));
    let host = &mut app.host_telemetry;
    host.cpu_utilization_percent = Some(0);
    host.memory_total_bytes = Some(100);
    host.memory_available_bytes = Some(100);
    host.disk_total_bytes = Some(100);
    host.disk_available_bytes = Some(100);
    let text = render(&app);
    assert!(text.contains("CPU 0%") && text.contains("RAM 0%") && text.contains("FS 0%"));
    let host = &mut app.host_telemetry;
    host.cpu_utilization_percent = Some(255);
    host.memory_available_bytes = Some(21);
    host.disk_available_bytes = Some(24);
    let text = render(&app);
    assert!(
        text.contains("CPU 100% · critical") && text.contains("FS 76% · high"),
        "{text}"
    );
    assert!(
        text.contains("RAM 79%") && !text.contains("RAM 79% · high"),
        "{text}"
    );
    app.host_telemetry.memory_available_bytes = Some(20);
    assert!(render(&app).contains("RAM 80% · high"));
    app.host_telemetry.memory_available_bytes = Some(101);
    assert!(render(&app).contains("RAM --"));
}

#[test]
fn performance_pressure_uses_observed_avg10_and_actual_configured_limits() {
    let mut app = App::new(32, 4096);
    app.daemon.status = ClientReplicaStatus::Current;
    app.preferences.build_performance = Some(BuildPerformanceSettings {
        cpu: 30_000,
        io: 30_000,
        memory: 30_000,
    });
    app.workspace
        .variables
        .insert("BB_PRESSURE_MAX_CPU".into(), "15000".into());
    app.daemon.telemetry = Some(ClientDaemonTelemetry {
        uptime_seconds: 10,
        active_jobs: 0,
        pty_sessions: 0,
        queue_depth: 0,
        pressure: Default::default(),
        memory_bytes: None,
        recovery: yoctui_model::DaemonRecoveryState::CleanStart,
        build_resources: Some(Box::new(BuildResourceTelemetry {
            cpu_pressure: Some(PressureSample {
                some_hundredths: 151,
                full_hundredths: None,
            }),
            io_pressure: Some(PressureSample {
                some_hundredths: 893,
                full_hundredths: Some(0),
            }),
            ..Default::default()
        })),
    });
    let report = app.build_performance_report();
    assert_eq!(report.pressure_warnings.len(), 1);
    let row = report.pressure_warnings[0];
    assert!(report.rows[row].contains("1.51%") && report.rows[row].contains("above 1.5% limit"));
    assert!(
        report
            .rows
            .iter()
            .any(|line| line.contains("8.93%") && line.contains("limit unavailable"))
    );
    assert!(report.rows.iter().any(|line| line == "Memory: unavailable"));
    for (width, height) in [(80, 24), (100, 30), (160, 48)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| crate::build_performance_render::workspace(frame, &app, frame.area()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let y = 4 + row as u16;
        let text: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
        assert!(text.contains("above 1.5% limit"), "{text}");
        assert_eq!(buffer[(1, y)].fg, ThemePalette::for_app(&app).warning);
    }
    let resources = app
        .daemon
        .telemetry
        .as_mut()
        .unwrap()
        .build_resources
        .as_mut()
        .unwrap();
    resources.active_profile = Some(BuildPerformanceSettings {
        cpu: 15_100,
        io: 900_000,
        memory: 15_000,
    });
    let report = app.build_performance_report();
    assert!(report.pressure_warnings.is_empty());
    assert!(
        report
            .rows
            .iter()
            .any(|line| line.contains("within 1.51% limit"))
    );
}

#[test]
fn dense_meter_graph_contains_only_existing_history() {
    let app = App::new(32, 4096);
    let draw = |history: &[u64]| {
        rendered_region_rows(32, 6, |frame, area| {
            render_dense_telemetry_meter(
                frame,
                &app,
                area,
                DenseTelemetryMeter {
                    title: "CPU Usage",
                    percent: 72,
                    detail: "4 cores",
                    active_style: Style::default(),
                    warning_percent: RESOURCE_WARNING_PERCENT,
                    history,
                },
            )
        })
    };
    let unknown = draw(&[]);
    assert!(unknown[1..4].iter().all(|line| line.trim().is_empty()));
    let sampled = draw(&[40, 50]);
    assert!(sampled[1..4].iter().any(|line| !line.trim().is_empty()));
    assert!(
        sampled[1..4]
            .iter()
            .all(|line| line.trim().chars().count() <= 2)
    );
    assert!(sampled[0].contains("72%") && sampled[0].contains("high"));
}
