use super::*;

pub(crate) fn literal_reference_app() -> App {
    let mut app = App::new(512, 1024 * 1024);
    let source_dir = PathBuf::from("/workspace/yocto");
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 2;
    app.backend = "bridge".into();
    app.workspace.build_dir = Some(source_dir.join("build"));
    app.workspace.source_dir = Some(source_dir.clone());
    app.workspace.release = Some("scarthgap".into());
    app.workspace.bitbake_version = Some("2.8.0".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.workspace.layers = [
        "poky",
        "meta",
        "meta-poky",
        "meta-yocto-bsp",
        "meta-oe",
        "meta-python",
        "meta-networking",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, name)| yoctui_model::Layer {
        name: name.into(),
        path: source_dir.join(name),
        priority: Some(index as i32 + 5),
    })
    .collect();
    app.workspace.recipes = ["busybox", "bash", "core-image-minimal"]
        .into_iter()
        .map(|name| yoctui_model::Recipe {
            name: name.into(),
            version: (name == "bash").then(|| "5.2.21".into()),
            layer: Some("poky".into()),
            ..Default::default()
        })
        .collect();
    app.available_images = vec![
        "core-image-minimal".into(),
        "core-image-full-cmdline".into(),
    ];
    app.build.status = BuildStatus::Running;
    app.build.target = Some("core-image-minimal".into());
    app.build.started = Some(literal_now() - Duration::from_secs(978));
    app.build.completed = 4;
    app.build.total = Some(10);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.daemon.connected_clients = 1;
    app.daemon.telemetry = Some(yoctui_model::ClientDaemonTelemetry {
        uptime_seconds: 8_100,
        active_jobs: 1,
        pty_sessions: 1,
        queue_depth: 0,
        pressure: yoctui_model::ClientDaemonPressureCounters::default(),
        memory_bytes: Some(32 * 1024 * 1024),
        recovery: yoctui_model::DaemonRecoveryState::Recovered,
    });
    app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
        id: 858,
        kind: yoctui_model::ClientDaemonJobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
        progress_current: Some(4),
        progress_total: Some(10),
        exit_code: None,
    });
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "terminal".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.host_telemetry.cpu_utilization_percent = Some(24);
    app.host_telemetry.disk_available_bytes = Some(23 * 1024 * 1024 * 1024);

    for (index, task_name) in ["do_fetch", "do_unpack", "do_patch", "do_configure"]
        .into_iter()
        .enumerate()
    {
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId(format!("bash:{task_name}")),
                recipe: "bash_5.2.21-2".into(),
                task: task_name.into(),
                progress: Some(100),
                state: yoctui_model::TaskState::Completed,
                started: Some(literal_now() - Duration::from_secs(600 - index as u64)),
                finished: Some(literal_now() - Duration::from_secs(599 - index as u64)),
                ..Default::default()
            },
            success: true,
        });
    }
    let active = yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("bash:do_compile".into()),
            recipe: "bash_5.2.21-2".into(),
            task: "do_compile".into(),
            progress: Some(72),
            state: yoctui_model::TaskState::Active,
            worker: Some("worker-1".into()),
            pid: Some(35_421),
            started: Some(literal_now() - Duration::from_secs(590)),
            log_path: Some("/workspace/yocto/build/tmp/work/qemux86-64-poky-linux/bash/5.2.21-r2/temp/log.do_compile.85873".into()),
            ..Default::default()
        };
    app.tasks.insert(active.id.clone(), active);
    app.task_progress_scroll = 0;
    for message in [
        "NOTE: Executing Tasks",
        "NOTE: Started: do_compile",
        "|  CC     builtins/histfile.o",
        "|  CC     builtins/jobs.o",
        "|  CC     execute_cmd.o",
        "[ 72%] Linking bash",
    ] {
        let _ = update(
            &mut app,
            Action::Log(yoctui_model::LogEntry {
                id: 0,
                severity: yoctui_model::Severity::Info,
                message: message.into(),
                recipe: Some("bash_5.2.21-2".into()),
                task: Some("do_compile".into()),
                path: None,
                timestamp: literal_now(),
                build: Some("core-image-minimal".into()),
                protected: false,
                diagnostic: None,
            }),
        );
    }
    for (id, title, started_ago, finished_ago, succeeded) in [
        (854, "virtual/kernel", 1_269, 1_215, true),
        (855, "core-image-minimal", 1_194, 1_086, false),
        (856, "busybox", 1_005, 988, true),
        (857, "core-image-minimal", 977, 682, true),
    ] {
        let job_id = yoctui_model::BackgroundJobId(id);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id: job_id,
                kind: yoctui_model::BackgroundJobKind::Build,
                title: title.into(),
                context: yoctui_model::BackgroundJobContext {
                    target: Some(title.into()),
                    ..Default::default()
                },
                cancellation_supported: true,
                queued_at: literal_now() - Duration::from_secs(started_ago + 1),
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id: job_id,
                started_at: literal_now() - Duration::from_secs(started_ago),
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id: job_id });
        let finished_at = literal_now() - Duration::from_secs(finished_ago);
        let _ = if succeeded {
            update(
                &mut app,
                Action::SucceedBackgroundJob {
                    id: job_id,
                    result: yoctui_model::BackgroundJobResult {
                        summary: "completed".into(),
                        artifacts: Vec::new(),
                    },
                    finished_at,
                },
            )
        } else {
            update(
                &mut app,
                Action::FailBackgroundJob {
                    id: job_id,
                    error: yoctui_model::BackgroundJobError {
                        summary: "build failed".into(),
                        detail: None,
                    },
                    finished_at,
                },
            )
        };
    }
    app
}

