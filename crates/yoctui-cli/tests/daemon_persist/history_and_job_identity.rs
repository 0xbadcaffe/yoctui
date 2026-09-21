use super::support::*;

#[test]
fn raw_history_persistence_round_trips_safe_terminal_metadata_and_rejects_future_schema() {
    let root = unique_temp_root("yoctui-cli-raw-history");
    let cleanup = Cleanup(root.clone());
    let paths = persist_paths_for(&root).unwrap();
    let snapshot = raw_history_snapshot(raw_history_record());
    let persisted = DaemonPersistedState::capture(
        &snapshot,
        200,
        "boot-one".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    write_persisted_state(&paths, &persisted).unwrap();
    let loaded = read_persisted_state(&paths).unwrap().unwrap();
    assert_eq!(loaded.raw_history, snapshot.raw_history);
    let serialized = fs::read_to_string(&paths.state).unwrap();
    for prohibited in [
        "raw-job:",
        "raw-session:",
        "process_group",
        "writer_epoch",
        "capability_generation",
        "preview_digest",
        "build_directory",
        "stdout",
        "stderr",
        "pty_screens",
    ] {
        assert!(!serialized.contains(prohibited), "retained {prohibited}");
    }

    let mut current = raw_history_snapshot(raw_history_record());
    current.raw_history.clear();
    current.daemon_instance_id = DaemonInstanceId([6; 16]);
    let (recovered, _) = recover_persisted_snapshot(current, &loaded, "boot-one");
    assert_eq!(recovered.raw_history, loaded.raw_history);
    assert!(recovered.raw_executions.is_empty());
    assert!(recovered.pty_sessions.is_empty());

    let mut future = loaded;
    future.raw_history[0].schema_version += 1;
    assert!(write_persisted_state(&paths, &future).is_err());
    drop(cleanup);
}

#[test]
fn daemon_persist_fixtures_use_unique_temp_roots() {
    let first = unique_temp_root("yoctui-cli-daemon-recovery");
    let second = unique_temp_root("yoctui-cli-daemon-recovery");

    assert_ne!(first, second);
}

#[test]
fn daemon_job_identity_real_daemon_preserves_recovery_across_report_owners() {
    use yoctui_protocol::daemon::{CommandOutcome, CommandRequest, DaemonCommand, RequestId};

    struct OwnedDaemon(PathBuf);
    impl OwnedDaemon {
        fn run(&self, action: &str) -> std::process::Output {
            Command::new(env!("CARGO_BIN_EXE_yoctui"))
                .args(["daemon", action])
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", &self.0)
                .env("XDG_CONFIG_HOME", self.0.join("config"))
                .env("XDG_RUNTIME_DIR", self.0.join("runtime"))
                .env("XDG_STATE_HOME", self.0.join("state"))
                .current_dir(&self.0)
                .output()
                .unwrap()
        }
    }
    impl Drop for OwnedDaemon {
        fn drop(&mut self) {
            if self.run("stop").status.success() {
                let _ = fs::remove_dir_all(&self.0);
            } else {
                eprintln!(
                    "retained private daemon fixture for recovery: {}",
                    self.0.display()
                );
            }
        }
    }

    fn receive_until(
        connection: &mut DaemonConnection,
        mut accept: impl FnMut(ServerMessage) -> Option<DaemonSnapshot>,
    ) -> DaemonSnapshot {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            let message = connection.receive::<ServerMessage>().unwrap();
            if let ServerMessage::Ping { nonce, .. } = message {
                connection.send(&ClientMessage::Pong { nonce }).unwrap();
            } else if let Some(snapshot) = accept(message) {
                return snapshot;
            }
        }
        panic!("private daemon response deadline exceeded");
    }

    let root = unique_temp_root("yoctui-daemon-job-identity");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(root.join("runtime"))
        .unwrap();
    let daemon = OwnedDaemon(root.clone());
    let mut before = raw_history_snapshot(raw_history_record());
    before.raw_history.clear();
    before.jobs = [1, 2]
        .into_iter()
        .map(|id| JobSummary {
            id: JobId(id),
            kind: JobKind::BitBakeBuild,
            label: format!("old build {id}"),
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code: Some(1),
        })
        .collect();
    write_persisted_state(
        &persist_paths_for(&root.join("state")).unwrap(),
        &DaemonPersistedState::capture(
            &before,
            10,
            read_boot_id().unwrap(),
            Vec::new(),
            PersistedPreferences::default(),
        ),
    )
    .unwrap();
    let report = root.join("report.json");
    fs::write(&report, "{}").unwrap();
    let start = daemon.run("start");
    assert!(start.status.success(), "{start:?}");
    // SAFETY: geteuid has no preconditions and does not modify memory.
    let paths = runtime_paths_for(root.join("runtime"), unsafe { libc::geteuid() }).unwrap();
    let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(2)).unwrap();
    connection
        .set_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    connection
        .send(&ClientMessage::Hello(ClientHello {
            minimum_version: ProtocolVersion::CURRENT,
            maximum_version: ProtocolVersion::CURRENT,
            client_id: ClientId([19; 16]),
            client_name: "job-identity-test".into(),
            capabilities: vec![Capability::StateSnapshots, Capability::BackgroundJobs],
        }))
        .unwrap();
    assert!(matches!(
        connection.receive::<ServerMessage>().unwrap(),
        ServerMessage::Hello(_)
    ));
    let attach = ClientMessage::Attach {
        workspace: None,
        subscription: Subscription {
            state: true,
            jobs: true,
            logs: false,
            pty_sessions: Vec::new(),
        },
        resume: None,
    };
    connection.send(&attach).unwrap();
    let recovered = receive_until(&mut connection, |message| match message {
        ServerMessage::Attached { snapshot, .. } => Some(snapshot),
        _ => None,
    });
    assert_eq!(recovered.jobs, before.jobs);
    for (index, command) in [
        DaemonCommand::StartQaReportScan {
            generation: 1,
            build_directory: root.display().to_string(),
            paths: vec![report.display().to_string()],
        },
        DaemonCommand::StartSecurityReportScan {
            generation: 1,
            paths: vec![report.display().to_string()],
        },
    ]
    .into_iter()
    .enumerate()
    {
        let request_id = RequestId(index as u64 + 1);
        connection
            .send(&ClientMessage::Command(CommandRequest {
                request_id,
                expected_generation: None,
                command,
            }))
            .unwrap();
        receive_until(&mut connection, |message| match message {
            ServerMessage::CommandResult(result) if result.request_id == request_id => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                Some(before.clone())
            }
            _ => None,
        });
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let after = loop {
        connection.send(&attach).unwrap();
        let snapshot = receive_until(&mut connection, |message| match message {
            ServerMessage::Attached { snapshot, .. } => Some(snapshot),
            _ => None,
        });
        if snapshot.jobs.len() == 4
            && snapshot.jobs.iter().all(|job| {
                matches!(
                    job.lifecycle,
                    LifecycleState::Exited | LifecycleState::Failed
                )
            })
        {
            break snapshot;
        }
        assert!(std::time::Instant::now() < deadline, "{snapshot:?}");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(&after.jobs[..2], &before.jobs);
    assert_eq!(
        (after.jobs[2].id, after.jobs[2].kind),
        (JobId(3), JobKind::Qa)
    );
    assert_eq!(
        (after.jobs[3].id, after.jobs[3].kind),
        (JobId(4), JobKind::Security)
    );
    drop(connection);
    drop(daemon);
}
