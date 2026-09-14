//! Raw fixtures.
use super::*;

pub(crate) fn raw_preview_fixture() -> yoctui_model::RawExecutionPreview {
    yoctui_model::RawExecutionPreview {
        catalog_version: 7,
        command: yoctui_model::RawCommandId::new("preview.run").unwrap(),
        executable: yoctui_model::RawExecutable::BitBake,
        arguments: vec![
            "-c".into(),
            "do_compile".into(),
            "".into(),
            "café value".into(),
        ],
        indexed_arguments: vec![
            yoctui_model::RawPreviewArgument {
                index: 0,
                value: "bitbake".into(),
                source: yoctui_model::RawPreviewArgumentSource::Executable,
            },
            yoctui_model::RawPreviewArgument {
                index: 1,
                value: "-c".into(),
                source: yoctui_model::RawPreviewArgumentSource::Template { index: 0 },
            },
            yoctui_model::RawPreviewArgument {
                index: 2,
                value: "do_compile".into(),
                source: yoctui_model::RawPreviewArgumentSource::Template { index: 1 },
            },
            yoctui_model::RawPreviewArgument {
                index: 3,
                value: "".into(),
                source: yoctui_model::RawPreviewArgumentSource::Template { index: 2 },
            },
            yoctui_model::RawPreviewArgument {
                index: 4,
                value: "café value".into(),
                source: yoctui_model::RawPreviewArgumentSource::Additional { index: 0 },
            },
        ],
        capability_generation: 9,
        environment: yoctui_model::YoctoEnvironmentIdentity::default(),
        build_directory: "/work/build".into(),
        implementations: vec![(
            yoctui_model::CapabilityId::BitBakeRawCli,
            "bitbake.raw.argv".into(),
        )],
        capability_issues: vec![yoctui_model::RawCapabilityIssue {
            capability: Some(yoctui_model::CapabilityId::BitBakeRawCli),
            reason: "Direct probe is limited.".into(),
            limitations: vec!["Exact fixture limitation.".into()],
        }],
        interaction: yoctui_model::RawInteractionMode::NoninteractiveJob,
        safety: yoctui_model::RawSafetyClass::Build,
        limitations: vec!["Exact fixture limitation.".into()],
    }
}

