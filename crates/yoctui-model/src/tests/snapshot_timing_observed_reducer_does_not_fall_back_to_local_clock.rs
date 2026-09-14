//! Regression tests grouped around snapshot_timing_observed_reducer_does_not_fall_back_to_local_clock.
use super::*;

#[test]
fn snapshot_timing_observed_reducer_does_not_fall_back_to_local_clock() {
    use super::*;
    let mut app = App::new(64, 64 * 1024);
    let task = TaskInfo {
        id: TaskId("llvm-native:do_compile".into()),
        recipe: "llvm-native".into(),
        task: "do_compile".into(),
        ..TaskInfo::default()
    };
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedStarted(task.clone())]),
    );
    assert_eq!(app.tasks[&task.id].started, None);
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedCompleted {
            id: task.id.clone(),
            success: true,
            timing: ObservedTaskTiming {
                started: Some(SystemTime::UNIX_EPOCH),
                finished: None,
            },
        }]),
    );
    assert_eq!(
        app.completed_tasks[0].task.elapsed_at(SystemTime::now()),
        None
    );
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedCompleted {
            id: task.id,
            success: true,
            timing: ObservedTaskTiming {
                started: Some(SystemTime::UNIX_EPOCH),
                finished: Some(SystemTime::now()),
            },
        }]),
    );
    assert_eq!(app.completed_tasks.len(), 1);
    assert_eq!(app.completed_tasks[0].task.finished, None);
}

#[test]
fn render_cache_viewport_recomputes_without_stale_selection_state() {
    assert_eq!(centered_viewport_range(None, 100, 10), 0..10);
    assert_eq!(centered_viewport_range(Some(50), 100, 10), 45..55);
    assert_eq!(centered_viewport_range(Some(99), 100, 10), 90..100);
    assert_eq!(
        centered_viewport_range(Some(99), 3, 10),
        0..3,
        "query/inventory shrink clamps a formerly valid selection immediately"
    );
    assert_eq!(centered_viewport_range(Some(1), 0, 10), 0..0);
    assert_eq!(centered_viewport_range(Some(1), 10, 0), 0..0);
}

