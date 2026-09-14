//! Daemon snapshot.
use super::*;

pub fn reduce_daemon_state(
    state: &mut yoctui_model::DaemonGlobalState,
    action: yoctui_model::DaemonStateAction,
) -> Result<yoctui_model::DaemonRevision, yoctui_model::DaemonStateError> {
    yoctui_model::update_daemon_state(state, action)
}

pub fn client_replica_from_daemon(
    state: &yoctui_model::DaemonGlobalState,
) -> yoctui_model::ClientDaemonReplica {
    let mut replica = yoctui_model::ClientDaemonReplica::default();
    replica.begin_synchronization();
    replica.replace(state.clone());
    replica
}

pub fn recover_daemon_model_metadata(
    state: &mut yoctui_model::DaemonGlobalState,
    persisted: &yoctui_protocol::daemon_persist::DaemonPersistedState,
    current_boot_id: &str,
) -> Result<(), yoctui_model::DaemonStateError> {
    if let Some(identity) = &persisted.workspace {
        let workspace = yoctui_model::Workspace {
            source_dir: Some(std::path::PathBuf::from(&identity.canonical_source)),
            build_dir: Some(std::path::PathBuf::from(&identity.canonical_build)),
            ..yoctui_model::Workspace::default()
        };
        reduce_daemon_state(
            state,
            yoctui_model::DaemonStateAction::ReplaceWorkspace(workspace),
        )?;
    }
    let mut warnings = persisted.recovery_warnings.clone();
    let profile = match &persisted.project_profile {
        yoctui_protocol::daemon::ProjectProfileSummary::NotLoaded => {
            yoctui_model::ProjectProfileState::NotLoaded
        }
        yoctui_protocol::daemon::ProjectProfileSummary::Absent => {
            yoctui_model::ProjectProfileState::Absent
        }
        yoctui_protocol::daemon::ProjectProfileSummary::Loaded { schema_version } => {
            warnings.push(format!(
                "project profile schema {schema_version} metadata was restored; contents must be reloaded and validated"
            ));
            yoctui_model::ProjectProfileState::NotLoaded
        }
        yoctui_protocol::daemon::ProjectProfileSummary::Invalid { message } => {
            yoctui_model::ProjectProfileState::Invalid(message.clone())
        }
    };
    reduce_daemon_state(
        state,
        yoctui_model::DaemonStateAction::ReplaceProjectProfile(profile),
    )?;
    let bitbake_reconnect_recommended =
        persisted.bitbake.version.is_some() || !persisted.bitbake.capabilities.is_empty();
    reduce_daemon_state(
        state,
        yoctui_model::DaemonStateAction::ReplaceBitBake(yoctui_model::DaemonBitBakeState {
            lifecycle: yoctui_model::DaemonBitBakeLifecycle::Disconnected,
            version: persisted.bitbake.version.clone(),
            capabilities: persisted
                .bitbake
                .capabilities
                .iter()
                .map(|capability| match capability {
                    yoctui_protocol::daemon::BitBakeCapability::WorkspaceInspection => {
                        "workspace_inspection"
                    }
                    yoctui_protocol::daemon::BitBakeCapability::RecipeInventory => {
                        "recipe_inventory"
                    }
                    yoctui_protocol::daemon::BitBakeCapability::LayerInventory => "layer_inventory",
                    yoctui_protocol::daemon::BitBakeCapability::BuildControl => "build_control",
                    yoctui_protocol::daemon::BitBakeCapability::Cancellation => "cancellation",
                    yoctui_protocol::daemon::BitBakeCapability::ServerRestart => "server_restart",
                    yoctui_protocol::daemon::BitBakeCapability::Unknown => "unknown",
                })
                .map(str::to_owned)
                .collect(),
            diagnostic: bitbake_reconnect_recommended
                .then(|| "persisted BitBake identity requires a supported reconnect probe".into()),
        }),
    )?;
    let boot_changed = persisted.previous_boot_id != current_boot_id;
    warnings.push(if boot_changed {
        "host boot identity changed; persisted process metadata cannot be live".into()
    } else {
        "daemon restarted; persisted process metadata cannot be assumed live".into()
    });
    reduce_daemon_state(
        state,
        yoctui_model::DaemonStateAction::ReplaceRecovery {
            state: if boot_changed {
                yoctui_model::DaemonRecoveryState::Degraded
            } else {
                yoctui_model::DaemonRecoveryState::Recovered
            },
            warnings,
        },
    )?;
    Ok(())
}

