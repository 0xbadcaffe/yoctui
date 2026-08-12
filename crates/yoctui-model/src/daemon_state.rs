use crate::{BuildEnvironmentState, FocusTarget, ProjectProfileState, Screen, Theme, Workspace};
use std::collections::VecDeque;
use thiserror::Error;

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
    pub session: DaemonSessionMetadata,
    pub recent_logs: VecDeque<String>,
    pub recent_errors: VecDeque<String>,
    pub task_history: VecDeque<String>,
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
            session: DaemonSessionMetadata {
                started_unix_ms,
                boot_id,
                recovery: DaemonRecoveryState::CleanStart,
                recovery_warnings: Vec::new(),
            },
            recent_logs: VecDeque::new(),
            recent_errors: VecDeque::new(),
            task_history: VecDeque::new(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientReplicaStatus {
    Disconnected,
    Synchronizing,
    Current,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonReplica {
    pub status: ClientReplicaStatus,
    pub state: Option<DaemonGlobalState>,
}

impl Default for ClientDaemonReplica {
    fn default() -> Self {
        Self {
            status: ClientReplicaStatus::Disconnected,
            state: None,
        }
    }
}

impl ClientDaemonReplica {
    pub fn begin_synchronization(&mut self) {
        self.status = ClientReplicaStatus::Synchronizing;
    }

    pub fn replace(&mut self, snapshot: DaemonGlobalState) {
        self.state = Some(snapshot);
        self.status = ClientReplicaStatus::Current;
    }

    pub fn mark_stale(&mut self) {
        self.status = ClientReplicaStatus::Stale;
    }

    pub fn disconnect(&mut self) {
        self.status = ClientReplicaStatus::Disconnected;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientPresentationState {
    pub screen: Screen,
    pub focus: FocusTarget,
    pub navigator_selection: usize,
    pub theme: Theme,
    pub pane_layout_revision: u64,
}

impl Default for ClientPresentationState {
    fn default() -> Self {
        Self {
            screen: Screen::Dashboard,
            focus: FocusTarget::Navigator,
            navigator_selection: 0,
            theme: Theme::default(),
            pane_layout_revision: 0,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DaemonStateError {
    #[error("invalid daemon {collection} limit {limit}")]
    InvalidLimit {
        collection: &'static str,
        limit: usize,
    },
    #[error("daemon state revision exhausted")]
    RevisionExhausted,
}

fn trim_front<T>(items: &mut VecDeque<T>, limit: usize) {
    while items.len() > limit {
        items.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_state_partition_keeps_global_authority_and_client_presentation_distinct() {
        let mut daemon = DaemonGlobalState::new(
            DaemonModelInstanceId([1; 16]),
            10,
            "boot-a".into(),
            DaemonStateLimits {
                logs: 2,
                errors: 2,
                history: 2,
            },
        )
        .unwrap();
        let revision = daemon
            .mutate(|state| {
                state.bitbake.lifecycle = DaemonBitBakeLifecycle::Connected;
                state.bitbake.version = Some("2.8.1".into());
                state
                    .recent_logs
                    .extend(["one".into(), "two".into(), "three".into()]);
            })
            .unwrap();
        assert_eq!(revision.sequence, 1);
        assert_eq!(revision.generation, 1);
        assert_eq!(
            daemon.recent_logs.iter().cloned().collect::<Vec<_>>(),
            ["two", "three"]
        );

        let mut replica = ClientDaemonReplica::default();
        replica.begin_synchronization();
        replica.replace(daemon.clone());
        let presentation = ClientPresentationState {
            screen: Screen::Layers,
            theme: Theme::WhiteClassic,
            ..ClientPresentationState::default()
        };
        assert_eq!(replica.state.as_ref().unwrap(), &daemon);
        assert_eq!(presentation.screen, Screen::Layers);
        assert_eq!(daemon.revision, revision);
        replica.mark_stale();
        assert_eq!(replica.status, ClientReplicaStatus::Stale);
        replica.disconnect();
        assert_eq!(replica.status, ClientReplicaStatus::Disconnected);
        assert_eq!(replica.state.as_ref().unwrap().revision, revision);
    }

    #[test]
    fn daemon_state_partition_rejects_unbounded_or_zero_collection_limits() {
        for limits in [
            DaemonStateLimits {
                logs: 0,
                ..DaemonStateLimits::default()
            },
            DaemonStateLimits {
                errors: MAX_DAEMON_COLLECTION_LIMIT + 1,
                ..DaemonStateLimits::default()
            },
        ] {
            assert!(matches!(
                DaemonGlobalState::new(DaemonModelInstanceId([0; 16]), 0, "boot".into(), limits),
                Err(DaemonStateError::InvalidLimit { .. })
            ));
        }
    }

    #[test]
    fn daemon_state_partition_fails_closed_when_revision_space_is_exhausted() {
        let mut revision = DaemonRevision {
            instance_id: DaemonModelInstanceId([0; 16]),
            sequence: u64::MAX,
            generation: 0,
        };
        assert_eq!(revision.advance(), Err(DaemonStateError::RevisionExhausted));
    }
}
