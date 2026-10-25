use super::*;
#[test]
fn performance_settings_are_bounded_and_opt_in() {
    assert!(BuildPerformanceSettings::default().valid());
    assert!(
        !BuildPerformanceSettings {
            cpu: 0,
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !BuildPerformanceSettings {
            memory: 1_000_001,
            ..Default::default()
        }
        .valid()
    );
    let app = App::new(100, 4096);
    assert!(app.preferences.build_performance.is_none());
    assert!(app.build_performance_unavailable().is_some());
    assert!(
        app.build_performance_rows()
            .iter()
            .any(|s| s.contains("Client-machine"))
    );
}

fn performance_app() -> App {
    let mut app = App::new(100, 4096);
    app.screen = Screen::BuildPerformance;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = crate::ClientReplicaStatus::Current;
    app.build_environment =
        crate::BuildEnvironmentState::Connected(crate::BuildEnvironmentProfile {
            source_dir: "/source".into(),
            build_dir: "/build".into(),
            init_script: "/source/oe-init-build-env".into(),
        });
    app.workspace.build_dir = Some("/build".into());
    app.workspace.bitbake_version = Some("2.20.0".into());
    let pressure = Some(PressureSample {
        some_hundredths: 0,
        full_hundredths: Some(0),
    });
    app.daemon.telemetry = Some(crate::ClientDaemonTelemetry {
        uptime_seconds: 10,
        active_jobs: 0,
        pty_sessions: 0,
        queue_depth: 0,
        pressure: Default::default(),
        memory_bytes: None,
        recovery: crate::DaemonRecoveryState::CleanStart,
        build_resources: Some(Box::new(BuildResourceTelemetry {
            sampled_unix_ms: 100,
            cpu_pressure: pressure,
            io_pressure: pressure,
            memory_pressure: pressure,
            ..Default::default()
        })),
    });
    app
}
#[test]
fn selecting_balanced_never_starts_a_build_or_changes_running_work() {
    let mut app = performance_app();
    let build = app.build.clone();
    let variables = app.workspace.variables.clone();
    assert_eq!(
        reduce(&mut app, BuildPerformanceAction::Balanced),
        Some(Effect::PersistSettings)
    );
    assert_eq!(
        app.preferences.build_performance,
        Some(BuildPerformanceSettings::default())
    );
    assert_eq!(app.build, build);
    assert_eq!(app.workspace.variables, variables);
    assert!(app.settings_dirty);
    reduce(&mut app, BuildPerformanceAction::Adjust(i32::MAX));
    assert_eq!(app.preferences.build_performance.unwrap().cpu, 1_000_000);
    reduce(&mut app, BuildPerformanceAction::Adjust(i32::MIN));
    assert_eq!(app.preferences.build_performance.unwrap().cpu, 1);
    reduce(&mut app, BuildPerformanceAction::Defaults);
    assert!(app.preferences.build_performance.is_none());
}
#[test]
fn unavailable_and_modal_performance_controls_are_inert() {
    let mut app = performance_app();
    app.daemon.status = crate::ClientReplicaStatus::Stale;
    reduce(&mut app, BuildPerformanceAction::Balanced);
    assert!(app.preferences.build_performance.is_none());
    app.daemon.status = crate::ClientReplicaStatus::Current;
    app.focus = FocusTarget::Navigator;
    reduce(&mut app, BuildPerformanceAction::Balanced);
    assert!(app.preferences.build_performance.is_none());
}
#[test]
fn report_and_scroll_preserve_truth_and_hide_cache_credentials() {
    let mut app = performance_app();
    app.workspace.variables.insert(
        "SSTATE_MIRRORS".into(),
        "file://.* https://user:secret@host/PATH".into(),
    );
    app.build_performance.visible_rows = 5;
    reduce(&mut app, BuildPerformanceAction::Last);
    let last = app.build_performance.scroll;
    assert_eq!(last, app.build_performance_rows().len() - 5);
    reduce(&mut app, BuildPerformanceAction::Scroll(-1));
    assert_eq!(app.build_performance.scroll, last - 1);
    reduce(&mut app, BuildPerformanceAction::First);
    assert_eq!(app.build_performance.scroll, 0);
    let text = app.build_performance_rows().join("\n");
    assert!(!text.contains("secret"));
    assert!(text.contains("some 0.00%"));
    assert!(text.contains("No task memory observations"));
}
#[test]
fn stored_profile_validation_and_legacy_preferences_are_safe() {
    let old: crate::WorkbenchPreferences = toml::from_str("schema_version = 1").unwrap();
    assert!(old.build_performance.is_none());
    let mut bad = old;
    bad.build_performance = Some(BuildPerformanceSettings {
        memory: 0,
        ..Default::default()
    });
    assert!(bad.validate().is_err());
    let mut sample = BuildResourceTelemetry {
        sampled_unix_ms: 1,
        ..Default::default()
    };
    assert!(sample.valid());
    sample.memory_pressure = Some(PressureSample {
        some_hundredths: 10001,
        full_hundredths: None,
    });
    assert!(!sample.valid());
    sample.memory_pressure = None;
    sample.recipe_peaks = vec![RecipeMemoryPeak {
        recipe: "bad\nname".into(),
        task: "do_compile".into(),
        sampled_peak_rss_bytes: 10,
    }];
    assert!(!sample.valid());
}

#[test]
fn balanced_rejects_old_and_unreported_bitbake_versions() {
    let mut app = performance_app();
    for version in [None, Some("2.0.0".into()), Some("unknown".into())] {
        app.workspace.bitbake_version = version;
        reduce(&mut app, BuildPerformanceAction::Balanced);
        assert!(app.preferences.build_performance.is_none());
        assert!(app.notification.as_deref().unwrap().contains("2.2"));
    }
}

#[test]
fn selecting_a_threshold_reveals_it_in_a_small_viewport() {
    let mut app = performance_app();
    app.build_performance.visible_rows = 5;
    reduce(&mut app, BuildPerformanceAction::Field(2));
    let rows = app.build_performance_rows();
    assert!(
        rows.iter()
            .skip(app.build_performance.scroll)
            .take(5)
            .any(|r| r.starts_with("> 3 BB_PRESSURE_MAX_MEMORY"))
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("defaults below are inactive"))
    );
}
