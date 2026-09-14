//! Regression tests grouped around coexistence_diagnostic_distinguishes_nominal_busy_and_unknown.
use super::*;

#[test]
fn coexistence_diagnostic_distinguishes_nominal_busy_and_unknown() {
    let mut workspace = Workspace::default();
    workspace
        .variables
        .insert("PARALLEL_MAKE".into(), "-j 8 -l8".into());
    let busy = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: Some(8),
            load_average_milli: Some([7_200, 7_000, 6_000]),
            ..HostTelemetry::default()
        },
    );
    assert_eq!(busy.pressure, BitBakeCoexistencePressure::Busy);
    workspace.variables.clear();
    let nominal = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: Some(8),
            load_average_milli: Some([1_000, 1_000, 1_000]),
            ..HostTelemetry::default()
        },
    );
    assert_eq!(nominal.pressure, BitBakeCoexistencePressure::Nominal);
    let unknown = bitbake_coexistence_diagnostic(&workspace, &HostTelemetry::default());
    assert_eq!(unknown.pressure, BitBakeCoexistencePressure::Unknown);
}

#[test]
fn parallel_make_parser_rejects_unbounded_or_dynamic_job_counts() {
    assert_eq!(parse_parallel_make_jobs("-j12"), Some(12));
    assert_eq!(parse_parallel_make_jobs("--jobs 6"), Some(6));
    assert_eq!(parse_parallel_make_jobs("-j"), None);
    assert_eq!(
        parse_parallel_make_jobs("-j ${@oe.utils.cpu_count()}"),
        None
    );
}

