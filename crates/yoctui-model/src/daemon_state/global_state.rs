pub const DEFAULT_DAEMON_LOG_LIMIT: usize = 10_000;
pub const DEFAULT_DAEMON_ERROR_LIMIT: usize = 1_000;
pub const DEFAULT_DAEMON_HISTORY_LIMIT: usize = 1_000;
pub const MAX_DAEMON_COLLECTION_LIMIT: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DaemonModelInstanceId(pub [u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonRevision {
    pub instance_id: DaemonModelInstanceId,
    pub sequence: u64,
    pub generation: u64,
}

impl DaemonRevision {
    pub fn advance(&mut self) -> Result<(), DaemonStateError> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(DaemonStateError::RevisionExhausted)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(DaemonStateError::RevisionExhausted)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonStateLimits {
    pub logs: usize,
    pub errors: usize,
    pub history: usize,
}

impl Default for DaemonStateLimits {
    fn default() -> Self {
        Self {
            logs: DEFAULT_DAEMON_LOG_LIMIT,
            errors: DEFAULT_DAEMON_ERROR_LIMIT,
            history: DEFAULT_DAEMON_HISTORY_LIMIT,
        }
    }
}

impl DaemonStateLimits {
    pub fn validate(self) -> Result<Self, DaemonStateError> {
        for (collection, limit) in [
            ("logs", self.logs),
            ("errors", self.errors),
            ("history", self.history),
        ] {
            if limit == 0 || limit > MAX_DAEMON_COLLECTION_LIMIT {
                return Err(DaemonStateError::InvalidLimit { collection, limit });
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonBitBakeLifecycle {
    Disconnected,
    Connecting,
    Connected,
    Stopping,
    Failed,
    Recovering,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonBitBakeState {
    pub lifecycle: DaemonBitBakeLifecycle,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCompatibilitySnapshot {
    pub snapshot: CapabilitySnapshot,
    pub implementations: BTreeMap<CapabilityId, CapabilityImplementation>,
}

impl DaemonCompatibilitySnapshot {
    pub fn normalize(mut self) -> Result<Self, DaemonStateError> {
        self.snapshot = self.snapshot.normalize()?;
        for capability in &self.snapshot.capabilities {
            let implementation = self.implementations.get(&capability.id);
            if capability.state.is_enabled() != implementation.is_some() {
                return Err(DaemonStateError::CompatibilityImplementationMismatch(
                    capability.id,
                ));
            }
        }
        if self
            .implementations
            .keys()
            .any(|id| self.snapshot.capability(*id).is_none())
        {
            return Err(DaemonStateError::CompatibilityUnknownImplementation);
        }
        Ok(self)
    }
}

impl Default for DaemonBitBakeState {
    fn default() -> Self {
        Self {
            lifecycle: DaemonBitBakeLifecycle::Disconnected,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonRecoveryState {
    CleanStart,
    Recovering,
    Recovered,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonTelemetry {
    pub uptime_seconds: u64,
    pub connected_clients: usize,
    pub active_jobs: usize,
    pub pty_sessions: usize,
    pub queue_depth: usize,
    pub memory_bytes: Option<u64>,
    pub recovery: DaemonRecoveryState,
}

impl Default for DaemonTelemetry {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            connected_clients: 0,
            active_jobs: 0,
            pty_sessions: 0,
            queue_depth: 0,
            memory_bytes: None,
            recovery: DaemonRecoveryState::CleanStart,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonSessionMetadata {
    pub started_unix_ms: u64,
    pub boot_id: String,
    pub recovery: DaemonRecoveryState,
    pub recovery_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonGlobalState {
    pub revision: DaemonRevision,
    pub limits: DaemonStateLimits,
    pub workspace: Workspace,
    pub build_environment: BuildEnvironmentState,
    pub project_profile: ProjectProfileState,
    pub bitbake: DaemonBitBakeState,
    pub compatibility: Option<DaemonCompatibilitySnapshot>,
    pub session: DaemonSessionMetadata,
    pub recent_logs: VecDeque<String>,
    pub recent_errors: VecDeque<String>,
    pub task_history: VecDeque<String>,
    pub jobs: Option<DaemonJobState>,
}

impl DaemonGlobalState {
    pub fn new(
        instance_id: DaemonModelInstanceId,
        started_unix_ms: u64,
        boot_id: String,
        limits: DaemonStateLimits,
    ) -> Result<Self, DaemonStateError> {
        let limits = limits.validate()?;
        Ok(Self {
            revision: DaemonRevision {
                instance_id,
                sequence: 0,
                generation: 0,
            },
            limits,
            workspace: Workspace::default(),
            build_environment: BuildEnvironmentState::Unconfigured,
            project_profile: ProjectProfileState::NotLoaded,
            bitbake: DaemonBitBakeState::default(),
            compatibility: None,
            session: DaemonSessionMetadata {
                started_unix_ms,
                boot_id,
                recovery: DaemonRecoveryState::CleanStart,
                recovery_warnings: Vec::new(),
            },
            recent_logs: VecDeque::new(),
            recent_errors: VecDeque::new(),
            task_history: VecDeque::new(),
            jobs: None,
        })
    }

    pub fn mutate(
        &mut self,
        mutation: impl FnOnce(&mut Self),
    ) -> Result<DaemonRevision, DaemonStateError> {
        mutation(self);
        trim_front(&mut self.recent_logs, self.limits.logs);
        trim_front(&mut self.recent_errors, self.limits.errors);
        trim_front(&mut self.task_history, self.limits.history);
        self.revision.advance()?;
        Ok(self.revision)
    }
}

