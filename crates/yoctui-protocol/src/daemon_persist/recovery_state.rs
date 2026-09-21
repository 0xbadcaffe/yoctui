#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPersistPaths {
    pub directory: PathBuf,
    pub state: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonPersistedState {
    pub schema_version: u32,
    pub saved_unix_ms: u64,
    pub previous_daemon_instance_id: DaemonInstanceId,
    pub previous_boot_id: String,
    pub last_sequence: u64,
    pub last_generation: u64,
    pub workspace: Option<WorkspaceIdentity>,
    pub project_profile: ProjectProfileSummary,
    pub bitbake: PersistedBitBakeMetadata,
    pub job_history: Vec<JobSummary>,
    #[serde(default)]
    pub raw_executions: Vec<RawExecutionSnapshotData>,
    #[serde(default)]
    pub raw_history: Vec<RawHistoryRecordData>,
    pub terminal_sessions: Vec<PersistedTerminalSession>,
    pub recent_logs: Vec<LogRecord>,
    pub recovery_warnings: Vec<String>,
    pub client_layouts: Vec<PersistedClientLayout>,
    pub preferences: PersistedPreferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedBitBakeMetadata {
    pub version: Option<String>,
    pub capabilities: Vec<BitBakeCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedTerminalSession {
    pub id: u64,
    pub name: String,
    pub kind: PtyKind,
    pub cwd: String,
    pub previous_lifecycle: LifecycleState,
    pub dimensions: TerminalDimensions,
    pub exit_code: Option<i32>,
    pub restartable: bool,
    /// Always false: process identity and liveness are never serialized.
    pub live_process_persisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedClientLayout {
    pub client_key: String,
    pub layout_revision: u64,
    pub session_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedPreferences {
    pub theme: Option<String>,
    pub prefix_key: Option<String>,
    pub recent_logs_enabled: bool,
}

impl DaemonPersistedState {
    pub fn capture(
        snapshot: &DaemonSnapshot,
        saved_unix_ms: u64,
        previous_boot_id: String,
        client_layouts: Vec<PersistedClientLayout>,
        preferences: PersistedPreferences,
    ) -> Self {
        let recent_logs = if preferences.recent_logs_enabled {
            snapshot.recent_logs.clone()
        } else {
            Vec::new()
        };
        Self {
            schema_version: DAEMON_PERSIST_SCHEMA_VERSION,
            saved_unix_ms,
            previous_daemon_instance_id: snapshot.daemon_instance_id,
            previous_boot_id,
            last_sequence: snapshot.sequence,
            last_generation: snapshot.generation,
            workspace: snapshot.workspace.clone(),
            project_profile: snapshot.project_profile.clone(),
            bitbake: PersistedBitBakeMetadata {
                version: snapshot.bitbake.version.clone(),
                capabilities: snapshot.bitbake.capabilities.clone(),
            },
            job_history: snapshot.jobs.clone(),
            raw_executions: snapshot
                .raw_executions
                .iter()
                .cloned()
                .map(sanitize_raw_execution_for_persistence)
                .collect(),
            raw_history: snapshot.raw_history.clone(),
            terminal_sessions: snapshot
                .pty_sessions
                .iter()
                .map(|session| PersistedTerminalSession {
                    id: session.id.0,
                    name: session.name.clone(),
                    kind: session.kind,
                    cwd: session.cwd.clone(),
                    previous_lifecycle: session.lifecycle,
                    dimensions: session.dimensions,
                    exit_code: session.exit_code,
                    restartable: session.restartable,
                    live_process_persisted: false,
                })
                .collect(),
            recent_logs,
            recovery_warnings: snapshot.recovery_warnings.clone(),
            client_layouts,
            preferences,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonRecoveryReport {
    pub boundary: DaemonRecoveryBoundary,
    pub previous_boot_changed: bool,
    pub lost_jobs: usize,
    pub lost_raw_executions: usize,
    pub lost_terminal_sessions: usize,
    pub bitbake_reconnect_recommended: bool,
    pub terminal_relaunch_intents: Vec<TerminalRelaunchIntent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonRecoveryBoundary {
    DaemonRestart,
    HostReboot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalRelaunchIntent {
    pub previous_session_id: u64,
    pub name: String,
    pub kind: PtyKind,
    pub cwd: String,
    pub dimensions: TerminalDimensions,
}

fn sanitize_raw_execution_for_persistence(
    mut execution: RawExecutionSnapshotData,
) -> RawExecutionSnapshotData {
    for output in [&mut execution.stdout, &mut execution.stderr] {
        output.dropped_bytes = output.dropped_bytes.saturating_add(output.retained_bytes);
        output.dropped_lines = output.dropped_lines.saturating_add(output.retained_lines);
        output.chunks.clear();
        output.retained_bytes = 0;
        output.retained_lines = 0;
    }
    execution.attachment = RawAttachmentData::Detached;
    execution
}

pub fn recover_persisted_snapshot(
    mut current: DaemonSnapshot,
    persisted: &DaemonPersistedState,
    current_boot_id: &str,
) -> (DaemonSnapshot, DaemonRecoveryReport) {
    let previous_boot_changed = persisted.previous_boot_id != current_boot_id;
    let mut lost_jobs = 0;
    let mut lost_raw_executions = 0;
    let mut lost_terminal_sessions = 0;
    let mut terminal_relaunch_intents = Vec::new();
    if current.workspace.is_none() {
        current.workspace.clone_from(&persisted.workspace);
    }
    current
        .project_profile
        .clone_from(&persisted.project_profile);
    let bitbake_reconnect_recommended =
        persisted.bitbake.version.is_some() || !persisted.bitbake.capabilities.is_empty();
    current.bitbake = BitBakeState {
        lifecycle: LifecycleState::Disconnected,
        version: persisted.bitbake.version.clone(),
        capabilities: persisted.bitbake.capabilities.clone(),
        diagnostic: bitbake_reconnect_recommended
            .then(|| "persisted BitBake identity requires a supported reconnect probe".into()),
    };
    current.jobs = persisted
        .job_history
        .iter()
        .cloned()
        .map(|mut job| {
            if !matches!(
                job.lifecycle,
                LifecycleState::Exited | LifecycleState::Failed | LifecycleState::Lost
            ) {
                job.lifecycle = LifecycleState::Lost;
                lost_jobs += 1;
            }
            job
        })
        .collect();
    current.raw_executions = persisted
        .raw_executions
        .iter()
        .cloned()
        .map(|mut execution| {
            execution.attachment = RawAttachmentData::Detached;
            if !matches!(execution.phase, RawExecutionPhaseData::Terminal(_)) {
                execution.phase = RawExecutionPhaseData::Terminal(RawExecutionOutcomeData::Lost);
                execution.result = Some(RawExecutionResultData {
                    schema_version: crate::daemon::RAW_EXECUTION_SCHEMA_VERSION,
                    outcome: RawExecutionOutcomeData::Lost,
                    exit_code: None,
                    message: Some("Raw process ownership was lost when the daemon stopped".into()),
                    elapsed_ms: execution.elapsed_ms,
                    durable_reference: None,
                });
                execution.sequence = execution.sequence.saturating_add(1);
                execution.generation = execution.generation.saturating_add(1);
                lost_raw_executions += 1;
            }
            execution
        })
        .collect();
    current.raw_history.clone_from(&persisted.raw_history);
    let terminal_records = current
        .raw_executions
        .iter()
        .filter_map(|execution| RawHistoryRecordData::from_terminal(execution).ok())
        .collect::<Vec<_>>();
    for record in terminal_records {
        crate::daemon::remember_raw_history(&mut current, record);
    }
    current.pty_sessions = persisted
        .terminal_sessions
        .iter()
        .map(|session| {
            let lifecycle = if matches!(
                session.previous_lifecycle,
                LifecycleState::Exited | LifecycleState::Failed | LifecycleState::Lost
            ) {
                session.previous_lifecycle
            } else {
                lost_terminal_sessions += 1;
                if session.restartable {
                    terminal_relaunch_intents.push(TerminalRelaunchIntent {
                        previous_session_id: session.id,
                        name: session.name.clone(),
                        kind: session.kind,
                        cwd: session.cwd.clone(),
                        dimensions: session.dimensions,
                    });
                }
                LifecycleState::Lost
            };
            crate::daemon::PtySessionSummary {
                id: crate::daemon::PtySessionId(session.id),
                name: session.name.clone(),
                kind: session.kind,
                cwd: session.cwd.clone(),
                lifecycle,
                dimensions: session.dimensions,
                writer: None,
                writer_epoch: 0,
                viewers: 0,
                exit_code: session.exit_code,
                restartable: session.restartable,
            }
        })
        .collect();
    current.recent_logs.clone_from(&persisted.recent_logs);
    let current_warnings = std::mem::take(&mut current.recovery_warnings);
    current
        .recovery_warnings
        .clone_from(&persisted.recovery_warnings);
    for warning in current_warnings {
        if !current.recovery_warnings.contains(&warning) {
            current.recovery_warnings.push(warning);
        }
    }
    current.recovery_warnings.push(if previous_boot_changed {
        "host boot changed; all previously nonterminal jobs and PTYs were classified Lost".into()
    } else {
        "daemon instance restarted; all unrecoverable nonterminal jobs and PTYs were classified Lost"
            .into()
    });
    current.clients.clear();
    (
        current,
        DaemonRecoveryReport {
            boundary: if previous_boot_changed {
                DaemonRecoveryBoundary::HostReboot
            } else {
                DaemonRecoveryBoundary::DaemonRestart
            },
            previous_boot_changed,
            lost_jobs,
            lost_raw_executions,
            lost_terminal_sessions,
            bitbake_reconnect_recommended,
            terminal_relaunch_intents,
        },
    )
}