#[test]
fn telemetry_provenance_catalog_is_complete_and_honest() {
    let metrics = TELEMETRY_PROVENANCE
        .iter()
        .map(|entry| entry.metric)
        .collect::<HashSet<_>>();
    assert_eq!(TELEMETRY_PROVENANCE.len(), 17);
    assert_eq!(metrics.len(), TELEMETRY_PROVENANCE.len());
    for required in [
        TelemetryMetric::HostCpuUtilization,
        TelemetryMetric::HostLogicalCpuCount,
        TelemetryMetric::HostMemoryCapacity,
        TelemetryMetric::BuildFilesystemCapacity,
        TelemetryMetric::DiskReadRate,
        TelemetryMetric::DiskWriteRate,
        TelemetryMetric::NetworkReceiveRate,
        TelemetryMetric::NetworkTransmitRate,
        TelemetryMetric::DaemonUptime,
        TelemetryMetric::BitBakeState,
        TelemetryMetric::ConnectedClients,
        TelemetryMetric::TerminalSessions,
        TelemetryMetric::ActiveJobs,
    ] {
        assert!(metrics.contains(&required), "missing {required:?}");
    }

    let provenance = |metric| {
        TELEMETRY_PROVENANCE
            .iter()
            .find(|entry| entry.metric == metric)
            .unwrap()
    };
    let cpu = provenance(TelemetryMetric::HostCpuUtilization);
    assert_eq!(cpu.source, TelemetrySource::HostProcStat);
    assert_eq!(cpu.sample_period_seconds, Some(1));
    assert_eq!(cpu.history_samples, HOST_TELEMETRY_HISTORY_SAMPLES);
    assert!(cpu.requires_delta && cpu.renderable);

    for metric in [
        TelemetryMetric::DiskReadRate,
        TelemetryMetric::DiskWriteRate,
    ] {
        let entry = provenance(metric);
        assert_eq!(entry.source, TelemetrySource::HostProcDiskstats);
        assert_eq!(entry.host_support, TelemetryHostSupport::Linux);
        assert_eq!(entry.sample_period_seconds, Some(1));
        assert_eq!(entry.history_samples, HOST_TELEMETRY_HISTORY_SAMPLES);
        assert!(entry.requires_delta && entry.renderable);
    }
    for metric in [
        TelemetryMetric::NetworkReceiveRate,
        TelemetryMetric::NetworkTransmitRate,
    ] {
        let entry = provenance(metric);
        assert_eq!(entry.source, TelemetrySource::HostProcNetDev);
        assert_eq!(entry.host_support, TelemetryHostSupport::Linux);
        assert_eq!(entry.sample_period_seconds, Some(1));
        assert_eq!(entry.history_samples, HOST_TELEMETRY_HISTORY_SAMPLES);
        assert!(entry.requires_delta && entry.renderable);
    }
    let queue = provenance(TelemetryMetric::DaemonQueueDepth);
    assert!(!queue.renderable);
    assert!(queue.precision.contains("client count"));
    let daemon_memory = provenance(TelemetryMetric::DaemonResidentMemory);
    assert!(!daemon_memory.renderable);
    assert!(daemon_memory.precision.contains("assumed 4096-byte"));
}
#[test]
fn settings_selection_and_changes_are_typed_and_persisted() {
    let mut app = App::new(10, 1_000);
    assert_eq!(SETTINGS[app.settings_selection], Setting::Theme);
    assert_eq!(
        update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
        Some(Effect::PersistSettings)
    );
    assert_eq!(app.theme, Theme::WhiteClassic);
    assert!(app.settings_dirty);

    let log_follow_index = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::LogFollow)
        .unwrap();
    let _ = update(
        &mut app,
        Action::SelectSetting {
            delta: log_follow_index as isize,
        },
    );
    assert_eq!(SETTINGS[app.settings_selection], Setting::LogFollow);
    assert_eq!(
        update(&mut app, Action::ChangeSelectedSetting { backwards: true }),
        Some(Effect::PersistSettings)
    );
    assert!(!app.logs.follow);
    assert_eq!(app.logs.paused_len, Some(0));

    let _ = update(&mut app, Action::SettingsPersisted);
    assert!(!app.settings_dirty);
    assert!(app.notification.is_none());
}
#[test]
fn settings_persistence_failure_retains_the_preview_and_dirty_state() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::ChangeSelectedSetting { backwards: true });
    assert_eq!(app.theme, Theme::HighContrast);

    let _ = update(
        &mut app,
        Action::SettingsPersistenceFailed("read-only filesystem".into()),
    );
    assert_eq!(app.theme, Theme::HighContrast);
    assert!(app.settings_dirty);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("read-only filesystem")
    );
    assert_eq!(
        update(&mut app, Action::RetrySettingsPersistence),
        Some(Effect::PersistSettings)
    );
}
#[test]
fn animation_ticks_advance_unless_reduced_motion_is_enabled() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Tick);
    let _ = update(&mut app, Action::Tick);
    assert_eq!(app.animation_frame, 2);

    app.reduced_motion = true;
    let _ = update(&mut app, Action::Tick);
    assert_eq!(app.animation_frame, 2);
}
#[test]
fn typed_event_actions_update_metadata_and_preserve_unknown_progress() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "zlib".into(),
                version: None,
                layer: Some("core".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "base-files".into(),
                version: None,
                layer: Some("core".into()),
                ..Recipe::default()
            },
        ]),
    );
    let _ = update(
        &mut app,
        Action::LayersLoaded(vec![Layer {
            name: "core".into(),
            path: "/poky/meta".into(),
            priority: Some(5),
        }]),
    );
    let _ = update(
        &mut app,
        Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            },
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: Some("/build/conf/local.conf:1".into()),
            operations: vec![],
            active_overrides: vec![],
        }),
    );
    let _ = update(
        &mut app,
        Action::RecipeSourcesLoaded {
            recipe: "base-files".into(),
            paths: vec!["/poky/meta/recipes-core/base-files/base-files.bb".into()],
        },
    );
    assert_eq!(app.workspace.recipes[0].name, "base-files");
    assert_eq!(app.workspace.layers[0].path, PathBuf::from("/poky/meta"));
    assert_eq!(app.workspace.variables["MACHINE"], "qemux86-64");
    assert_eq!(
        app.recipe_sources["base-files"][0],
        PathBuf::from("/poky/meta/recipes-core/base-files/base-files.bb")
    );

    let id = TaskId("base-files:do_install".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "base-files".into(),
            task: "do_install".into(),
            progress: None,
            ..TaskInfo::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id: id.clone(),
            progress: None,
        },
    );
    assert_eq!(app.tasks[&id].progress, None);
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id: id.clone(),
            progress: Some(250),
        },
    );
    assert_eq!(app.tasks[&id].progress, Some(100));
}
#[test]
fn bbmask_editing_requires_a_preview_and_confirmation() {
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("BBMASK".into(), "meta-old/.*".into());
    let _ = update(&mut app, Action::BeginBbmaskEdit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BbmaskEdit(editor))
            if editor.text == "bbmask = \"meta-old/.*\"\n"
                && editor.selected_text() == Some("meta-old/.*")
    ));
    if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut() {
        editor.text = "bbmask = \"meta-old/.* x\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewBbmaskEdit);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::BbmaskConfirmation("meta-old/.* x".into()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmBbmaskWrite),
        Some(Effect::WriteBbmask("meta-old/.* x".into()))
    );
}
#[test]
fn hardening_stress_model_retention_preserves_high_volume_invariants() {
    const EVENTS: usize = 20_000;
    let mut app = App::new(128, 4_096);
    let _ = update(&mut app, Action::ToggleLogFollow);
    for index in 0..EVENTS {
        let severity = match index % 4 {
            0 => Severity::Trace,
            1 => Severity::Info,
            2 => Severity::Warning,
            _ => Severity::Error,
        };
        let _ = update(
            &mut app,
            Action::Log(LogEntry {
                id: 0,
                severity,
                message: format!("stress-event-{index:05}-{}", "x".repeat(index % 31)),
                recipe: Some(format!("recipe-{}", index % 17)),
                task: Some(format!("do_task_{}", index % 11)),
                path: None,
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_millis(index as u64),
                build: Some(format!("build-{}", index % 3)),
                protected: false,
                diagnostic: None,
            }),
        );
        if index % 257 == 0 {
            let _ = update(&mut app, Action::ScrollLogs { delta: 19 });
        }
    }

    assert!(app.logs.entries.len() <= app.logs.max_entries);
    assert!(app.logs.retained_bytes <= app.logs.max_bytes);
    assert_eq!(
        app.logs.retained_bytes,
        app.logs
            .entries
            .iter()
            .map(|entry| entry.message.len())
            .sum::<usize>()
    );
    assert_eq!(app.logs.dropped + app.logs.entries.len(), EVENTS);
    assert_eq!(app.logs.coalesced, 0);
    assert_eq!(app.build.warnings, EVENTS / 4);
    assert_eq!(app.build.errors, EVENTS / 4);
    let visible = app.logs.filtered().count();
    assert!(visible == 0 || app.logs.selection < visible);
    assert!(app.logs.scroll_offset <= visible.saturating_sub(1));
}
#[test]
fn quitting_always_requires_confirmation() {
    let mut idle = App::new(2, 10);
    update(&mut idle, Action::Quit);
    assert!(matches!(
        idle.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
    assert!(!idle.should_quit);

    let mut a = App::new(2, 10);
    a.build.status = BuildStatus::Running;
    update(&mut a, Action::Quit);
    assert!(matches!(a.active_dialog(), Some(Dialog::QuitConfirmation)));
    assert!(!a.should_quit)
}
#[test]
fn duplicate_or_unknown_completion_does_not_increment_task_count() {
    let mut app = App::new(2, 10);
    let id = TaskId("busybox:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: None,
            ..TaskInfo::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: id.clone(),
            success: true,
        },
    );
    let _ = update(&mut app, Action::TaskCompleted { id, success: true });
    assert_eq!(app.build.completed, 1);
    assert_eq!(app.completed_tasks.len(), 1);
    assert!(app.completed_tasks.front().is_some_and(|task| task.success));
}
#[test]
fn build_task_scrolling_stays_within_observed_task_history() {
    let mut app = App::new(2, 10);
    for recipe in ["busybox", "bash"] {
        let id = TaskId(format!("{recipe}:do_compile"));
        let _ = update(
            &mut app,
            Action::TaskStarted(TaskInfo {
                id: id.clone(),
                recipe: recipe.into(),
                task: "do_compile".into(),
                progress: None,
                ..TaskInfo::default()
            }),
        );
        let _ = update(&mut app, Action::TaskCompleted { id, success: true });
    }
    let _ = update(&mut app, Action::ScrollBuildTasks { delta: 8 });
    assert_eq!(app.task_progress_scroll, 1);
    let _ = update(&mut app, Action::ScrollBuildTasks { delta: -8 });
    assert_eq!(app.task_progress_scroll, 0);
}
#[test]
fn log_filters_combine_severity_recipe_task_and_search() {
    let mut logs = LogState::new(10, 1_000);
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "Compiler warning",
    ));
    logs.insert(tagged_log(
        "bash",
        "do_install",
        Severity::Warning,
        "Install warning",
    ));
    logs.filter = Some(Severity::Warning);
    logs.recipe_filter = Some("busybox".into());
    logs.task_filter = Some("do_compile".into());
    logs.query = "compiler".into();
    assert_eq!(logs.filtered().count(), 1);
}
#[test]
fn toggles_log_view_preferences() {
    let mut app = App::new(2, 10);
    let _ = update(&mut app, Action::ToggleLogFollow);
    let _ = update(&mut app, Action::ToggleLogWrap);
    assert!(!app.logs.follow);
    assert!(app.logs.wrap);
}
#[test]
fn paused_log_view_holds_the_visible_horizon() {
    let mut app = App::new(10, 100);
    app.logs.insert(log("before pause"));
    let _ = update(&mut app, Action::ToggleLogFollow);
    app.logs.insert(log("after pause"));
    assert_eq!(app.logs.filtered().count(), 1);
    let _ = update(&mut app, Action::ToggleLogFollow);
    assert_eq!(app.logs.filtered().count(), 2);
}