#[test]
fn function_shortcut_catalog_is_complete_unique_and_truthful() {
    assert_eq!(FUNCTION_SHORTCUTS.len(), 10);
    let keys = FUNCTION_SHORTCUTS
        .iter()
        .map(|shortcut| shortcut.key)
        .collect::<HashSet<_>>();
    let labels = FUNCTION_SHORTCUTS
        .iter()
        .map(|shortcut| shortcut.key_label)
        .collect::<HashSet<_>>();
    assert_eq!(keys.len(), FUNCTION_SHORTCUTS.len());
    assert_eq!(labels.len(), FUNCTION_SHORTCUTS.len());
    for shortcut in FUNCTION_SHORTCUTS {
        assert!(!shortcut.action_label.is_empty());
        assert_eq!(
            function_shortcut_action(shortcut.key),
            match shortcut.route {
                FunctionShortcutRoute::Open(screen) => Action::Open(screen),
                FunctionShortcutRoute::CommandPalette => Action::OpenCommandPalette,
                FunctionShortcutRoute::ApplicationMenu => Action::OpenApplicationMenu,
            }
        );
    }
}
#[test]
fn notification_transient_status_uses_typed_priority_and_dismisses() {
    let mut app = App::new(10, 1_000);
    assert_eq!(app.transient_status(), None);

    let _ = update(&mut app, Action::Notify("  Saved profile  ".into()));
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Notification,
            text: "Saved profile".into(),
        })
    );

    app.dialogs.push_front(Dialog::QuitConfirmation);
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Confirmation,
            text: "Confirmation pending".into(),
        })
    );
    app.build.status = BuildStatus::Failed;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Confirmation,
            text: "Confirmation pending".into(),
        })
    );
    app.dialogs.clear();
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Notification)
    );
    let _ = update(
        &mut app,
        Action::Failure(AppError::new("backend", "connection lost", "retry")),
    );
    app.dialogs.push_front(Dialog::QuitConfirmation);
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Error)
    );
    let _ = update(&mut app, Action::DismissNotification);
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Confirmation)
    );
    app.dialogs.clear();
    assert_eq!(app.transient_status(), None);

    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Confirmation)
    );
    app.dialogs.clear();
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Success)
    );
    let _ = update(&mut app, Action::BuildCancelled { exit_code: None });
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Confirmation)
    );
    app.dialogs.clear();
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Warning)
    );
    app.notification = Some("   ".into());
    app.build.status = BuildStatus::Idle;
    assert_eq!(app.transient_status(), None);

    app.notification = None;
    app.daemon.status = ClientReplicaStatus::Synchronizing;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Reconnecting,
            text: "Daemon synchronizing".into(),
        })
    );
    app.daemon.status = ClientReplicaStatus::Stale;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Warning,
            text: "Daemon state stale".into(),
        })
    );
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.bitbake = ClientDaemonLifecycle::Connecting;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Reconnecting,
            text: "BitBake connecting".into(),
        })
    );

    app.daemon.bitbake = ClientDaemonLifecycle::Running;
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(7, true)),
    );
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "0 active jobs · 1 queued".into(),
        })
    );
    app.background_jobs.jobs.clear();
    app.build.status = BuildStatus::Running;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "Build running · 0 active".into(),
        })
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(8, true)),
    );
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "Build running · 0 active · 1 queued".into(),
        })
    );
}
#[test]
fn background_job_completes_and_survives_workspace_navigation() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    let _ = update(&mut app, Action::Open(Screen::Layers));
    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::UpdateBackgroundJobProgress {
            id,
            progress: BackgroundJobProgress::Units {
                completed: 4,
                total: 10,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id,
            entry: BackgroundJobOutputEntry {
                severity: Severity::Warning,
                message: "cache miss".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            },
        },
    );
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "image built".into(),
                artifacts: vec!["/deploy/core-image-minimal.wic".into()],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        },
    );
    let _ = update(&mut app, Action::Open(Screen::Settings));

    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(
        job.progress,
        BackgroundJobProgress::Units {
            completed: 4,
            total: 10
        }
    );
    assert_eq!(job.warnings, 1);
    assert_eq!(
        job.started_at,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
    );
    assert_eq!(
        job.finished_at,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(3))
    );
    assert_eq!(
        job.result.as_ref().map(|result| result.summary.as_str()),
        Some("image built")
    );
}
#[test]
fn background_job_records_failure_and_loss() {
    let mut app = App::new(10, 1_000);
    for id in [1, 2] {
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(id, true)),
        );
        run_background_job(&mut app, id);
    }
    let _ = update(
        &mut app,
        Action::FailBackgroundJob {
            id: BackgroundJobId(1),
            error: BackgroundJobError {
                summary: "BitBake failed".into(),
                detail: Some("exit code 1".into()),
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
        },
    );
    let _ = update(
        &mut app,
        Action::LoseBackgroundJob {
            id: BackgroundJobId(2),
            error: BackgroundJobError {
                summary: "bridge disconnected".into(),
                detail: None,
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
        },
    );

    assert_eq!(
        app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
        BackgroundJobStatus::Failed
    );
    let lost = app.background_jobs.get(BackgroundJobId(2)).unwrap();
    assert_eq!(lost.status, BackgroundJobStatus::Lost);
    assert_eq!(
        lost.error.as_ref().map(|error| error.summary.as_str()),
        Some("bridge disconnected")
    );
}
#[test]
fn background_job_history_pins_active_rows_and_bounds_shared_selection() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: BackgroundJobId(1),
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: Vec::new(),
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
        },
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(2, true)),
    );
    app.build_history.push_back(BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(9)),
        completed_tasks: 4,
        warnings: 0,
        errors: 0,
    });

    let rows = app.job_history_rows();
    assert!(matches!(
        rows.as_slice(),
        [
            JobHistoryRowRef::Background(active),
            JobHistoryRowRef::Background(terminal),
            JobHistoryRowRef::Build(_)
        ] if active.id == BackgroundJobId(2)
            && active.status == BackgroundJobStatus::Queued
            && terminal.id == BackgroundJobId(1)
            && terminal.status == BackgroundJobStatus::Succeeded
    ));
    let _ = update(&mut app, Action::SelectBuildHistory { delta: 99 });
    assert_eq!(app.build_history_selection, 2);
}

#[test]
fn background_job_summary_counts_exact_states_and_current_daemon_ownership() {
    let mut app = App::new(10, 1_000);
    for id in 1..=4 {
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(id, true)),
        );
    }
    run_background_job(&mut app, 2);
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id: BackgroundJobId(3),
            started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        },
    );
    let _ = update(
        &mut app,
        Action::RunBackgroundJob {
            id: BackgroundJobId(3),
        },
    );
    let _ = update(
        &mut app,
        Action::FailBackgroundJob {
            id: BackgroundJobId(3),
            error: BackgroundJobError {
                summary: "failed".into(),
                detail: None,
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
        },
    );
    run_background_job(&mut app, 4);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: BackgroundJobId(4),
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: Vec::new(),
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
        },
    );
    app.build_history.push_back(BuildRecord {
        target: Some("already represented".into()),
        success: false,
        exit_code: Some(1),
        elapsed: Some(Duration::from_secs(5)),
        completed_tasks: 1,
        warnings: 0,
        errors: 1,
    });
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.jobs.push(ClientDaemonJobSummary {
        id: 90,
        kind: ClientDaemonJobKind::Utility,
        label: "daemon job".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });

    assert_eq!(
        app.job_summary(),
        JobSummary {
            active: 2,
            queued: 1,
            failed: 1,
            recent_completed: 2,
            daemon_owned: Some(1),
        }
    );
    app.daemon.status = ClientReplicaStatus::Stale;
    assert_eq!(app.job_summary().daemon_owned, None);
}