pub(crate) fn concept_idle_dashboard_app() -> App {
    let mut app = literal_reference_app();
    app.host_telemetry.cpu_utilization_percent = Some(18);
    app.host_telemetry.logical_cpu_count = Some(4);
    app.host_telemetry.memory_total_bytes = Some(16 * 1024 * 1024 * 1024);
    app.host_telemetry.memory_available_bytes = Some(9_964_324_126);
    app.host_telemetry.disk_total_bytes = Some(150 * 1024 * 1024 * 1024);
    app.host_telemetry.disk_available_bytes = Some(59_592_671_232);
    app.screen = Screen::Dashboard;
    app.navigator_selection = 0;
    app.focus = FocusTarget::Navigator;
    app.build.status = BuildStatus::Idle;
    app.build.started = None;
    app.build.completed = 0;
    app.build.total = None;
    app.tasks.clear();
    app.completed_tasks.clear();
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Exited;
    app.daemon.jobs.clear();
    app.daemon.pty_sessions.clear();
    app.daemon.pty_screens.clear();
    if let Some(telemetry) = app.daemon.telemetry.as_mut() {
        telemetry.active_jobs = 0;
        telemetry.pty_sessions = 0;
    }
    app
}

pub(crate) fn concept_failed_errors_app() -> App {
    let mut app = literal_reference_app();
    app.screen = Screen::Errors;
    app.navigator_selection = 11;
    app.focus = FocusTarget::Workspace;
    app.build.status = BuildStatus::Failed;
    app.build.exit_code = Some(1);
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Failed;
    if let Some(job) = app.daemon.jobs.first_mut() {
        job.lifecycle = yoctui_model::ClientDaemonLifecycle::Failed;
        job.exit_code = Some(1);
    }
    app.logs = yoctui_model::LogState::new(512, 1024 * 1024);
    let failed_task = app
        .tasks
        .get_mut(&yoctui_model::TaskId("bash:do_compile".into()))
        .expect("literal fixture has the selected compile task");
    failed_task.state = yoctui_model::TaskState::Failed;
    failed_task.progress = None;
    failed_task.finished = Some(literal_now());
    for (severity, message) in [
        (
            Severity::Info,
            "NOTE: running task bash:do_compile with oe_runmake",
        ),
        (
            Severity::Warning,
            "WARNING: bash:do_compile found a recoverable configure mismatch",
        ),
        (
            Severity::Error,
            "ERROR: bash:do_compile failed with exit code 1",
        ),
    ] {
        let _ = update(
            &mut app,
            Action::Log(yoctui_model::LogEntry {
                id: 0,
                severity,
                message: message.into(),
                recipe: Some("bash_5.2.21-2".into()),
                task: Some("do_compile".into()),
                path: Some("/workspace/yocto/build/tmp/log.do_compile.85873".into()),
                timestamp: literal_now(),
                build: Some("core-image-minimal".into()),
                protected: true,
                diagnostic: None,
            }),
        );
    }
    app.build.errors = 1;
    app.build.warnings = 1;
    app.error_selection = 1;
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.query = "do_compile".into();
    app.logs.selection = app.logs.visible_count().saturating_sub(1);
    app.logs.dropped = 2;
    app.logs.dropped_warnings = 1;
    app.logs.dropped_errors = 1;
    app
}