pub fn daemon_protocol_snapshot(
    state: &yoctui_model::DaemonGlobalState,
) -> yoctui_protocol::daemon::DaemonSnapshot {
    use yoctui_protocol::daemon::{
        BitBakeCapability, BitBakeState, ClientSummary, DaemonInstanceId, DaemonSnapshot,
        LifecycleState, LogRecord, LogSeverity, ProjectProfileSummary, PtyKind, PtySessionId,
        PtySessionSummary, TerminalDimensions, WorkspaceIdentity,
    };

    let workspace = match (&state.workspace.source_dir, &state.workspace.build_dir) {
        (Some(source), Some(build)) => {
            let source = source.to_string_lossy().into_owned();
            let build = build.to_string_lossy().into_owned();
            Some(WorkspaceIdentity {
                identity_hash: stable_workspace_hash(&source, &build),
                canonical_source: source,
                canonical_build: build,
            })
        }
        _ => None,
    };
    let project_profile = match &state.project_profile {
        yoctui_model::ProjectProfileState::NotLoaded => ProjectProfileSummary::NotLoaded,
        yoctui_model::ProjectProfileState::Absent => ProjectProfileSummary::Absent,
        yoctui_model::ProjectProfileState::Loaded(profile)
        | yoctui_model::ProjectProfileState::GenerationPreview(profile)
        | yoctui_model::ProjectProfileState::Generating(profile) => ProjectProfileSummary::Loaded {
            schema_version: profile.schema_version,
        },
        yoctui_model::ProjectProfileState::Invalid(message) => ProjectProfileSummary::Invalid {
            message: message.clone(),
        },
    };
    let bitbake = BitBakeState {
        lifecycle: daemon_bitbake_lifecycle(state.bitbake.lifecycle),
        version: state.bitbake.version.clone(),
        capabilities: state
            .bitbake
            .capabilities
            .iter()
            .map(|capability| match capability.as_str() {
                "workspace_inspection" => BitBakeCapability::WorkspaceInspection,
                "recipe_inventory" => BitBakeCapability::RecipeInventory,
                "layer_inventory" => BitBakeCapability::LayerInventory,
                "build_control" => BitBakeCapability::BuildControl,
                "cancellation" => BitBakeCapability::Cancellation,
                "server_restart" => BitBakeCapability::ServerRestart,
                _ => BitBakeCapability::Unknown,
            })
            .collect(),
        diagnostic: state.bitbake.diagnostic.clone(),
    };
    let jobs = state
        .jobs
        .as_ref()
        .map(|jobs| {
            jobs.background_jobs
                .jobs
                .iter()
                .map(|job| yoctui_protocol::daemon::JobSummary {
                    id: yoctui_protocol::daemon::JobId(job.id.0),
                    kind: daemon_job_kind(job.kind),
                    label: job.title.clone(),
                    lifecycle: daemon_job_lifecycle(job.status),
                    progress_current: match job.progress {
                        yoctui_model::BackgroundJobProgress::Indeterminate => None,
                        yoctui_model::BackgroundJobProgress::Percent(percent) => {
                            Some(u64::from(percent))
                        }
                        yoctui_model::BackgroundJobProgress::Units { completed, .. } => {
                            Some(completed)
                        }
                    },
                    progress_total: match job.progress {
                        yoctui_model::BackgroundJobProgress::Indeterminate => None,
                        yoctui_model::BackgroundJobProgress::Percent(_) => Some(100),
                        yoctui_model::BackgroundJobProgress::Units { total, .. } => Some(total),
                    },
                    exit_code: None,
                })
                .collect()
        })
        .unwrap_or_default();
    let pty_sessions = state
        .jobs
        .as_ref()
        .map(|jobs| {
            jobs.pty_sessions
                .iter()
                .map(|session| PtySessionSummary {
                    id: PtySessionId(session.id),
                    name: session.name.clone(),
                    kind: match session.kind.as_str() {
                        "build_shell" => PtyKind::BuildShell,
                        "source_shell" => PtyKind::SourceShell,
                        "layer_shell" => PtyKind::LayerShell,
                        "recipe_shell" => PtyKind::RecipeShell,
                        "devtool_shell" => PtyKind::DevtoolShell,
                        "devshell" => PtyKind::Devshell,
                        "menuconfig" => PtyKind::Menuconfig,
                        "sdk_shell" => PtyKind::SdkShell,
                        "native_shell" => PtyKind::NativeShell,
                        "qemu_console" => PtyKind::QemuConsole,
                        "ssh_console" => PtyKind::SshConsole,
                        _ => PtyKind::Utility,
                    },
                    cwd: String::new(),
                    lifecycle: match session.lifecycle {
                        yoctui_model::DaemonPtyLifecycle::Running => LifecycleState::Running,
                        yoctui_model::DaemonPtyLifecycle::Exited => LifecycleState::Exited,
                        yoctui_model::DaemonPtyLifecycle::Lost => LifecycleState::Lost,
                    },
                    dimensions: TerminalDimensions {
                        columns: 80,
                        rows: 24,
                    },
                    writer: None,
                    writer_epoch: 0,
                    viewers: 0,
                    exit_code: None,
                    restartable: session.restartable,
                })
                .collect()
        })
        .unwrap_or_default();
    let mut recent_logs: Vec<LogRecord> = state
        .recent_logs
        .iter()
        .map(|message| LogRecord {
            source: "daemon".into(),
            severity: LogSeverity::Info,
            message: message.clone(),
            unix_ms: 0,
            recipe: None,
            task: None,
            path: None,
            build: None,
        })
        .chain(state.recent_errors.iter().map(|message| LogRecord {
            source: "daemon".into(),
            severity: LogSeverity::Error,
            message: message.clone(),
            unix_ms: 0,
            recipe: None,
            task: None,
            path: None,
            build: None,
        }))
        .collect();
    if recent_logs.len() > state.limits.logs {
        recent_logs.drain(..recent_logs.len() - state.limits.logs);
    }

    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId(state.revision.instance_id.0),
        sequence: state.revision.sequence,
        generation: state.revision.generation,
        workspace,
        project_profile,
        bitbake,
        compatibility: state
            .compatibility
            .as_ref()
            .map(daemon_compatibility_protocol),
        jobs,
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions,
        pty_screens: Vec::new(),
        clients: Vec::<ClientSummary>::new(),
        recent_logs,
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: state.session.recovery_warnings.clone(),
    }
}