#[test]
fn scrolling_logs_pauses_follow_and_bounds_offset() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Log(log("first")));
    let _ = update(&mut app, Action::Log(log("second")));
    let _ = update(&mut app, Action::ScrollLogs { delta: 9 });
    assert!(!app.logs.follow);
    assert_eq!(app.logs.scroll_offset, 1);
    assert_eq!(
        app.logs.selected().map(|entry| entry.message.as_str()),
        Some("first")
    );
    let _ = update(&mut app, Action::ScrollLogs { delta: -9 });
    assert_eq!(app.logs.scroll_offset, 0);
    assert_eq!(
        app.logs.selected().map(|entry| entry.message.as_str()),
        Some("second")
    );
}
#[test]
fn log_state_reports_bounded_vertical_and_horizontal_positions() {
    let mut logs = LogState::new(10, 1_000);
    logs.insert(log("short"));
    logs.insert(log("a much longer retained line"));
    logs.selection = usize::MAX;
    logs.horizontal_offset = usize::MAX;

    assert_eq!(logs.vertical_position(), Some((2, 2)));
    assert_eq!(logs.horizontal_position(), (26, 26));

    logs.query = "missing".into();
    assert_eq!(logs.vertical_position(), None);
    assert_eq!(logs.horizontal_position(), (0, 0));
}
#[test]
fn cycles_log_severity_filter() {
    let mut app = App::new(2, 10);
    for expected in [
        Some(Severity::Info),
        Some(Severity::Warning),
        Some(Severity::Error),
        None,
    ] {
        let _ = update(&mut app, Action::CycleLogSeverity);
        assert_eq!(app.logs.filter, expected);
    }
}
#[test]
fn log_retention_prefers_important_diagnostics_and_reports_coalescing() {
    let mut logs = LogState::new(3, 1_000);
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "warning retained",
    ));
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Error,
        "error retained",
    ));
    for index in 0..20 {
        logs.insert(log(&format!("ordinary {index}")));
    }
    assert_eq!(logs.entries.len(), 3);
    assert!(
        logs.entries
            .iter()
            .any(|entry| entry.message == "warning retained")
    );
    assert!(
        logs.entries
            .iter()
            .any(|entry| entry.message == "error retained")
    );
    assert_eq!(logs.dropped_warnings, 0);
    assert_eq!(logs.dropped_errors, 0);

    logs.insert(log("repeat"));
    logs.insert(log("repeat"));
    assert_eq!(logs.coalesced, 1);
}

