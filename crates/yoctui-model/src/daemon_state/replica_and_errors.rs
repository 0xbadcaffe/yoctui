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
    #[error(transparent)]
    InvalidCompatibility(#[from] crate::CapabilityModelError),
    #[error("capability implementation does not match enabled state for {0}")]
    CompatibilityImplementationMismatch(CapabilityId),
    #[error("capability implementation references an absent snapshot capability")]
    CompatibilityUnknownImplementation,
    #[error("stale daemon compatibility generation: current {current}, received {received}")]
    StaleCompatibilityGeneration { current: u64, received: u64 },
}

