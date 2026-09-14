//! Regression tests grouped around mouse_runtime_routes_dialog_and_terminal_session_clicks.
use super::*;

#[test]
fn mouse_runtime_routes_dialog_and_terminal_session_clicks() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 2
            },
            &app,
            120,
            30,
        ),
        Some(Action::SelectPtyPane {
            pane: yoctui_model::PaneId(1),
            index: 0,
        })
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 40,
                row: 2,
            },
            &app,
            120,
            30,
        ),
        None,
        "a single terminal leaf has no supported split to resize"
    );
    app.dialogs.push_back(yoctui_model::Dialog::BuildOptions);
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 2
            },
            &app,
            120,
            30,
        ),
        Some(Action::Focus(FocusTarget::Dialog))
    );
}

#[test]
fn daemon_state_jobs_cross_app_boundary_without_replacing_presentation() {
    let mut authoritative = yoctui_model::App::new(16, 4096);
    authoritative.qemu_session_generation = 9;
    authoritative.screen = Screen::Maintenance;
    let jobs = daemon_job_state_from_app(&authoritative);

    let mut client = yoctui_model::App::new(16, 4096);
    client.screen = Screen::Layers;
    client.focus = FocusTarget::Navigator;
    install_daemon_job_replica(&mut client, &jobs);
    assert_eq!(client.qemu_sessions, authoritative.qemu_sessions);
    assert_eq!(client.background_jobs, authoritative.background_jobs);
    assert_eq!(client.screen, Screen::Layers);
    assert_eq!(client.focus, FocusTarget::Navigator);
}