#[test]
fn log_batches_preserve_critical_order_counts_and_cached_search() {
    let mut app = App::new(8, 4_096);
    app.build.target = Some("core-image-minimal".into());
    let _ = update(
        &mut app,
        Action::Logs(vec![
            log("ordinary Alpha"),
            tagged_log("busybox", "do_compile", Severity::Warning, "warning Beta"),
            tagged_log("busybox", "do_install", Severity::Error, "failure Gamma"),
        ]),
    );

    assert_eq!(app.build.warnings, 1);
    assert_eq!(app.build.errors, 1);
    assert_eq!(app.logs.entries.len(), 3);
    assert_eq!(app.logs.normalized_messages.len(), 3);
    assert!(app.logs.entries[1].protected);
    assert!(app.logs.entries[2].protected);
    assert_eq!(app.logs.entries[1].message, "warning Beta");
    assert_eq!(app.logs.entries[2].message, "failure Gamma");
    assert!(
        app.logs
            .entries
            .iter()
            .all(|entry| entry.build.as_deref() == Some("core-image-minimal"))
    );

    app.logs.query = "GAMMA".into();
    assert_eq!(app.logs.filtered().next().unwrap().message, "failure Gamma");
}

#[test]
fn log_build_filter_selection_source_and_copy_are_typed() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let mut first = tagged_log("busybox", "do_compile", Severity::Info, "compiler output");
    first.path = Some(PathBuf::from("/tmp/log.do_compile"));
    let _ = update(&mut app, Action::Log(first));
    app.build.target = Some("core-image-full-cmdline".into());
    let _ = update(&mut app, Action::Log(log("second build")));

    let _ = update(&mut app, Action::CycleLogBuildFilter);
    assert_eq!(
        app.logs.build_filter.as_deref(),
        Some("core-image-full-cmdline")
    );
    assert_eq!(app.logs.filtered().count(), 1);
    let _ = update(&mut app, Action::CycleLogBuildFilter);
    assert_eq!(app.logs.build_filter.as_deref(), Some("core-image-minimal"));
    assert_eq!(
        update(&mut app, Action::OpenSelectedLogSource),
        Some(Effect::OpenInEditor(PathBuf::from("/tmp/log.do_compile")))
    );
    let Some(Effect::CopyToClipboard(details)) = update(&mut app, Action::CopySelectedLog) else {
        panic!("selected log details were not copied through a typed effect");
    };
    assert!(details.contains("Build: core-image-minimal"));
    assert!(details.contains("compiler output"));
}
#[test]
fn ux_logs_virtualized_window_and_source_time_filters_stay_bounded() {
    let mut logs = LogState::new(12_000, 2_000_000);
    for index in 0..10_000 {
        let mut entry = log(&format!("entry-{index:05}"));
        entry.path = Some(PathBuf::from(format!("/logs/source-{}", index % 3)));
        entry.timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(index as u64);
        logs.insert(entry);
    }
    logs.follow = false;
    logs.paused_len = Some(logs.entries.len());
    logs.selection = 9_000;
    let window = logs.window(7);
    assert_eq!(window.entries.len(), 7);
    assert_eq!(window.start, 8_994);
    assert_eq!(window.total, 10_000);
    assert_eq!(window.entries.last().unwrap().message, "entry-09000");

    logs.source_filter = Some(PathBuf::from("/logs/source-0"));
    logs.time_range = LogTimeRange::LastMinute;
    let filtered = logs.filtered().collect::<Vec<_>>();
    assert!(filtered.len() <= 21);
    assert!(
        filtered
            .iter()
            .all(|entry| entry.path.as_deref() == Some(Path::new("/logs/source-0")))
    );
    assert!(filtered.iter().all(|entry| {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(9_999))
            .and_then(|latest| latest.duration_since(entry.timestamp).ok())
            .is_some_and(|age| age <= Duration::from_secs(60))
    }));
}