#[test]
fn job_history_deduplicates_only_matching_terminal_daemon_builds() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.jobs.push(ClientDaemonJobSummary {
        id: 1,
        kind: ClientDaemonJobKind::BitBakeBuild,
        label: "BitBake build core-image-minimal".into(),
        lifecycle: ClientDaemonLifecycle::Exited,
        progress_current: Some(100),
        progress_total: Some(100),
        exit_code: Some(0),
    });
    for target in ["older-image", "core-image-minimal"] {
        app.build_history.push_back(BuildRecord {
            target: Some(target.into()),
            success: true,
            exit_code: Some(0),
            elapsed: None,
            completed_tasks: 1,
            warnings: 0,
            errors: 0,
        });
    }

    let rows = app.job_history_rows();
    assert_eq!(rows.len(), 2);
    assert!(matches!(rows[0], JobHistoryRowRef::Daemon(job) if job.id == 1));
    assert!(matches!(
        rows[1],
        JobHistoryRowRef::Build(record)
            if record.target.as_deref() == Some("older-image")
    ));
}

#[test]
fn inspector_mode_tracks_typed_screen_selection_and_navigator_focus() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Workspace;
    for (screen, mode) in [
        (Screen::Dashboard, InspectorMode::DaemonSession),
        (Screen::Tasks, InspectorMode::Task),
        (Screen::BuildHistory, InspectorMode::Job),
        (Screen::Dependencies, InspectorMode::Dependency),
        (Screen::Recipes, InspectorMode::Recipe),
        (Screen::Packages, InspectorMode::Package),
        (Screen::Images, InspectorMode::Artifact),
        (Screen::Testing, InspectorMode::Test),
        (Screen::Maintenance, InspectorMode::Utility),
        (Screen::Errors, InspectorMode::Error),
        (
            Screen::Compatibility,
            InspectorMode::CompatibilityCapability,
        ),
    ] {
        app.screen = screen;
        assert_eq!(app.inspector_mode(), mode);
    }

    app.screen = Screen::Layers;
    assert_eq!(app.inspector_mode(), InspectorMode::Layer);
    let mut browser = LayerBrowser::new("meta-test".into(), PathBuf::from("/layers/meta-test"));
    browser.entries.push(LayerBrowserEntry {
        path: PathBuf::from("recipes-test/test.bb"),
        ..LayerBrowserEntry::default()
    });
    app.layer_browser = Some(browser);
    assert_eq!(app.inspector_mode(), InspectorMode::File);

    app.focus = FocusTarget::Navigator;
    assert_eq!(app.inspector_mode(), InspectorMode::Navigator);
    assert_eq!(
        InspectorMode::CompatibilityCapability.label(),
        "Compatibility capability"
    );
}