#[test]
fn daemon_state_runtime_reduces_authority_and_exposes_current_replica() {
    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([7; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let jobs = daemon_job_state_from_app(&yoctui_model::App::new(16, 4096));
    let revision = reduce_daemon_state(
        &mut state,
        yoctui_model::DaemonStateAction::ReplaceJobs(Box::new(jobs)),
    )
    .unwrap();

    assert_eq!(revision.sequence, 1);
    assert_eq!(revision.generation, 1);
    assert!(state.jobs.is_some());
    let replica = client_replica_from_daemon(&state);
    assert_eq!(replica.status, yoctui_model::ClientReplicaStatus::Current);
    assert_eq!(replica.state.as_ref(), Some(&state));
    let snapshot = daemon_protocol_snapshot(&state);
    assert_eq!(snapshot.daemon_instance_id.0, [7; 16]);
    assert_eq!(snapshot.sequence, 1);
    assert_eq!(snapshot.generation, 1);
    assert!(matches!(
        snapshot.bitbake.lifecycle,
        yoctui_protocol::daemon::LifecycleState::Disconnected
    ));
}

#[test]
fn rootfs_daemon_authority_retains_full_instance_beyond_display_prefix() {
    let authority = compatibility_workspace_authority(1).normalize().unwrap();
    let mut snapshot = compatibility_workspace_daemon_snapshot(&authority);
    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot.clone());
    let first = app.daemon.instance_id.unwrap();
    let short = app.daemon.instance_identity.clone();
    snapshot.daemon_instance_id.0[15] ^= 1;
    client.replace_app(&mut app, snapshot.clone());
    assert_eq!(app.daemon.instance_identity, short);
    assert_ne!(app.daemon.instance_id, Some(first));
    assert_eq!(
        app.daemon.instance_id,
        Some(yoctui_model::DaemonModelInstanceId(
            snapshot.daemon_instance_id.0
        ))
    );
}

#[test]
fn daemon_snapshot_client_applies_ordered_events_and_replaces_stale_state() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([4; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut client = DaemonClientSnapshot::default();
    client.begin_synchronization();
    client.replace(initial.clone());
    let event = yoctui_protocol::daemon::SequencedEvent {
        sequence: 1,
        generation: 1,
        event: yoctui_protocol::daemon::DaemonEvent::Log(yoctui_protocol::daemon::LogRecord {
            source: "bitbake".into(),
            severity: yoctui_protocol::daemon::LogSeverity::Info,
            message: "parsing".into(),
            unix_ms: 1,
            recipe: None,
            task: None,
            path: None,
            build: None,
        }),
    };
    client.apply_event(&event).unwrap();
    assert_eq!(client.snapshot.as_ref().unwrap().sequence, 1);
    assert_eq!(client.resume_cursor().unwrap().last_sequence, 1);

    let gap = yoctui_protocol::daemon::SequencedEvent {
        sequence: 3,
        generation: 3,
        event: yoctui_protocol::daemon::DaemonEvent::Unknown,
    };
    assert!(matches!(
        client.apply_event(&gap),
        Err(DaemonClientSyncError::Protocol(
            yoctui_protocol::daemon::DaemonSnapshotError::EventGap { .. }
        ))
    ));
    assert_eq!(client.status, yoctui_model::ClientReplicaStatus::Stale);
    assert!(client.resume_cursor().is_none());

    client.begin_synchronization();
    client.replace(initial.clone());
    assert_eq!(client.snapshot.as_ref(), Some(&initial));
    assert_eq!(client.status, yoctui_model::ClientReplicaStatus::Current);
    client.disconnect();
    assert_eq!(
        client.status,
        yoctui_model::ClientReplicaStatus::Disconnected
    );
    assert_eq!(client.snapshot.as_ref(), Some(&initial));
}

#[test]
fn daemon_client_batches_contiguous_logs_with_one_model_install() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([6; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(16, 4096);
    client.replace_app(&mut app, daemon_protocol_snapshot(&state));
    let record = |severity, message: &str| yoctui_protocol::daemon::LogRecord {
        source: "bitbake".into(),
        severity,
        message: message.into(),
        unix_ms: 42,
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        build: Some("core-image-minimal".into()),
    };
    let events = vec![
        yoctui_protocol::daemon::SequencedEvent {
            sequence: 1,
            generation: 1,
            event: yoctui_protocol::daemon::DaemonEvent::Log(record(
                yoctui_protocol::daemon::LogSeverity::Info,
                "ordinary",
            )),
        },
        yoctui_protocol::daemon::SequencedEvent {
            sequence: 2,
            generation: 2,
            event: yoctui_protocol::daemon::DaemonEvent::Log(record(
                yoctui_protocol::daemon::LogSeverity::Error,
                "critical",
            )),
        },
    ];

    client.apply_log_events_to_app(&mut app, &events).unwrap();
    assert_eq!(client.resume_cursor().unwrap().last_sequence, 2);
    assert_eq!(app.logs.entries.len(), 2);
    assert_eq!(app.logs.entries[0].message, "ordinary");
    assert_eq!(app.logs.entries[1].message, "critical");
    assert!(app.logs.entries[1].protected);
    assert_eq!(app.build.errors, 1);
}

#[test]
fn daemon_client_batches_task_progress_without_losing_failure() {
    use yoctui_protocol::daemon::{DaemonBuildEvent, DaemonEvent, SequencedEvent};

    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(16, 4096);
    client.replace_app(&mut app, daemon_protocol_snapshot(&state));
    let task = |sequence, event| SequencedEvent {
        sequence,
        generation: sequence,
        event: DaemonEvent::Build(event),
    };
    let events = vec![
        task(
            1,
            DaemonBuildEvent::TaskStarted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: None,
            },
        ),
        task(
            2,
            DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(10),
            },
        ),
        task(
            3,
            DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(90),
            },
        ),
        task(
            4,
            DaemonBuildEvent::TaskCompleted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                success: false,
                started_unix_ms: None,
                finished_unix_ms: None,
            },
        ),
    ];

    client.apply_task_events_to_app(&mut app, &events).unwrap();
    assert_eq!(client.resume_cursor().unwrap().last_sequence, 4);
    assert_eq!(app.task_progress_events, 2);
    assert_eq!(app.task_progress_coalesced, 1);
    assert!(app.tasks.is_empty());
    assert_eq!(app.completed_tasks.len(), 1);
    assert!(!app.completed_tasks[0].success);
    assert_eq!(
        app.completed_tasks[0].task.state,
        yoctui_model::TaskState::Failed
    );
    assert!(matches!(
        client.snapshot.as_ref().unwrap().build_events.last(),
        Some(DaemonBuildEvent::TaskCompleted { success: false, .. })
    ));
}