pub(crate) fn raw_output_app(interaction: yoctui_model::RawInteractionMode) -> App {
    let request = yoctui_model::RawConfirmedExecutionRequest {
        id: yoctui_model::RawRequestId::new(match interaction {
            yoctui_model::RawInteractionMode::NoninteractiveJob => "raw-request:ui-output-job",
            yoctui_model::RawInteractionMode::InteractivePty => "raw-request:ui-output-pty",
        })
        .unwrap(),
        catalog_version: 1,
        command: yoctui_model::RawCommandId::new("output.fixture").unwrap(),
        parameters: std::collections::BTreeMap::new(),
        additional_arguments: Vec::new(),
        interaction,
        safety: yoctui_model::RawSafetyClass::Build,
        capability_generation: 9,
        build_directory: "/work/build".into(),
        preview_digest: yoctui_model::RawPreviewDigest([9; 32]),
    };
    let mut execution = yoctui_model::RawExecutionState::queued(
        request,
        yoctui_model::RawStreamId::new("raw-stream:ui-output-stdout").unwrap(),
        yoctui_model::RawStreamId::new("raw-stream:ui-output-stderr").unwrap(),
        10,
        yoctui_model::RawEventCursor::default(),
    )
    .unwrap();
    let mut apply = |kind| {
        let event = yoctui_model::RawExecutionEvent {
            request_id: execution.request.id.clone(),
            sequence: execution.cursor.sequence + 1,
            generation: execution.cursor.generation + 1,
            kind,
        };
        yoctui_model::reduce_raw_execution(&mut execution, event).unwrap();
    };
    apply(yoctui_model::RawExecutionEventKind::Starting {
        owner: match interaction {
            yoctui_model::RawInteractionMode::NoninteractiveJob => {
                yoctui_model::RawExecutionOwner::Job(
                    yoctui_model::RawJobId::new("raw-job:ui-output").unwrap(),
                )
            }
            yoctui_model::RawInteractionMode::InteractivePty => {
                yoctui_model::RawExecutionOwner::Pty(
                    yoctui_model::RawSessionId::new("raw-session:daemon-3").unwrap(),
                )
            }
        },
    });
    apply(yoctui_model::RawExecutionEventKind::Running {
        started_unix_ms: 20,
    });
    if interaction == yoctui_model::RawInteractionMode::NoninteractiveJob {
        apply(yoctui_model::RawExecutionEventKind::Output {
            chunk: yoctui_model::RawOutputChunk {
                stream_id: yoctui_model::RawStreamId::new("raw-stream:ui-output-stdout").unwrap(),
                stream: yoctui_model::RawOutputStream::Stdout,
                sequence: 1,
                text: "first line\nmatching café output".into(),
                truncated_bytes: 4,
                dropped_lines: 1,
            },
        });
        apply(yoctui_model::RawExecutionEventKind::Output {
            chunk: yoctui_model::RawOutputChunk {
                stream_id: yoctui_model::RawStreamId::new("raw-stream:ui-output-stderr").unwrap(),
                stream: yoctui_model::RawOutputStream::Stderr,
                sequence: 1,
                text: "warning output".into(),
                truncated_bytes: 0,
                dropped_lines: 0,
            },
        });
    }
    let request_id = execution.request.id.clone();
    let mut app = App::new(16, 4096);
    app.screen = Screen::RawMode;
    app.focus = FocusTarget::Workspace;
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Execution;
    app.raw_mode.execution = Some(execution.request.command.clone());
    app.raw_mode.output.request = Some(request_id.clone());
    app.raw_mode.execution_states.insert(request_id, execution);
    if interaction == yoctui_model::RawInteractionMode::InteractivePty {
        let pty_id = 6 << 60 | 3;
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id: pty_id,
                name: "Raw output fixture".into(),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            });
        app.daemon
            .pty_screens
            .push(yoctui_model::ClientDaemonPtyScreen {
                session_id: pty_id,
                columns: 80,
                rows_count: 24,
                cursor_column: 6,
                cursor_row: 1,
                cursor_hidden: false,
                scrollback_offset: 0,
                rows: vec!["interactive prompt".into(), "typed terminal row".into()],
                cells: Vec::new(),
                scrollback_lines: 2,
                dropped_line_feeds_lower_bound: 0,
            });
    }
    app
}