#[test]
fn ux_logs_bookmarks_survive_preferred_eviction_and_support_correlated_jumps() {
    let mut app = App::new(3, 1_000);
    for message in ["first", "second", "third"] {
        let _ = update(&mut app, Action::Log(log(message)));
    }
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.selection = 0;
    let first_id = app.logs.selected().unwrap().id;
    let _ = update(&mut app, Action::ToggleSelectedLogBookmark);
    assert!(app.logs.is_bookmarked(first_id));

    let _ = update(&mut app, Action::Log(log("fourth")));
    assert!(app.logs.entries.iter().any(|entry| entry.id == first_id));
    assert!(
        !app.logs
            .entries
            .iter()
            .any(|entry| entry.message == "second")
    );

    app.logs.paused_len = None;
    let third_id = app
        .logs
        .entries
        .iter()
        .find(|entry| entry.message == "third")
        .unwrap()
        .id;
    assert!(app.logs.jump_to(third_id));
    let _ = update(&mut app, Action::ToggleSelectedLogBookmark);
    let _ = update(&mut app, Action::NextLogBookmark);
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(first_id));
    let _ = update(&mut app, Action::PreviousLogBookmark);
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(third_id));

    assert!(!app.logs.jump_to(u64::MAX));
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(third_id));
}

#[test]
fn ux_logs_tasks_errors_and_job_history_open_exact_correlated_records() {
    let mut tasks = App::new(20, 4_000);
    tasks.screen = Screen::Tasks;
    let task = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    let _ = update(&mut tasks, Action::TaskStarted(task));
    let _ = update(&mut tasks, Action::Log(log("unrelated")));
    let correlated = tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "exact task warning",
    );
    let _ = update(&mut tasks, Action::Log(correlated));
    let expected = tasks.logs.entries.back().unwrap().id;
    let _ = update(&mut tasks, Action::Open(Screen::Logs));
    assert_eq!(tasks.logs.selected().map(|entry| entry.id), Some(expected));
    assert!(!tasks.logs.follow);

    tasks.error_selection = 0;
    tasks.screen = Screen::Errors;
    let _ = update(&mut tasks, Action::JumpToSelectedError);
    assert_eq!(tasks.logs.selected().map(|entry| entry.id), Some(expected));

    let mut history = App::new(20, 4_000);
    history.screen = Screen::BuildHistory;
    history.build_history.push_back(BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(1)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    });
    let mut entry = log("matching build output");
    entry.build = Some("core-image-minimal".into());
    history.logs.insert(entry);
    let expected = history.logs.entries.back().unwrap().id;
    let _ = update(&mut history, Action::Open(Screen::Logs));
    assert_eq!(
        history.logs.selected().map(|entry| entry.id),
        Some(expected)
    );
}