#[test]
fn daemon_log_context_survives_snapshot_and_live_event_mapping() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([8; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    let record = yoctui_protocol::daemon::LogRecord {
        source: "bitbake".into(),
        severity: yoctui_protocol::daemon::LogSeverity::Info,
        message: "compiler output".into(),
        unix_ms: 42,
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/build/tmp/work/busybox/temp/log.do_compile".into()),
        build: Some("core-image-minimal".into()),
    };
    snapshot.recent_logs.push(record.clone());

    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(16, 4096);
    client.replace_app(&mut app, snapshot);
    let retained = app.logs.entries.back().unwrap();
    assert_eq!(retained.recipe.as_deref(), Some("busybox"));
    assert_eq!(retained.task.as_deref(), Some("do_compile"));
    assert_eq!(
        retained.path.as_deref(),
        Some(std::path::Path::new(
            "/build/tmp/work/busybox/temp/log.do_compile"
        ))
    );
    assert_eq!(retained.build.as_deref(), Some("core-image-minimal"));

    let event = yoctui_protocol::daemon::SequencedEvent {
        sequence: client.snapshot.as_ref().unwrap().sequence + 1,
        generation: client.snapshot.as_ref().unwrap().generation + 1,
        event: yoctui_protocol::daemon::DaemonEvent::Log(record),
    };
    client.apply_event_to_app(&mut app, &event).unwrap();
    assert_eq!(
        app.logs.entries.back().unwrap().task.as_deref(),
        Some("do_compile")
    );
}

#[test]
fn logs_open_with_scrollable_workspace_focus() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.focus = FocusTarget::Navigator;
    let _ = yoctui_model::update(&mut app, Action::Open(Screen::Logs));
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(
        focus_action_for_app(&app, Input::PageUp),
        None,
        "workspace-owned log scrolling must reach the collection router"
    );
    assert_eq!(
        workspace_collection_action(&app, Input::PageUp),
        Some(Action::ScrollLogs { delta: 10 })
    );
}

#[test]
fn daemon_snapshot_resync_preserves_paused_live_log_scroll() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    for index in 0..12 {
        snapshot
            .recent_logs
            .push(yoctui_protocol::daemon::LogRecord {
                source: "bitbake".into(),
                severity: yoctui_protocol::daemon::LogSeverity::Info,
                message: format!("line {index}"),
                unix_ms: index,
                recipe: Some("busybox".into()),
                task: Some("do_compile".into()),
                path: None,
                build: Some("core-image-minimal".into()),
            });
    }
    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(32, 8192);
    client.replace_app(&mut app, snapshot.clone());
    let _ = yoctui_model::update(&mut app, Action::ScrollLogs { delta: 5 });
    app.logs.query = "line".into();
    let selection = app.logs.selection;

    snapshot.sequence += 1;
    snapshot.generation += 1;
    snapshot
        .recent_logs
        .push(yoctui_protocol::daemon::LogRecord {
            source: "bitbake".into(),
            severity: yoctui_protocol::daemon::LogSeverity::Info,
            message: "line 12".into(),
            unix_ms: 12,
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            build: Some("core-image-minimal".into()),
        });
    client.replace_app(&mut app, snapshot);

    assert!(!app.logs.follow);
    assert_eq!(app.logs.query, "line");
    assert_eq!(app.logs.selection, selection);
    assert!(app.logs.scroll_offset > 0);
}

#[test]
fn daemon_cancel_terminal_clears_pending_build_state() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.build.status = yoctui_model::BuildStatus::Cancelling;
    apply_daemon_build_event(
        &mut app,
        yoctui_protocol::daemon::DaemonBuildEvent::Completed {
            success: false,
            exit_code: Some(130),
            finished_unix_ms: None,
        },
    );
    assert_eq!(app.build.status, yoctui_model::BuildStatus::Cancelled);
    assert_eq!(app.build.exit_code, Some(130));
}

#[test]
fn next_generation_pty_screen_crosses_replica_as_typed_bounded_rows() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "pty-screen".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut client = DaemonClientSnapshot::default();
    client.replace(daemon_protocol_snapshot(&state));
    let mut app = yoctui_model::App::new(16, 4096);
    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: 1,
                generation: 1,
                event: yoctui_protocol::daemon::DaemonEvent::PtyScreen(
                    yoctui_protocol::daemon::PtyScreenSnapshot {
                        session_id: yoctui_protocol::daemon::PtySessionId(4),
                        dimensions: yoctui_protocol::daemon::TerminalDimensions {
                            columns: 20,
                            rows: 3,
                        },
                        cursor_column: 2,
                        cursor_row: 1,
                        cursor_hidden: false,
                        scrollback_offset: 0,
                        cells: vec![yoctui_protocol::daemon::PtyScreenCell {
                            index: 0,
                            contents: "r".into(),
                            foreground: yoctui_protocol::daemon::PtyTerminalColor::Rgb(1, 2, 3),
                            background: yoctui_protocol::daemon::PtyTerminalColor::Default,
                            bold: true,
                            dim: false,
                            italic: false,
                            underline: false,
                            inverse: false,
                            wide: false,
                            wide_continuation: false,
                        }],
                        scrollback_lines: 8,
                        dropped_line_feeds_lower_bound: 2,
                    },
                ),
            },
        )
        .unwrap();
    let screen = &app.daemon.pty_screens[0];
    assert_eq!(screen.session_id, 4);
    assert_eq!(screen.rows[0], "r");
    assert_eq!(screen.scrollback_lines, 8);
    assert_eq!(screen.cells.len(), 60);
    assert_eq!(screen.cells[0].contents, "r");
    assert!(screen.cells[0].bold);
    assert_eq!(
        screen.cells[0].foreground,
        yoctui_model::ClientDaemonTerminalColor::Rgb(1, 2, 3)
    );
}

