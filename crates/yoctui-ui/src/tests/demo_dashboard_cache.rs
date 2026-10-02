use super::*;

fn cache_app() -> App {
    let mut app = App::new(32, 8192);
    app.require_daemon = true;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.host_telemetry = yoctui_model::HostTelemetry {
        cpu_utilization_percent: Some(42),
        logical_cpu_count: Some(16),
        memory_total_bytes: Some(16 * 1024 * 1024 * 1024),
        memory_available_bytes: Some(4 * 1024 * 1024 * 1024),
        disk_total_bytes: Some(100 * 1024 * 1024 * 1024),
        disk_available_bytes: Some(40 * 1024 * 1024 * 1024),
        ..Default::default()
    };
    app
}

fn cache_strip(app: &App, width: u16, height: u16, dashboard: bool) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            if dashboard {
                render_dashboard_telemetry(frame, app, frame.area());
            } else {
                render_telemetry_strip(frame, app, frame.area());
            }
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn demo_dashboard_cache_retains_resources_and_truthful_policy_observations() {
    let mut app = cache_app();
    app.build.cache.summary = Some(yoctui_model::SstateSummary {
        wanted: 10,
        local: 4,
        mirrors: 2,
        missed: 4,
        current: 3,
    });
    app.build.cache.fetch_completed = 7;
    app.build.cache.fetch_failed = 1;
    let _ = update(
        &mut app,
        Action::TaskStarted(yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("busybox:do_fetch".into()),
            "busybox".into(),
            "do_fetch".into(),
        )),
    );
    for (no_network, premirrors, policy) in [
        ("1", "0", "disabled"),
        ("0", "1", "premirrors only"),
        ("0", "0", "allowed"),
        ("invalid", "0", "unknown"),
    ] {
        app.workspace
            .variables
            .insert("BB_NO_NETWORK".into(), no_network.into());
        app.workspace
            .variables
            .insert("BB_FETCH_PREMIRRORONLY".into(), premirrors.into());
        for width in [80, 89, 100, 160, 240] {
            let output = cache_strip(&app, width, 8, true);
            for expected in [
                "42%",
                "16 cores",
                "75%",
                "12.0/16.0 GiB",
                "60%",
                "40.0 GiB free",
                "Sstate: 4 local + 2 mirrors / 10 wanted; 3 current",
                "Downloads: 7 completed, 1 failed, 1 active",
                &format!("Network: {policy}; offline readiness: unverified"),
            ] {
                assert!(
                    output.contains(expected),
                    "{width} lost {expected}: {output}"
                );
            }
            assert!(output.contains('▪') && output.contains('▫'), "{output}");
            assert!(!output.contains("SSTATE ! unavailable"), "{output}");
        }
    }
    let output = rendered_text(&app, 300, 60);
    assert!(
        output.contains("Downloads: 7 completed, 1 failed, 1 active"),
        "{output}"
    );
    assert!(output.contains("offline readiness: unverified"), "{output}");
    assert_eq!(app.build.cache.fetch_completed, 7);
    assert_eq!(app.screen, Screen::Dashboard);
}

#[test]
fn demo_dashboard_cache_marks_missing_invalid_offline_and_small_fallbacks() {
    let mut app = cache_app();
    for summary in [
        None,
        Some(yoctui_model::SstateSummary {
            wanted: 1,
            local: 2,
            mirrors: 0,
            missed: 0,
            current: 0,
        }),
        Some(yoctui_model::SstateSummary {
            wanted: u64::MAX,
            local: u64::MAX,
            mirrors: 1,
            missed: 0,
            current: 0,
        }),
    ] {
        app.build.cache.summary = summary;
        let output = cache_strip(&app, 89, 8, true);
        assert!(output.contains("Sstate: summary not reported"), "{output}");
        assert!(
            output.contains("Network: unknown; offline readiness: unverified"),
            "{output}"
        );
        let standalone = cache_strip(&app, 86, 8, false);
        assert!(standalone.contains("Sstate:"), "{standalone}");
        assert!(!standalone.contains("SSTATE ! unavailable"), "{standalone}");
    }
    app.build.cache.summary = Some(yoctui_model::SstateSummary {
        wanted: 10,
        local: 4,
        mirrors: 2,
        missed: 4,
        current: 3,
    });
    let standalone = cache_strip(&app, 86, 8, false);
    for expected in ["Sstate: 4 local + 2", "mirrors / 10 wanted;", "3 current"] {
        assert!(standalone.contains(expected), "{standalone}");
    }
    app.daemon.status = yoctui_model::ClientReplicaStatus::Disconnected;
    let output = cache_strip(&app, 89, 8, true);
    if app.is_offline() {
        assert!(output.contains("cache last observed"), "{output}");
        assert!(output.contains("0 active (last observed)"), "{output}");
    } else {
        panic!("fixture must explicitly represent disconnected observations");
    }
    app.preferences.symbols = SymbolPreference::Ascii;
    app.color_enabled = false;
    let output = cache_strip(&app, 89, 8, true);
    assert!(output.contains('#') && output.contains('.'), "{output}");
    for width in [0, 1, 12, 63, 64, 80] {
        for height in [0, 1, 3, 4, 7, 8] {
            let output = cache_strip(&app, width, height, true);
            if width < 64 || height < 4 {
                assert!(!output.contains("Downloads:"), "{width}x{height}: {output}");
            }
        }
    }
}