pub(crate) fn raw_command_list_app() -> App {
    let evidence = |outcome, subject: &str| yoctui_model::CapabilityEvidence {
        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
        outcome,
        subject: subject.into(),
        detail: format!("Raw command-list evidence for {subject}."),
        argv: vec!["bitbake".into(), "--help".into()],
    };
    let reason = |code: &str, message: &str| {
        yoctui_model::CapabilityReason::new(code, message, None).unwrap()
    };
    let authority = yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: yoctui_model::CapabilitySnapshot {
                generation: 19,
                environment: yoctui_model::YoctoEnvironmentIdentity {
                    build_directory: yoctui_model::AuthoritativeValue::detected(
                        "/work/build".into(),
                        yoctui_model::IdentityAuthority::InitializedEnvironment,
                    ),
                    ..yoctui_model::YoctoEnvironmentIdentity::default()
                },
                capabilities: vec![
                    yoctui_model::CapabilityRecord {
                        id: yoctui_model::CapabilityId::BitBakeRawCli,
                        state: yoctui_model::CapabilityState::Available,
                        evidence: vec![evidence(
                            yoctui_model::CapabilityEvidenceOutcome::Positive,
                            "bitbake.raw.cli",
                        )],
                    },
                    yoctui_model::CapabilityRecord {
                        id: yoctui_model::CapabilityId::BitBakeRawContinue,
                        state: yoctui_model::CapabilityState::AvailableWithLimitations {
                            reason: reason(
                                "raw.continue.limited",
                                "Continue is available with a fixture limitation and Unicode detail δοκιμή 界界界界界界界界.",
                            ),
                            limitations: vec!["Fixture limitation.".into()],
                        },
                        evidence: vec![evidence(
                            yoctui_model::CapabilityEvidenceOutcome::Positive,
                            "bitbake.raw.continue",
                        )],
                    },
                    yoctui_model::CapabilityRecord {
                        id: yoctui_model::CapabilityId::BitBakeRawShowVersions,
                        state: yoctui_model::CapabilityState::Unavailable {
                            reason: reason(
                                "raw.show_versions.absent",
                                "Show versions is unavailable.",
                            ),
                        },
                        evidence: vec![evidence(
                            yoctui_model::CapabilityEvidenceOutcome::Negative,
                            "bitbake.raw.show_versions",
                        )],
                    },
                    yoctui_model::CapabilityRecord {
                        id: yoctui_model::CapabilityId::BitBakeRawUi,
                        state: yoctui_model::CapabilityState::Unknown {
                            reason: reason("raw.ui.unknown", "UI support is unknown."),
                        },
                        evidence: vec![evidence(
                            yoctui_model::CapabilityEvidenceOutcome::Inconclusive,
                            "bitbake.raw.ui",
                        )],
                    },
                ],
            },
            implementations: std::collections::BTreeMap::from([
                (
                    yoctui_model::CapabilityId::BitBakeRawCli,
                    yoctui_model::CapabilityImplementation {
                        id: "bitbake.raw.argv".into(),
                        kind: yoctui_model::CapabilityImplementationKind::Command,
                    },
                ),
                (
                    yoctui_model::CapabilityId::BitBakeRawContinue,
                    yoctui_model::CapabilityImplementation {
                        id: "bitbake.raw.continue.argv".into(),
                        kind: yoctui_model::CapabilityImplementationKind::Command,
                    },
                ),
            ]),
        }
        .normalize()
        .unwrap();
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/work/build".into());
    app.build.target = Some("busybox".into());
    yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();
    let _ = update(&mut app, Action::Open(Screen::RawMode));
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::SelectCategory { delta: 3 }),
    );
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::FocusCommands),
    );
    app
}

pub(crate) fn set_raw_command_query(app: &mut App, query: &str) {
    let _ = update(
        app,
        Action::RawMode(yoctui_model::RawModeAction::BeginSearch),
    );
    let _ = update(
        app,
        Action::RawMode(yoctui_model::RawModeAction::ClearSearch),
    );
    for character in query.chars() {
        let _ = update(
            app,
            Action::RawMode(yoctui_model::RawModeAction::AppendSearch(character)),
        );
    }
    let _ = update(
        app,
        Action::RawMode(yoctui_model::RawModeAction::FinishSearch),
    );
}

pub(crate) fn rendered_raw_preview(width: u16, height: u16) -> String {
    let preview = raw_preview_fixture();
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| render_raw_execution_preview(frame, &preview, frame.area()))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

pub(crate) fn ux_terminal_render_fixture() -> App {
    let mut app = App::new(16, 4_096);
    app.screen = Screen::TerminalSessions;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.client_id = Some([1; 16]);
    for (id, name, writer, kind) in [
        (
            1,
            "shell",
            Some([1; 16]),
            yoctui_model::ClientDaemonPtyKind::BuildShell,
        ),
        (
            2,
            "devshell:busybox",
            Some([2; 16]),
            yoctui_model::ClientDaemonPtyKind::Devshell,
        ),
    ] {
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id,
                name: name.into(),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 2,
            });
        app.daemon
            .pty_details
            .push(yoctui_model::ClientDaemonPtyDetails {
                id,
                kind,
                cwd: "/work/poky/build".into(),
                columns: 120,
                rows: 30,
                writer,
                writer_epoch: 4,
                exit_code: None,
                restartable: true,
            });
        app.daemon
            .pty_screens
            .push(yoctui_model::ClientDaemonPtyScreen {
                session_id: id,
                columns: 120,
                rows_count: 30,
                cursor_column: 2,
                cursor_row: 1,
                cursor_hidden: false,
                scrollback_offset: 0,
                rows: vec![format!("{name}: bounded output"), "$ ".into()],
                cells: Vec::new(),
                scrollback_lines: 4_096,
                dropped_line_feeds_lower_bound: 312,
            });
    }
    app.pane_layout
        .split(app.pane_layout.focused, yoctui_model::SplitAxis::Vertical)
        .unwrap();
    app
}
