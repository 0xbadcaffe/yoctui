use crate::{
    CapabilityAvailabilitySummary, CapabilityEvidence, CapabilityId, CapabilityImplementation,
    CapabilityReason, CapabilityState, ClientReplicaStatus, DaemonCompatibilitySnapshot,
    EnvironmentOperatingMode, WorkspaceAvailability, WorkspaceAvailabilityState,
    WorkspaceCompatibilityState, WorkspaceEffectRequirement, YoctoEnvironmentIdentity,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiActionAvailability {
    pub state: WorkspaceAvailabilityState,
    pub enabled: bool,
    pub reasons: Vec<String>,
    pub implementations: Vec<(CapabilityId, String)>,
}

impl From<WorkspaceAvailability> for CompatibilityUiActionAvailability {
    fn from(availability: WorkspaceAvailability) -> Self {
        Self {
            state: availability.state,
            enabled: availability.is_enabled(),
            reasons: availability
                .issues
                .into_iter()
                .map(|issue| issue.reason)
                .collect(),
            implementations: availability.implementations,
        }
    }
}

pub fn compatibility_ui_action_availability(
    compatibility: &WorkspaceCompatibilityState,
    requirement: &WorkspaceEffectRequirement,
) -> CompatibilityUiActionAvailability {
    compatibility.availability(requirement).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthoritativeValue, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, IdentityAuthority,
    };
    use std::collections::BTreeMap;

    fn reason(code: &str, message: &str, requirement: Option<&str>) -> CapabilityReason {
        CapabilityReason::new(code, message, requirement.map(str::to_owned)).unwrap()
    }

    fn evidence(outcome: CapabilityEvidenceOutcome, subject: &str) -> CapabilityEvidence {
        CapabilityEvidence {
            kind: CapabilityEvidenceKind::DirectProbe,
            outcome,
            subject: subject.into(),
            detail: format!("{subject} fixture evidence"),
            argv: vec![subject.into(), "--help".into()],
        }
    }

    fn authority(generation: u64) -> DaemonCompatibilitySnapshot {
        let records = vec![
            CapabilityRecord {
                id: CapabilityId::BitBakeBuild,
                state: CapabilityState::Available,
                evidence: vec![evidence(CapabilityEvidenceOutcome::Positive, "bitbake")],
            },
            CapabilityRecord {
                id: CapabilityId::BitBakeGetVar,
                state: CapabilityState::AvailableWithLimitations {
                    reason: reason(
                        "compatibility.fallback",
                        "Native getvar is absent; the environment dump fallback is selected.",
                        Some("bitbake -e"),
                    ),
                    limitations: vec!["The fallback parses a complete environment dump.".into()],
                },
                evidence: vec![evidence(CapabilityEvidenceOutcome::Positive, "bitbake -e")],
            },
            CapabilityRecord {
                id: CapabilityId::DevtoolUpgrade,
                state: CapabilityState::Unavailable {
                    reason: reason(
                        "probe.subcommand_absent",
                        "Current Devtool does not expose the upgrade subcommand.",
                        Some("devtool upgrade"),
                    ),
                },
                evidence: vec![evidence(CapabilityEvidenceOutcome::Negative, "devtool")],
            },
            CapabilityRecord {
                id: CapabilityId::ResultTool,
                state: CapabilityState::Unknown {
                    reason: reason(
                        "probe.timed_out",
                        "The resulttool probe timed out.",
                        Some("resulttool --help"),
                    ),
                },
                evidence: vec![evidence(
                    CapabilityEvidenceOutcome::Inconclusive,
                    "resulttool",
                )],
            },
            CapabilityRecord {
                id: CapabilityId::GitArchive,
                state: CapabilityState::Unsupported {
                    reason: reason(
                        "yoctui.not_implemented",
                        "Yoctui does not maintain this environment adapter.",
                        None,
                    ),
                },
                evidence: Vec::new(),
            },
        ];
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        "/work/poky/build".into(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    bitbake_version: AuthoritativeValue::detected(
                        "2.18.0".into(),
                        IdentityAuthority::BitBakeVersionProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: records,
            },
            implementations: BTreeMap::from([
                (
                    CapabilityId::BitBakeBuild,
                    CapabilityImplementation {
                        id: "bitbake.build.command".into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                ),
                (
                    CapabilityId::BitBakeGetVar,
                    CapabilityImplementation {
                        id: "bitbake.getvar.environment-fallback".into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                ),
            ]),
        }
        .normalize()
        .unwrap()
    }

    fn state_with(authority: DaemonCompatibilitySnapshot) -> WorkspaceCompatibilityState {
        let mut state = WorkspaceCompatibilityState::default();
        state.install(authority).unwrap();
        state
    }

    #[test]
    fn compatibility_ui_model_projects_identity_summary_states_reasons_and_evidence() {
        let compatibility = state_with(authority(1));
        let mut state = CompatibilityUiState::default();
        state.reconcile(compatibility.authority());
        let projection = state.project(&compatibility, ClientReplicaStatus::Current);

        assert_eq!(
            projection.authority,
            CompatibilityUiAuthorityStatus::Current {
                generation: 1,
                mode: EnvironmentOperatingMode::Degraded,
            }
        );
        assert_eq!(
            projection.summary,
            CapabilityAvailabilitySummary {
                available: 1,
                limited: 1,
                unavailable: 1,
                unknown: 1,
                unsupported: 1,
            }
        );
        assert_eq!(projection.total_capabilities, 5);
        assert_eq!(projection.rows.len(), 5);
        assert_eq!(
            projection
                .environment
                .as_ref()
                .unwrap()
                .bitbake_version
                .value()
                .map(String::as_str),
            Some("2.18.0")
        );
        let limited = projection
            .rows
            .iter()
            .find(|row| row.id == CapabilityId::BitBakeGetVar)
            .unwrap();
        assert_eq!(limited.state, CompatibilityUiCapabilityState::Limited);
        assert_eq!(
            limited.reason.as_ref().unwrap().code.as_str(),
            "compatibility.fallback"
        );
        assert_eq!(limited.limitations.len(), 1);
        assert_eq!(
            limited.implementation.as_ref().unwrap().id,
            "bitbake.getvar.environment-fallback"
        );
        assert_eq!(limited.evidence[0].argv, ["bitbake -e", "--help"]);
    }

    #[test]
    fn compatibility_ui_model_absent_authority_is_explicit_for_every_replica_state() {
        let compatibility = WorkspaceCompatibilityState::default();
        let state = CompatibilityUiState::default();
        for (replica, expected) in [
            (ClientReplicaStatus::Disconnected, "disconnected"),
            (ClientReplicaStatus::Synchronizing, "synchronizing"),
            (ClientReplicaStatus::Current, "no authoritative"),
            (ClientReplicaStatus::Stale, "stale"),
        ] {
            let projection = state.project(&compatibility, replica);
            let CompatibilityUiAuthorityStatus::Unavailable { reason } = projection.authority
            else {
                panic!("absent authority must not project as current");
            };
            assert!(reason.contains(expected));
            assert!(projection.environment.is_none());
            assert!(projection.rows.is_empty());
            assert_eq!(projection.summary, CapabilityAvailabilitySummary::default());
        }
    }

    #[test]
    fn compatibility_ui_model_filter_search_and_selection_reconcile_by_stable_id() {
        let first = authority(1);
        let compatibility = state_with(first.clone());
        let mut state = CompatibilityUiState::default();
        state.reconcile(compatibility.authority());
        state.set_filter(CompatibilityUiFilter::Attention, compatibility.authority());
        assert_eq!(
            state
                .project(&compatibility, ClientReplicaStatus::Current)
                .rows
                .len(),
            2
        );
        state.select(1, compatibility.authority());
        let selected = state.selected().unwrap();
        state.begin_search();
        for character in selected.as_str().chars() {
            assert!(state.append_query(character, compatibility.authority()));
        }
        assert_eq!(
            state
                .project(&compatibility, ClientReplicaStatus::Current)
                .selected,
            Some(selected)
        );

        let replacement = state_with(authority(2));
        state.reconcile(replacement.authority());
        assert_eq!(state.selected(), Some(selected));
        while !state.query.is_empty() {
            state.backspace_query(replacement.authority());
        }
        state.set_filter(CompatibilityUiFilter::Unavailable, replacement.authority());
        assert_eq!(state.selected(), Some(CapabilityId::DevtoolUpgrade));
        state.reconcile(None);
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn compatibility_ui_model_query_is_bounded_and_rejects_control_input() {
        let compatibility = state_with(authority(1));
        let mut state = CompatibilityUiState::default();
        assert!(!state.append_query('\n', compatibility.authority()));
        for _ in 0..MAX_COMPATIBILITY_UI_QUERY_BYTES {
            assert!(state.append_query('a', compatibility.authority()));
        }
        assert!(!state.append_query('b', compatibility.authority()));
        assert_eq!(state.query.len(), MAX_COMPATIBILITY_UI_QUERY_BYTES);
    }

    #[test]
    fn compatibility_ui_model_action_availability_preserves_exact_workspace_projection() {
        let compatibility = state_with(authority(1));
        let available = compatibility_ui_action_availability(
            &compatibility,
            &WorkspaceEffectRequirement::Capabilities {
                all: vec![CapabilityId::BitBakeGetVar],
                any: Vec::new(),
            },
        );
        assert!(available.enabled);
        assert_eq!(
            available.state,
            WorkspaceAvailabilityState::AvailableWithLimitations
        );
        assert!(available.reasons[0].contains("fallback"));
        assert_eq!(available.implementations[0].0, CapabilityId::BitBakeGetVar);

        let denied = compatibility_ui_action_availability(
            &compatibility,
            &WorkspaceEffectRequirement::Capabilities {
                all: vec![CapabilityId::DevtoolUpgrade],
                any: Vec::new(),
            },
        );
        assert!(!denied.enabled);
        assert_eq!(denied.state, WorkspaceAvailabilityState::Unavailable);
        assert_eq!(
            denied.reasons,
            ["Current Devtool does not expose the upgrade subcommand."]
        );
    }
}
