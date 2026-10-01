use super::*;
use yoctui_protocol::daemon::*;

fn record(source: &Path, build: &Path) -> SavedBuild {
    SavedBuild {
        id: "past".into(),
        target: "do-not-replay".into(),
        machine: Some("historical".into()),
        source: Some(source.display().to_string()),
        build_dir: Some(build.display().to_string()),
        outcome: yoctui_model::SavedBuildOutcome::Failed,
        saved_unix_ms: 1,
        started_unix_ms: None,
        finished_unix_ms: None,
        logs: Vec::new(),
        tasks: Vec::new(),
        limitations: Vec::new(),
    }
}

fn snapshot() -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([1; 16]),
        sequence: 1,
        generation: 7,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Disconnected,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        build_progress: None,
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        recovery_warnings: Vec::new(),
    }
}

#[tokio::test]
async fn saved_environment_validates_sibling_paths_without_sourcing_then_initializes_exactly() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-saved-environment-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let source = root.join("source with spaces");
    let build = root.join("build with spaces");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(build.join("conf")).unwrap();
    fs::write(build.join("conf/local.conf"), "MACHINE = \"current\"\n").unwrap();
    fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
    let script = source.join("oe-init-build-env");
    fs::write(
        &script,
        "#!/bin/bash\nprintf sourced > \"$1/sourced-marker\"\nexport BUILDDIR=\"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let profile = saved_profile(&record(&source, &build)).unwrap();
    revalidate_profile(&profile).unwrap();
    assert!(!build.join("sourced-marker").exists());
    let initialized = BuildEnvironmentAdapter::default()
        .initialize(profile.clone())
        .await
        .unwrap();
    assert_eq!(
        initialized.environment.get("BUILDDIR"),
        Some(&build.display().to_string())
    );
    assert!(build.join("sourced-marker").exists());
    assert_eq!(
        fs::read_to_string(build.join("conf/local.conf")).unwrap(),
        "MACHINE = \"current\"\n"
    );
    fs::remove_file(build.join("conf/local.conf")).unwrap();
    assert!(revalidate_profile(&profile).is_err());
    fs::remove_dir_all(&root).unwrap();
    assert!(saved_profile(&record(&source, &build)).is_err());
    assert!(
        !build.exists(),
        "deleted build directories must not be recreated"
    );
}

#[test]
fn saved_environment_replacement_guards_jobs_terminals_and_stale_generation() {
    let mut snapshot = snapshot();
    assert!(replacement_denial(&snapshot, Some(7)).is_none());
    assert_eq!(
        replacement_denial(&snapshot, Some(6)).unwrap().0,
        ProtocolErrorCode::StaleGeneration
    );
    snapshot.jobs.push(JobSummary {
        id: JobId(1),
        kind: JobKind::BitBakeBuild,
        label: "new work after review".into(),
        lifecycle: LifecycleState::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });
    assert!(
        replacement_denial(&snapshot, Some(7))
            .unwrap()
            .1
            .contains("1 active job")
    );
    snapshot.jobs[0].lifecycle = LifecycleState::Exited;
    snapshot.pty_sessions.push(PtySessionSummary {
        id: PtySessionId(1),
        name: "shell".into(),
        kind: PtyKind::BuildShell,
        cwd: "/build".into(),
        lifecycle: LifecycleState::Running,
        dimensions: TerminalDimensions {
            rows: 24,
            columns: 80,
        },
        writer: None,
        writer_epoch: 0,
        viewers: 0,
        exit_code: None,
        restartable: true,
    });
    assert!(
        replacement_denial(&snapshot, Some(7))
            .unwrap()
            .1
            .contains("1 active terminal")
    );
    for lifecycle in [LifecycleState::Connecting, LifecycleState::Stopping] {
        snapshot.pty_sessions[0].lifecycle = lifecycle;
        assert!(replacement_denial(&snapshot, Some(7)).is_some());
    }
    snapshot.pty_sessions[0].lifecycle = LifecycleState::Exited;
    assert!(replacement_denial(&snapshot, Some(7)).is_none());
}

#[test]
fn saved_environment_missing_history_paths_are_explicit_errors() {
    let mut record = record(Path::new("/missing/source"), Path::new("/missing/build"));
    record.build_dir = None;
    assert!(
        saved_profile(&record)
            .unwrap_err()
            .to_string()
            .contains("no recorded build directory")
    );
    record.build_dir = Some("relative/build".into());
    assert!(
        saved_profile(&record)
            .unwrap_err()
            .to_string()
            .contains("absolute")
    );
}