#[test]
fn daemon_compatibility_snapshot_is_identical_on_attach_reconnect_and_update() {
    fn compatibility(generation: u64) -> yoctui_model::DaemonCompatibilitySnapshot {
        let environment = yoctui_model::YoctoEnvironmentIdentity {
            build_directory: yoctui_model::AuthoritativeValue::detected(
                "/work/poky/build".into(),
                yoctui_model::IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: yoctui_model::AuthoritativeValue::detected(
                "2.18.0".into(),
                yoctui_model::IdentityAuthority::BitBakeVersionProbe,
            ),
            ..yoctui_model::YoctoEnvironmentIdentity::default()
        };
        yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: yoctui_model::CapabilitySnapshot {
                generation,
                environment,
                capabilities: vec![yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::BitBakeBuild,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                        subject: "bitbake executable".into(),
                        detail: "The initialized environment exposes BitBake.".into(),
                        argv: Vec::new(),
                    }],
                }],
            },
            implementations: std::collections::BTreeMap::from([(
                yoctui_model::CapabilityId::BitBakeBuild,
                yoctui_model::CapabilityImplementation {
                    id: "bitbake.build.command".into(),
                    kind: yoctui_model::CapabilityImplementationKind::Command,
                },
            )]),
        }
    }

    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([6; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    reduce_daemon_state(
        &mut state,
        yoctui_model::DaemonStateAction::ReplaceCompatibility(Box::new(compatibility(1))),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    initial.compatibility.as_ref().unwrap().validate().unwrap();

    let mut first_client = DaemonClientSnapshot::default();
    let mut reconnecting_client = DaemonClientSnapshot::default();
    first_client.replace(initial.clone());
    reconnecting_client.replace(initial.clone());
    assert_eq!(
        first_client.snapshot.as_ref().unwrap().compatibility,
        reconnecting_client.snapshot.as_ref().unwrap().compatibility
    );

    let replacement = daemon_compatibility_protocol(&compatibility(2));
    let event = yoctui_protocol::daemon::SequencedEvent {
        sequence: initial.sequence + 1,
        generation: initial.generation + 1,
        event: yoctui_protocol::daemon::DaemonEvent::CompatibilityChanged(Box::new(replacement)),
    };
    first_client.apply_event(&event).unwrap();
    assert_eq!(
        first_client
            .snapshot
            .as_ref()
            .unwrap()
            .compatibility
            .as_ref()
            .unwrap()
            .generation,
        2
    );
    assert_eq!(
        reconnecting_client
            .snapshot
            .as_ref()
            .unwrap()
            .compatibility
            .as_ref()
            .unwrap()
            .generation,
        1
    );
}

#[test]
fn daemon_status_event_updates_client_telemetry_without_mutating_snapshot_shape() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut client = DaemonClientSnapshot::default();
    client.replace(initial);
    client
        .apply_event(&yoctui_protocol::daemon::SequencedEvent {
            sequence: 1,
            generation: 1,
            event: yoctui_protocol::daemon::DaemonEvent::Telemetry(
                yoctui_protocol::daemon::DaemonTelemetry {
                    uptime_seconds: 7,
                    bitbake: yoctui_protocol::daemon::LifecycleState::Running,
                    connected_clients: 2,
                    active_jobs: 1,
                    pty_sessions: 1,
                    queue_depth: 3,
                    pressure: yoctui_protocol::daemon::DaemonPressureCounters {
                        current_queue_depth: 3,
                        maximum_queue_depth: 9,
                        cosmetic_coalesced: 2,
                        cosmetic_dropped: 4,
                        reliable_waits: 1,
                        forced_resynchronizations: 2,
                        slow_client_disconnects: 1,
                    },
                    memory_bytes: Some(4096),
                    recovery: yoctui_protocol::daemon::DaemonRecoveryState::Recovered,
                },
            ),
        })
        .unwrap();
    let telemetry = client.telemetry.unwrap();
    assert_eq!(telemetry.uptime_seconds, 7);
    assert_eq!(telemetry.pressure.current_queue_depth, 3);
    assert_eq!(telemetry.pressure.maximum_queue_depth, 9);
    assert_eq!(telemetry.pressure.cosmetic_coalesced, 2);
    assert_eq!(telemetry.pressure.cosmetic_dropped, 4);
    assert_eq!(telemetry.pressure.reliable_waits, 1);
    assert_eq!(telemetry.pressure.forced_resynchronizations, 2);
    assert_eq!(telemetry.pressure.slow_client_disconnects, 1);
    assert_eq!(client.snapshot.unwrap().sequence, 1);
}

#[test]
fn client_replica_installs_authority_without_replacing_presentation() {
    use yoctui_protocol::daemon::{
        DaemonEvent, JobId, JobKind, JobSummary, LifecycleState, PtyKind, PtySessionId,
        PtySessionSummary, TerminalDimensions,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([4; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut initial = daemon_protocol_snapshot(&state);
    initial.bitbake.lifecycle = LifecycleState::Running;
    initial.jobs.push(JobSummary {
        id: JobId(8),
        kind: JobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: LifecycleState::Running,
        progress_current: Some(3),
        progress_total: Some(10),
        exit_code: None,
    });
    initial.pty_sessions.push(PtySessionSummary {
        id: PtySessionId(9),
        name: "devshell".into(),
        kind: PtyKind::Devshell,
        cwd: "/build".into(),
        lifecycle: LifecycleState::Running,
        dimensions: TerminalDimensions {
            columns: 80,
            rows: 24,
        },
        writer: None,
        writer_epoch: 0,
        viewers: 2,
        exit_code: None,
        restartable: true,
    });
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Inspector;
    app.theme = yoctui_model::Theme::MatrixGreen;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    let mut client = DaemonClientSnapshot::default();
    client.begin_synchronization();
    client.replace_app(&mut app, initial);
    assert_eq!(
        app.daemon.status,
        yoctui_model::ClientReplicaStatus::Current
    );
    assert_eq!(
        app.daemon.bitbake,
        yoctui_model::ClientDaemonLifecycle::Running
    );
    assert_eq!(app.daemon.jobs[0].label, "core-image-minimal");
    assert_eq!(app.daemon.pty_sessions[0].viewers, 2);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert_eq!(app.theme, yoctui_model::Theme::MatrixGreen);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::QuitConfirmation)
    ));

    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: 1,
                generation: 1,
                event: DaemonEvent::JobRemoved { job_id: JobId(8) },
            },
        )
        .unwrap();
    assert!(app.daemon.jobs.is_empty());
    client.disconnect_app(&mut app);
    assert_eq!(
        app.daemon.status,
        yoctui_model::ClientReplicaStatus::Disconnected
    );
    assert_eq!(app.screen, Screen::Recipes);
}