#[test]
fn ux_logs_copy_and_filtered_export_are_unicode_safe_and_hard_bounded() {
    let mut app = App::new(200, 2_000_000);
    for index in 0..100 {
        let _ = update(
            &mut app,
            Action::Log(log(&format!("{index:03}-{}", "構".repeat(40_000)))),
        );
    }
    let Some(Effect::CopyToClipboard(copy)) = update(&mut app, Action::CopySelectedLog) else {
        panic!("selected log copy must remain a typed effect");
    };
    assert!(copy.len() <= MAX_LOG_COPY_BYTES);
    assert!(copy.ends_with("[copy truncated at 64 KiB]"));

    let export = format_log_export(&app.logs);
    assert!(export.content.len() <= MAX_LOG_EXPORT_BYTES);
    assert!(export.truncated);
    assert!(export.included < 100);
    assert!(export.omitted > 0);
    assert!(export.content.ends_with("[export truncated at 256 KiB]"));
    let Some(Effect::CopyToClipboard(effect_export)) = update(&mut app, Action::ExportFilteredLogs)
    else {
        panic!("filtered export must use the typed clipboard effect");
    };
    assert_eq!(effect_export, export.content);
}
#[test]
fn log_terminal_diagnostics_are_protected_and_observable() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    let entry = app.logs.entries.back().unwrap();
    assert!(entry.protected);
    assert_eq!(entry.build.as_deref(), Some("core-image-minimal"));
    assert!(entry.message.contains("completed"));
}
#[test]
fn request_validation() {
    assert!(
        BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }
        .validate()
        .is_ok()
    );
    assert!(
        BuildRequest {
            targets: vec!["bad target".into()],
            task: None,
            force: false,
        }
        .validate()
        .is_err()
    );
    assert!(
        BuildRequest {
            targets: vec!["..".into()],
            task: None,
            force: false,
        }
        .validate()
        .is_err()
    );
}
