pub const MAX_COMPATIBILITY_UI_QUERY_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompatibilityUiFilter {
    #[default]
    All,
    Available,
    Limited,
    Unavailable,
    Attention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityUiCapabilityState {
    Available,
    Limited,
    Unavailable,
    Unknown,
    Unsupported,
}

impl CompatibilityUiCapabilityState {
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Available | Self::Limited)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityUiAuthorityStatus {
    Current {
        generation: u64,
        mode: EnvironmentOperatingMode,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiCapabilityRow {
    pub id: CapabilityId,
    pub state: CompatibilityUiCapabilityState,
    pub reason: Option<CapabilityReason>,
    pub limitations: Vec<String>,
    pub implementation: Option<CapabilityImplementation>,
    pub evidence: Vec<CapabilityEvidence>,
}

impl CompatibilityUiCapabilityRow {
    fn from_snapshot(authority: &DaemonCompatibilitySnapshot, id: CapabilityId) -> Option<Self> {
        let record = authority.snapshot.capability(id)?;
        let (state, reason, limitations) = match &record.state {
            CapabilityState::Available => {
                (CompatibilityUiCapabilityState::Available, None, Vec::new())
            }
            CapabilityState::AvailableWithLimitations {
                reason,
                limitations,
            } => (
                CompatibilityUiCapabilityState::Limited,
                Some(reason.clone()),
                limitations.clone(),
            ),
            CapabilityState::Unavailable { reason } => (
                CompatibilityUiCapabilityState::Unavailable,
                Some(reason.clone()),
                Vec::new(),
            ),
            CapabilityState::Unknown { reason } => (
                CompatibilityUiCapabilityState::Unknown,
                Some(reason.clone()),
                Vec::new(),
            ),
            CapabilityState::Unsupported { reason } => (
                CompatibilityUiCapabilityState::Unsupported,
                Some(reason.clone()),
                Vec::new(),
            ),
        };
        Some(Self {
            id,
            state,
            reason,
            limitations,
            implementation: authority.implementations.get(&id).cloned(),
            evidence: record.evidence.clone(),
        })
    }

    fn matches_filter(&self, filter: CompatibilityUiFilter) -> bool {
        match filter {
            CompatibilityUiFilter::All => true,
            CompatibilityUiFilter::Available => {
                self.state == CompatibilityUiCapabilityState::Available
            }
            CompatibilityUiFilter::Limited => self.state == CompatibilityUiCapabilityState::Limited,
            CompatibilityUiFilter::Unavailable => {
                self.state == CompatibilityUiCapabilityState::Unavailable
            }
            CompatibilityUiFilter::Attention => matches!(
                self.state,
                CompatibilityUiCapabilityState::Unknown
                    | CompatibilityUiCapabilityState::Unsupported
            ),
        }
    }

    fn matches_query(&self, normalized_query: &str) -> bool {
        normalized_query.is_empty()
            || self
                .search_fields()
                .any(|value| value.to_lowercase().contains(normalized_query))
    }

    fn search_fields(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str())
            .chain(self.reason.iter().flat_map(|reason| {
                std::iter::once(reason.message.as_str()).chain(reason.requirement.as_deref())
            }))
            .chain(
                self.implementation
                    .iter()
                    .map(|implementation| implementation.id.as_str()),
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiProjection {
    pub authority: CompatibilityUiAuthorityStatus,
    pub environment: Option<YoctoEnvironmentIdentity>,
    pub summary: CapabilityAvailabilitySummary,
    pub total_capabilities: usize,
    pub rows: Vec<CompatibilityUiCapabilityRow>,
    pub selected: Option<CapabilityId>,
}

impl CompatibilityUiProjection {
    pub fn selected_row(&self) -> Option<&CompatibilityUiCapabilityRow> {
        let selected = self.selected?;
        self.rows.iter().find(|row| row.id == selected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiState {
    pub filter: CompatibilityUiFilter,
    pub query: String,
    pub searching: bool,
    selected: Option<CapabilityId>,
    selection_hint: usize,
}

impl Default for CompatibilityUiState {
    fn default() -> Self {
        Self {
            filter: CompatibilityUiFilter::All,
            query: String::new(),
            searching: false,
            selected: None,
            selection_hint: 0,
        }
    }
}

impl CompatibilityUiState {
    pub const fn selected(&self) -> Option<CapabilityId> {
        self.selected
    }

    pub fn set_filter(
        &mut self,
        filter: CompatibilityUiFilter,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) {
        self.filter = filter;
        self.reconcile(authority);
    }

    pub fn begin_search(&mut self) {
        self.searching = true;
    }

    pub fn finish_search(&mut self) {
        self.searching = false;
    }

    pub fn append_query(
        &mut self,
        character: char,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> bool {
        if character.is_control()
            || self.query.len() + character.len_utf8() > MAX_COMPATIBILITY_UI_QUERY_BYTES
        {
            return false;
        }
        self.query.push(character);
        self.reconcile(authority);
        true
    }

    pub fn backspace_query(&mut self, authority: Option<&DaemonCompatibilitySnapshot>) {
        self.query.pop();
        self.reconcile(authority);
    }

    pub fn clear_query(&mut self, authority: Option<&DaemonCompatibilitySnapshot>) {
        self.query.clear();
        self.reconcile(authority);
    }

    pub fn select(&mut self, delta: isize, authority: Option<&DaemonCompatibilitySnapshot>) {
        let rows = self.filtered_rows(authority);
        if rows.is_empty() {
            self.selected = None;
            self.selection_hint = 0;
            return;
        }
        let current = self
            .selected
            .and_then(|selected| rows.iter().position(|row| row.id == selected))
            .unwrap_or_else(|| self.selection_hint.min(rows.len() - 1));
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(rows.len() - 1)
        };
        self.selection_hint = next;
        self.selected = Some(rows[next].id);
    }

    pub fn reconcile(&mut self, authority: Option<&DaemonCompatibilitySnapshot>) {
        let rows = self.filtered_rows(authority);
        if rows.is_empty() {
            self.selected = None;
            self.selection_hint = 0;
            return;
        }
        if let Some(index) = self
            .selected
            .and_then(|selected| rows.iter().position(|row| row.id == selected))
        {
            self.selection_hint = index;
            return;
        }
        let index = self.selection_hint.min(rows.len() - 1);
        self.selected = Some(rows[index].id);
        self.selection_hint = index;
    }

    pub fn project(
        &self,
        compatibility: &WorkspaceCompatibilityState,
        replica_status: ClientReplicaStatus,
    ) -> CompatibilityUiProjection {
        let Some(authority) = compatibility.authority() else {
            return CompatibilityUiProjection {
                authority: CompatibilityUiAuthorityStatus::Unavailable {
                    reason: unavailable_authority_reason(replica_status).into(),
                },
                environment: None,
                summary: CapabilityAvailabilitySummary::default(),
                total_capabilities: 0,
                rows: Vec::new(),
                selected: None,
            };
        };
        let rows = self.filtered_rows(Some(authority));
        let selected = self
            .selected
            .filter(|selected| rows.iter().any(|row| row.id == *selected))
            .or_else(|| rows.first().map(|row| row.id));
        CompatibilityUiProjection {
            authority: CompatibilityUiAuthorityStatus::Current {
                generation: authority.snapshot.generation,
                mode: authority.snapshot.operating_mode(),
            },
            environment: Some(authority.snapshot.environment.clone()),
            summary: authority.snapshot.availability_summary(),
            total_capabilities: authority.snapshot.capabilities.len(),
            rows,
            selected,
        }
    }

    fn filtered_rows(
        &self,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> Vec<CompatibilityUiCapabilityRow> {
        let Some(authority) = authority else {
            return Vec::new();
        };
        let query = self.query.to_lowercase();
        authority
            .snapshot
            .capabilities
            .iter()
            .filter_map(|record| CompatibilityUiCapabilityRow::from_snapshot(authority, record.id))
            .filter(|row| row.matches_filter(self.filter) && row.matches_query(&query))
            .collect()
    }
}

fn unavailable_authority_reason(status: ClientReplicaStatus) -> &'static str {
    match status {
        ClientReplicaStatus::Disconnected => {
            "Daemon is disconnected; no current environment capability snapshot is installed."
        }
        ClientReplicaStatus::Synchronizing => {
            "Daemon compatibility state is synchronizing; actions remain unknown until a current snapshot arrives."
        }
        ClientReplicaStatus::Current => {
            "The current daemon snapshot has no authoritative compatibility state for this environment."
        }
        ClientReplicaStatus::Stale => {
            "Daemon compatibility state is stale; actions remain unknown until resynchronization."
        }
    }
}