#[test]
fn daemon_jobs_populate_shared_job_history() {
    use yoctui_protocol::daemon::{JobId, JobKind, JobSummary, LifecycleState};

    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([3; 16]),
        123,
        "job-history".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.jobs = vec![
        JobSummary {
            id: JobId(8),
            kind: JobKind::BitBakeBuild,
            label: "BitBake build core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(59),
            progress_total: Some(100),
            exit_code: None,
        },
        JobSummary {
            id: JobId(7),
            kind: JobKind::Raw,
            label: "Raw bitbake -s".into(),
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: Some(0),
        },
    ];

    let mut app = yoctui_model::App::new(16, 4096);
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: None,
        completed_tasks: 10,
        warnings: 0,
        errors: 0,
    });
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot);

    let rows = app.job_history_rows();
    assert_eq!(
        rows.len(),
        3,
        "a running daemon build must not hide a prior retained completion"
    );
    assert!(matches!(
        rows[0],
        yoctui_model::JobHistoryRowRef::Daemon(job)
            if job.id == 8
                && job.kind == yoctui_model::ClientDaemonJobKind::BitBakeBuild
                && job.progress_current == Some(59)
    ));
    assert!(matches!(
        rows[1],
        yoctui_model::JobHistoryRowRef::Daemon(job)
            if job.id == 7 && job.exit_code == Some(0)
    ));
    assert!(matches!(
        rows[2],
        yoctui_model::JobHistoryRowRef::Build(record)
            if record.target.as_deref() == Some("core-image-minimal")
    ));
    assert_eq!(app.job_summary().active, 1);
    assert_eq!(app.job_summary().recent_completed, 1);
}