#[test]
fn saved_environment_review_distinguishes_start_attach_restart_and_pending_authority() {
    let profile = BuildEnvironmentProfile {
        source_dir: "/source".into(),
        build_dir: "/build".into(),
        init_script: "/source/oe-init-build-env".into(),
    };
    assert_eq!(
        reviewed_mode(&profile, None).unwrap(),
        SavedEnvironmentMode::Start
    );
    let mut snapshot = snapshot();
    assert!(reviewed_mode(&profile, Some(snapshot.clone())).is_err());
    snapshot.compatibility = Some(CompatibilitySnapshotData {
        schema_version: COMPATIBILITY_SCHEMA_VERSION,
        generation: 1,
        environment: CompatibilityEnvironmentIdentity {
            build_directory: CompatibilityDetected::Detected {
                value: "/build".into(),
                authority: CompatibilityIdentityAuthority::InitializedEnvironment,
            },
            source_roots: CompatibilityDetected::Unknown,
            bitbake_version: CompatibilityDetected::Unknown,
            oe_core: CompatibilityDetected::Unknown,
            poky: CompatibilityDetected::Unknown,
            distro: CompatibilityDetected::Unknown,
            machine: CompatibilityDetected::Unknown,
            layer_series: CompatibilityDetected::Unknown,
            available_tools: CompatibilityDetected::Unknown,
            backend: CompatibilityDetected::Unknown,
            protocol: CompatibilityDetected::Unknown,
        },
        capabilities: Vec::new(),
    });
    assert_eq!(
        reviewed_mode(&profile, Some(snapshot.clone())).unwrap(),
        SavedEnvironmentMode::Attach { instance: [1; 16] }
    );
    let different = BuildEnvironmentProfile {
        build_dir: "/another-build".into(),
        ..profile
    };
    assert_eq!(
        reviewed_mode(&different, Some(snapshot.clone())).unwrap(),
        SavedEnvironmentMode::Restart { instance: [1; 16] }
    );
    snapshot.jobs.push(JobSummary {
        id: JobId(1),
        kind: JobKind::BitBakeBuild,
        label: "work".into(),
        lifecycle: LifecycleState::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });
    assert!(reviewed_mode(&different, Some(snapshot)).is_err());
}

#[test]
fn saved_environment_shutdown_correlates_generation_and_rejections_over_real_ipc() {
    use yoctui_protocol::daemon_ipc::{DaemonConnection, DaemonListener, runtime_paths_for};
    for reject in [false, true] {
        let root = std::env::temp_dir().join(format!(
            "yoctui-saved-env-ipc-{}-{}",
            std::process::id(),
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
        let listener = DaemonListener::bind(&paths).unwrap();
        let server = std::thread::spawn(move || {
            let mut connection = listener.accept(Duration::from_secs(2)).unwrap();
            connection
                .set_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let ClientMessage::Command(request) = connection.receive::<ClientMessage>().unwrap()
            else {
                panic!("expected reviewed shutdown")
            };
            assert_eq!(request.expected_generation, Some(7));
            assert_eq!(request.command, DaemonCommand::PrepareShutdown);
            let outcome = if reject {
                CommandOutcome::Rejected {
                    code: ProtocolErrorCode::LimitExceeded,
                    message: "new terminal started after review".into(),
                    current_generation: 8,
                }
            } else {
                CommandOutcome::Completed
            };
            connection
                .send(&ServerMessage::CommandResult(CommandResult {
                    request_id: request.request_id,
                    outcome,
                }))
                .unwrap();
        });
        let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(2)).unwrap();
        connection
            .set_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let result = request_reviewed_shutdown(&mut connection, &snapshot(), [1; 16]);
        assert_eq!(result.is_err(), reject);
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}

#[tokio::test]
async fn saved_environment_pending_worker_is_owned_and_nonblocking() {
    let operation = Operation {
        generation: 1,
        loading: true,
        task: tokio::spawn(std::future::pending()),
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    assert!(!operation.task.is_finished());
    let handle = operation.task.abort_handle();
    let cancelled = operation.cancelled.clone();
    drop(operation);
    tokio::task::yield_now().await;
    assert!(handle.is_finished());
    assert!(cancelled.load(Ordering::Relaxed));
}

#[test]
fn saved_environment_cancelled_discovery_does_not_connect_or_wait() {
    let plan = SavedEnvironmentPlan {
        profile: BuildEnvironmentProfile {
            source_dir: "/source".into(),
            build_dir: "/build".into(),
            init_script: "/source/oe-init-build-env".into(),
        },
        target: "image".into(),
        machine: None,
        mode: SavedEnvironmentMode::Attach { instance: [1; 16] },
    };
    let started = Instant::now();
    let result = attach_loaded(plan, &AtomicBool::new(true));
    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_millis(50));
}