#[test]
fn task_inspector_projects_recipe_dependencies_and_a_bounded_correlated_tail() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        version: Some("1.36.1".into()),
        ..Recipe::default()
    });
    let mut task = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.dependencies = vec![
        TaskId("busybox:do_configure".into()),
        TaskId("virtual/libc:do_populate_sysroot".into()),
    ];
    task.started = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));
    app.tasks.insert(task.id.clone(), task);
    for (id, recipe, task, message) in [
        (1, "busybox", "do_compile", "old"),
        (2, "bash", "do_compile", "unrelated"),
        (3, "busybox", "do_compile", "middle"),
        (4, "busybox", "do_compile", "new"),
    ] {
        app.logs.insert(LogEntry {
            id,
            severity: Severity::Info,
            message: message.into(),
            recipe: Some(recipe.into()),
            task: Some(task.into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(id),
            build: None,
            protected: false,
            diagnostic: None,
        });
    }
    let rows = app.visible_task_row_refs_at(SystemTime::UNIX_EPOCH + Duration::from_secs(20));
    let inspector = app.task_inspector(rows.first().copied(), 2);
    let TaskInspectorRef::Task {
        task,
        state,
        version,
        revision,
        workdir,
        recent_logs,
    } = inspector
    else {
        panic!("expected selected task inspector")
    };
    assert_eq!(task.recipe, "busybox");
    assert_eq!(state, TaskState::Active);
    assert_eq!(version, Some("1.36.1"));
    assert_eq!(revision, None);
    assert_eq!(workdir, None);
    assert_eq!(task.dependencies.len(), 2);
    assert_eq!(
        recent_logs
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<Vec<_>>(),
        ["middle", "new"]
    );
    assert_eq!(
        app.task_inspector(Some(TaskRowRef::WaitingSummary(7)), 2),
        TaskInspectorRef::Waiting { count: 7 }
    );
}
#[test]
fn background_job_cancellation_requires_capability_and_acknowledgement() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    let _ = update(
        &mut app,
        Action::RequestBackgroundJobCancellation {
            id: BackgroundJobId(1),
        },
    );
    assert_eq!(
        app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
        BackgroundJobStatus::Cancelling
    );
    let _ = update(
        &mut app,
        Action::CancelBackgroundJob {
            id: BackgroundJobId(1),
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        },
    );
    assert_eq!(
        app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
        BackgroundJobStatus::Cancelled
    );

    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(2, false)),
    );
    run_background_job(&mut app, 2);
    let ignored_before = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::RequestBackgroundJobCancellation {
            id: BackgroundJobId(2),
        },
    );
    assert_eq!(
        app.background_jobs.get(BackgroundJobId(2)).unwrap().status,
        BackgroundJobStatus::Running
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored_before + 1);
}
#[test]
fn background_job_rejected_cancellation_returns_to_running() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    run_background_job(&mut app, 1);
    let _ = update(&mut app, Action::RequestBackgroundJobCancellation { id });
    let _ = update(&mut app, Action::RejectBackgroundJobCancellation { id });
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Running
    );
}
#[test]
fn background_job_invalid_transitions_leave_state_unchanged() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::UpdateBackgroundJobProgress {
            id,
            progress: BackgroundJobProgress::Percent(101),
        },
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Queued
    );
    assert_eq!(app.background_jobs.ignored_transitions, 2);

    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        },
    );
    let _ = update(
        &mut app,
        Action::FailBackgroundJob {
            id,
            error: BackgroundJobError {
                summary: "late failure".into(),
                detail: None,
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        },
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Succeeded
    );
    assert_eq!(app.background_jobs.ignored_transitions, 3);
}
#[test]
fn background_job_history_and_output_retention_are_bounded_and_observable() {
    let mut app = App::new(10, 1_000);
    app.background_jobs = BackgroundJobs::new(2, 2, 4);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: BackgroundJobId(1),
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        },
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(2, true)),
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(3, true)),
    );
    assert_eq!(app.background_jobs.jobs.len(), 2);
    assert_eq!(app.background_jobs.dropped_jobs, 1);
    assert!(app.background_jobs.get(BackgroundJobId(1)).is_none());

    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id: BackgroundJobId(2),
            entry: BackgroundJobOutputEntry {
                severity: Severity::Warning,
                message: "abc".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id: BackgroundJobId(2),
            entry: BackgroundJobOutputEntry {
                severity: Severity::Error,
                message: "de".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let retained = app.background_jobs.get(BackgroundJobId(2)).unwrap();
    assert_eq!(retained.output.len(), 1);
    assert_eq!(retained.retained_output_bytes, 2);
    assert_eq!(retained.dropped_output_entries, 1);
    assert_eq!(retained.warnings, 1);
    assert_eq!(retained.errors, 1);

    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(4, true)),
    );
    assert_eq!(app.background_jobs.jobs.len(), 2);
    assert_eq!(app.background_jobs.rejected_jobs, 1);
}
#[test]
fn bounded_logs_report_eviction() {
    let mut l = LogState::new(2, 100);
    l.insert(log("a"));
    l.insert(log("b"));
    l.insert(log("c"));
    assert_eq!(l.entries.len(), 2);
    assert_eq!(l.dropped, 1)
}
#[test]
fn navigator_selection_and_focus_cycle_are_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Focus(FocusTarget::Navigator));
    let _ = update(&mut app, Action::SelectNavigator { delta: 100 });
    assert_eq!(app.navigator_selection, NAVIGATOR_SCREENS.len() - 1);
    let _ = update(&mut app, Action::ActivateNavigator);
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.focus, FocusTarget::Workspace);
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Inspector);
    let _ = update(&mut app, Action::CycleFocus { backwards: true });
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn navigator_screen_projects_the_bounded_selection() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = 2;
    assert_eq!(app.navigator_screen(), Screen::Layers);
    app.navigator_selection = usize::MAX;
    assert_eq!(app.navigator_screen(), Screen::Dashboard);
}
