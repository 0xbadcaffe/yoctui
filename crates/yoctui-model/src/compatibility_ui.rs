use crate::{
    CapabilityAvailabilitySummary, CapabilityEvidence, CapabilityId, CapabilityImplementation,
    CapabilityReason, CapabilityState, ClientReplicaStatus, CommandId, DaemonCompatibilitySnapshot,
    Dialog, Effect, EnvironmentOperatingMode, Screen, WorkspaceAvailability,
    WorkspaceAvailabilityState, WorkspaceCompatibilityState, WorkspaceEffectRequirement,
    YoctoEnvironmentIdentity, workspace_destination_requirement, workspace_dialog_requirement,
    workspace_effect_requirement, workspace_screen_destination,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiActionAvailability {
    pub state: WorkspaceAvailabilityState,
    pub enabled: bool,
    pub reasons: Vec<String>,
    pub limitations: Vec<String>,
    pub implementations: Vec<(CapabilityId, String)>,
}

/// Whether a visible UI surface performs only local work, remains reachable so
/// unavailable environment behavior can be inspected, or must be rejected
/// before it can prepare an environment-backed effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityUiActionActivation {
    ClientLocal,
    Inspectable,
    CapabilityGated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiActionDefinition {
    pub activation: CompatibilityUiActionActivation,
    pub requirement: WorkspaceEffectRequirement,
}

impl CompatibilityUiActionDefinition {
    fn local() -> Self {
        Self {
            activation: CompatibilityUiActionActivation::ClientLocal,
            requirement: WorkspaceEffectRequirement::ClientLocal,
        }
    }

    fn inspectable(requirement: WorkspaceEffectRequirement) -> Self {
        Self {
            activation: CompatibilityUiActionActivation::Inspectable,
            requirement,
        }
    }

    fn gated(requirement: WorkspaceEffectRequirement) -> Self {
        let activation = if requirement == WorkspaceEffectRequirement::ClientLocal {
            CompatibilityUiActionActivation::ClientLocal
        } else {
            CompatibilityUiActionActivation::CapabilityGated
        };
        Self {
            activation,
            requirement,
        }
    }
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
            limitations: Vec::new(),
            implementations: availability.implementations,
        }
    }
}

impl CompatibilityUiActionAvailability {
    fn for_definition(
        compatibility: &WorkspaceCompatibilityState,
        definition: &CompatibilityUiActionDefinition,
    ) -> Self {
        let mut presentation: Self = compatibility.availability(&definition.requirement).into();
        if let Some(authority) = compatibility.authority() {
            presentation.limitations = presentation
                .implementations
                .iter()
                .filter_map(|(id, _)| authority.snapshot.capability(*id))
                .flat_map(|record| match &record.state {
                    CapabilityState::AvailableWithLimitations { limitations, .. } => {
                        limitations.clone()
                    }
                    _ => Vec::new(),
                })
                .collect();
        }
        if definition.activation == CompatibilityUiActionActivation::Inspectable {
            presentation.enabled = true;
        }
        presentation
    }

    pub fn exact_reason(&self) -> Option<String> {
        (!self.reasons.is_empty()).then(|| self.reasons.join(" "))
    }
}

/// Every Navigator destination stays reachable. Its environment summary is
/// still projected so normal rendering can disclose degraded support.
pub fn compatibility_ui_destination_action_definition(
    screen: Screen,
) -> CompatibilityUiActionDefinition {
    compatibility_ui_workspace_destination_action_definition(workspace_screen_destination(screen))
}

pub fn compatibility_ui_workspace_destination_action_definition(
    destination: crate::WorkspaceDestination,
) -> CompatibilityUiActionDefinition {
    CompatibilityUiActionDefinition::inspectable(workspace_destination_requirement(destination))
}

/// Closed command-palette classification. Adding a `CommandId` requires an
/// explicit choice here before the model compiles.
pub fn compatibility_ui_command_action_definition(
    command: CommandId,
) -> CompatibilityUiActionDefinition {
    use CapabilityId as Id;
    match command {
        CommandId::BuildImage | CommandId::SelectImage | CommandId::BuildSelectedRecipe => {
            CompatibilityUiActionDefinition::gated(WorkspaceEffectRequirement::one(
                Id::BitBakeBuild,
            ))
        }
        CommandId::EditBbmask
        | CommandId::ChooseTheme
        | CommandId::FocusNavigator
        | CommandId::FocusWorkspace
        | CommandId::FocusInspector
        | CommandId::PreviousSubfocus
        | CommandId::NextSubfocus
        | CommandId::TogglePaneZoom
        | CommandId::ScrollFirst
        | CommandId::ScrollLast
        | CommandId::OpenDashboard
        | CommandId::OpenLogs
        | CommandId::OpenErrors
        | CommandId::OpenCompatibility
        | CommandId::OpenSettings
        | CommandId::OpenHelp => CompatibilityUiActionDefinition::local(),
        CommandId::OpenLayers => compatibility_ui_destination_action_definition(Screen::Layers),
        CommandId::OpenRecipes => compatibility_ui_destination_action_definition(Screen::Recipes),
        CommandId::OpenImages => compatibility_ui_destination_action_definition(Screen::Images),
        CommandId::OpenTasks => compatibility_ui_destination_action_definition(Screen::Tasks),
        CommandId::OpenConfiguration => {
            compatibility_ui_destination_action_definition(Screen::Configuration)
        }
        CommandId::OpenRawMode => compatibility_ui_destination_action_definition(Screen::RawMode),
    }
}

/// Contextual workspace operations are represented by their typed effect. The
/// exhaustive effect classifier remains the sole behavior-to-capability map.
pub fn compatibility_ui_effect_action_definition(
    effect: &Effect,
) -> CompatibilityUiActionDefinition {
    CompatibilityUiActionDefinition::gated(workspace_effect_requirement(effect))
}

/// Dialog confirmation uses the same exhaustive dialog classifier as runtime
/// revalidation, preventing rendering and launch authorization from diverging.
pub fn compatibility_ui_dialog_action_definition(
    dialog: &Dialog,
) -> CompatibilityUiActionDefinition {
    CompatibilityUiActionDefinition::gated(workspace_dialog_requirement(dialog))
}

pub fn compatibility_ui_action_definition_availability(
    compatibility: &WorkspaceCompatibilityState,
    definition: &CompatibilityUiActionDefinition,
) -> CompatibilityUiActionAvailability {
    CompatibilityUiActionAvailability::for_definition(compatibility, definition)
}

pub fn compatibility_ui_destination_action_availability(
    compatibility: &WorkspaceCompatibilityState,
    screen: Screen,
) -> CompatibilityUiActionAvailability {
    compatibility_ui_action_definition_availability(
        compatibility,
        &compatibility_ui_destination_action_definition(screen),
    )
}

pub fn compatibility_ui_workspace_destination_action_availability(
    compatibility: &WorkspaceCompatibilityState,
    destination: crate::WorkspaceDestination,
) -> CompatibilityUiActionAvailability {
    compatibility_ui_action_definition_availability(
        compatibility,
        &compatibility_ui_workspace_destination_action_definition(destination),
    )
}

pub fn compatibility_ui_command_action_availability(
    compatibility: &WorkspaceCompatibilityState,
    command: CommandId,
) -> CompatibilityUiActionAvailability {
    compatibility_ui_action_definition_availability(
        compatibility,
        &compatibility_ui_command_action_definition(command),
    )
}

pub fn compatibility_ui_effect_action_availability(
    compatibility: &WorkspaceCompatibilityState,
    effect: &Effect,
) -> CompatibilityUiActionAvailability {
    compatibility_ui_action_definition_availability(
        compatibility,
        &compatibility_ui_effect_action_definition(effect),
    )
}

pub fn compatibility_ui_dialog_action_availability(
    compatibility: &WorkspaceCompatibilityState,
    dialog: &Dialog,
) -> CompatibilityUiActionAvailability {
    compatibility_ui_action_definition_availability(
        compatibility,
        &compatibility_ui_dialog_action_definition(dialog),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiWorkspaceActionDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub requirement: WorkspaceEffectRequirement,
}

impl CompatibilityUiWorkspaceActionDefinition {
    fn capability(
        id: &'static str,
        label: &'static str,
        shortcut: &'static str,
        capability: CapabilityId,
    ) -> Self {
        Self {
            id,
            label,
            shortcut,
            requirement: WorkspaceEffectRequirement::one(capability),
        }
    }

    fn all(
        id: &'static str,
        label: &'static str,
        shortcut: &'static str,
        capabilities: &[CapabilityId],
    ) -> Self {
        Self {
            id,
            label,
            shortcut,
            requirement: WorkspaceEffectRequirement::all(capabilities),
        }
    }

    fn alternatives(
        id: &'static str,
        label: &'static str,
        shortcut: &'static str,
        capabilities: &[CapabilityId],
    ) -> Self {
        Self {
            id,
            label,
            shortcut,
            requirement: WorkspaceEffectRequirement::all_and_any(&[], capabilities),
        }
    }

    fn local(id: &'static str, label: &'static str, shortcut: &'static str) -> Self {
        Self {
            id,
            label,
            shortcut,
            requirement: WorkspaceEffectRequirement::ClientLocal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityUiWorkspaceActionPresentation {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub description: String,
    pub menu_path: Vec<&'static str>,
    pub safety: crate::OperatorActionSafety,
    pub footer_priority: u8,
    pub help_group: crate::OperatorActionHelpGroup,
    pub availability: CompatibilityUiActionAvailability,
}

/// Closed contextual action inventory. It represents the useful operations
/// users can launch from each workspace; renderers consume this data and never
/// embed capability IDs or release policy.
pub(crate) fn compatibility_ui_workspace_action_seeds(
    destination: crate::WorkspaceDestination,
) -> Vec<CompatibilityUiWorkspaceActionDefinition> {
    use crate::WorkspaceDestination as Destination;
    use CapabilityId as Id;
    use CompatibilityUiWorkspaceActionDefinition as Action;
    match destination {
        Destination::Dashboard => vec![
            Action::capability("dashboard.build", "Build image", "B", Id::BitBakeBuild),
            Action::capability(
                "dashboard.cancel",
                "Cancel active build",
                "c",
                Id::BitBakeCancellation,
            ),
        ],
        Destination::Recipes => vec![
            Action::capability(
                "recipes.metadata",
                "Refresh metadata",
                "r",
                Id::BitBakeRecipeMetadata,
            ),
            Action::capability(
                "recipes.dependencies",
                "Dependencies",
                "A",
                Id::BitBakeRecipeDependencies,
            ),
            Action::capability(
                "recipes.build",
                "Build selected recipe",
                "b",
                Id::BitBakeBuild,
            ),
            Action::all(
                "recipes.force_task",
                "Force selected task",
                "f",
                &[Id::BitBakeBuild, Id::BitBakeForceTask],
            ),
            Action::capability(
                "recipes.signatures",
                "Inspect signatures",
                "z",
                Id::BitBakeDumpSig,
            ),
            Action::all(
                "recipes.cve",
                "Run CVE check",
                "V",
                &[Id::BitBakeBuild, Id::CveCheck],
            ),
            Action::all(
                "recipes.spdx",
                "Create SPDX",
                "X",
                &[Id::BitBakeBuild, Id::SpdxCreate],
            ),
            Action::capability(
                "recipes.devtool_modify",
                "Devtool modify",
                "d",
                Id::DevtoolModify,
            ),
            Action::capability(
                "recipes.devtool_update",
                "Devtool update-recipe",
                "u",
                Id::DevtoolUpdateRecipe,
            ),
            Action::capability(
                "recipes.devtool_finish",
                "Devtool finish",
                "F",
                Id::DevtoolFinish,
            ),
            Action::capability(
                "recipes.devtool_deploy",
                "Devtool deploy-target",
                "P",
                Id::DevtoolDeployTarget,
            ),
            Action::capability(
                "recipes.devtool_reset",
                "Devtool reset",
                "D",
                Id::DevtoolReset,
            ),
            Action::local("recipes.open", "Open provider/log/source", "Enter/o/e"),
        ],
        Destination::Layers => vec![
            Action::alternatives(
                "layers.inventory",
                "Refresh layer inventory",
                "r",
                &[Id::BitBakeLayerInventory, Id::BitBakeLayersShowLayers],
            ),
            Action::capability(
                "layers.relationships",
                "Layer relationships",
                "R",
                Id::BitBakeLayerRelationships,
            ),
            Action::capability(
                "layers.create",
                "Create layer",
                "c",
                Id::BitBakeLayersCreateLayer,
            ),
            Action::capability("layers.add", "Add layer", "a", Id::BitBakeLayersAddLayer),
            Action::capability(
                "layers.remove",
                "Remove layer",
                "x",
                Id::BitBakeLayersRemoveLayer,
            ),
            Action::local("layers.open", "Browse/edit configured layer", "Enter/e/o"),
        ],
        Destination::Configuration => vec![
            Action::capability(
                "configuration.getvar",
                "Refresh effective variables",
                "r",
                Id::BitBakeGetVar,
            ),
            Action::local(
                "configuration.inspect",
                "Inspect/copy/source",
                "Enter/C/U/o",
            ),
            Action::local("configuration.edit", "Edit local assignment", "E/x"),
        ],
        Destination::Tasks => vec![
            Action::capability(
                "tasks.inventory",
                "Inspect task inventory",
                "F2",
                Id::BitBakeTaskList,
            ),
            Action::capability("tasks.build", "Build options", "B", Id::BitBakeBuild),
            Action::capability(
                "tasks.cancel",
                "Cancel active build",
                "c",
                Id::BitBakeCancellation,
            ),
            Action::local("tasks.logs", "Open Logs", "l"),
            Action::local("tasks.history", "Build History", "h"),
        ],
        Destination::BuildHistory => vec![Action::local(
            "build_history.inspect",
            "Inspect retained build record",
            "Enter",
        )],
        Destination::Logs => vec![Action::local(
            "logs.inspect",
            "Filter/bookmark/copy/export/open retained logs",
            "/m/[/]/C/E/o",
        )],
        Destination::Errors => vec![Action::local(
            "errors.inspect",
            "Inspect retained diagnostic source",
            "Enter/o",
        )],
        Destination::Dependencies => vec![
            Action::alternatives(
                "dependencies.refresh",
                "Refresh dependency graph",
                "r",
                &[Id::BitBakeRecipeDependencies, Id::BitBakeDependencyGraph],
            ),
            Action::local("dependencies.open", "Open provider/task log", "Enter/o/L"),
        ],
        Destination::Signatures => vec![
            Action::capability(
                "signatures.dump",
                "Dump task signature",
                "r",
                Id::BitBakeDumpSig,
            ),
            Action::capability(
                "signatures.compare",
                "Compare signatures",
                "c",
                Id::BitBakeDiffSigs,
            ),
            Action::local("signatures.open", "Open provider", "e"),
        ],
        Destination::Packages => vec![
            Action::all(
                "packages.inventory",
                "Refresh package inventory",
                "R",
                &[Id::PkgDataGenerated, Id::PkgDataListPackages],
            ),
            Action::all(
                "packages.detail",
                "Load package details",
                "Enter",
                &[
                    Id::PkgDataGenerated,
                    Id::PkgDataPackageInfo,
                    Id::PkgDataListPackageFiles,
                    Id::PkgDataReadValue,
                ],
            ),
            Action::local(
                "packages.navigate",
                "Navigate/open package evidence",
                "[/]/d/u/o/e",
            ),
            Action::local("packages.cancel", "Cancel owned package scan", "c"),
        ],
        Destination::Images => vec![
            Action::capability(
                "images.build",
                "Build selected image",
                "b",
                Id::BitBakeBuild,
            ),
            Action::capability("images.qemu", "Launch QEMU", "Q", Id::RunQemu),
            Action::capability("images.wic", "Create Wic image", "W", Id::WicCreate),
            Action::local("images.device_write", "Write selected local device", "D"),
            Action::local("images.artifacts", "Scan/open deployed artifacts", "R/o/O"),
            Action::local("images.cancel", "Cancel owned image operation", "x/c"),
        ],
        Destination::Sdk => vec![
            Action::all(
                "sdk.standard",
                "Populate standard SDK",
                "s",
                &[Id::BitBakeBuild, Id::SdkPopulate],
            ),
            Action::all(
                "sdk.extensible",
                "Populate extensible SDK",
                "E",
                &[Id::BitBakeBuild, Id::SdkExtensible],
            ),
            Action::all(
                "sdk.testsdk",
                "Run testsdk",
                "t",
                &[Id::BitBakeBuild, Id::TestSdk],
            ),
            Action::all(
                "sdk.testsdkext",
                "Run testsdkext",
                "T",
                &[Id::BitBakeBuild, Id::TestSdkExtensible],
            ),
            Action::capability("sdk.publish", "Publish SDK", "P", Id::SdkPublish),
            Action::capability("sdk.native", "Run native SDK tool", "n", Id::SdkNativeTools),
            Action::local("sdk.artifacts", "Scan/open SDK artifacts", "R/o"),
            Action::local("sdk.cancel", "Cancel owned SDK operation", "c"),
        ],
        Destination::Testing => vec![
            Action::capability(
                "testing.oe_selftest",
                "Run oe-selftest",
                "r",
                Id::OeSelftest,
            ),
            Action::capability(
                "testing.bitbake_selftest",
                "Run BitBake selftest",
                "r",
                Id::BitBakeSelftest,
            ),
            Action::all(
                "testing.testimage",
                "Run testimage",
                "r",
                &[Id::BitBakeBuild, Id::TestImage],
            ),
            Action::all(
                "testing.testsdk",
                "Run testsdk",
                "r",
                &[Id::BitBakeBuild, Id::TestSdk],
            ),
            Action::all(
                "testing.testsdkext",
                "Run testsdkext",
                "r",
                &[Id::BitBakeBuild, Id::TestSdkExtensible],
            ),
            Action::all(
                "testing.ptest",
                "Run ptest",
                "r",
                &[Id::BitBakeBuild, Id::Ptest],
            ),
            Action::capability("testing.compare", "Compare results", "c", Id::ResultTool),
            Action::local("testing.import", "Import/open/export results", "I/o/J"),
            Action::local("testing.cancel", "Cancel owned test operation", "x"),
        ],
        Destination::Security => vec![
            Action::all(
                "security.cve",
                "Run CVE check",
                "V",
                &[Id::BitBakeBuild, Id::CveCheck],
            ),
            Action::all(
                "security.spdx",
                "Create SPDX/SBOM",
                "X",
                &[Id::BitBakeBuild, Id::SpdxCreate],
            ),
            Action::all(
                "security.package_map",
                "Map package data",
                "M",
                &[Id::PkgDataGenerated, Id::PkgDataLookupPackage],
            ),
            Action::local(
                "security.reports",
                "Import/open security evidence",
                "I/R/o/e/v",
            ),
            Action::local("security.cancel", "Cancel owned security operation", "c"),
        ],
        Destination::Qa => vec![
            Action::all(
                "qa.recipe",
                "Run recipe QA task",
                "r",
                &[Id::BitBakeBuild, Id::QaTask],
            ),
            Action::capability(
                "qa.layer",
                "Run layer compatibility check",
                "r",
                Id::YoctoCheckLayer,
            ),
            Action::local("qa.reports", "Import/open QA evidence", "I/R/o/e/l"),
            Action::local("qa.cancel", "Cancel owned QA operation", "c"),
        ],
        Destination::RawMode => vec![Action::local(
            "raw.inspect",
            "Inspect Raw command catalog",
            "Enter",
        )],
        Destination::Devtool => vec![
            Action::capability(
                "devtool.status",
                "Refresh Devtool status",
                "r",
                Id::DevtoolStatus,
            ),
            Action::capability("devtool.edit", "Edit recipe", "e", Id::DevtoolEditRecipe),
            Action::capability("devtool.modify", "Modify recipe", "d", Id::DevtoolModify),
            Action::capability(
                "devtool.update",
                "Update recipe",
                "u",
                Id::DevtoolUpdateRecipe,
            ),
            Action::capability("devtool.finish", "Finish recipe", "F", Id::DevtoolFinish),
            Action::capability(
                "devtool.deploy",
                "Deploy target",
                "P",
                Id::DevtoolDeployTarget,
            ),
            Action::capability(
                "devtool.undeploy",
                "Undeploy target",
                "P",
                Id::DevtoolUndeployTarget,
            ),
            Action::capability("devtool.reset", "Reset recipe", "D", Id::DevtoolReset),
            Action::capability("devtool.upgrade", "Upgrade recipe", "U", Id::DevtoolUpgrade),
        ],
        Destination::QemuWic => vec![
            Action::capability("qemu_wic.qemu", "Launch QEMU", "Q", Id::RunQemu),
            Action::capability("qemu_wic.wic", "Create Wic image", "W", Id::WicCreate),
            Action::local("qemu_wic.write", "Write local block device", "D"),
            Action::local("qemu_wic.cancel", "Cancel owned runtime", "x"),
        ],
        Destination::Maintenance => vec![
            Action::capability(
                "maintenance.readiness",
                "Check sstate readiness",
                "c",
                Id::SstateReadiness,
            ),
            Action::capability(
                "maintenance.cleanup",
                "Clean shared state",
                "d",
                Id::SstateCleanup,
            ),
            Action::capability(
                "maintenance.prserv",
                "Manage PR service",
                "e/m",
                Id::PrservManagement,
            ),
            Action::capability(
                "maintenance.locked",
                "Generate locked signatures",
                "l",
                Id::LockedSignatures,
            ),
            Action::capability(
                "maintenance.history",
                "Compare build history",
                "h",
                Id::BuildHistoryCompare,
            ),
            Action::capability(
                "maintenance.archive",
                "Archive repository",
                "a",
                Id::GitArchive,
            ),
            Action::local(
                "maintenance.cancel",
                "Cancel owned maintenance operation",
                "x",
            ),
            Action::local("maintenance.evidence", "Open retained evidence", "o"),
        ],
        Destination::TerminalSessions => vec![
            Action::capability("terminal.devshell", "Open devshell", "s", Id::DevShell),
            Action::capability(
                "terminal.menuconfig",
                "Open menuconfig",
                "m",
                Id::MenuConfig,
            ),
            Action::capability(
                "terminal.server",
                "Attach BitBake server",
                "Enter",
                Id::BitBakeServerSocket,
            ),
            Action::local("terminal.cancel", "Detach/cancel owned terminal", "Esc/c"),
        ],
        Destination::ProjectProfiles
        | Destination::BuildEnvironment
        | Destination::Compatibility
        | Destination::Settings
        | Destination::Help => Vec::new(),
    }
}

/// Backward-compatible compatibility projection sourced from the canonical
/// operator action catalog.
pub fn compatibility_ui_workspace_action_definitions(
    destination: crate::WorkspaceDestination,
) -> Vec<CompatibilityUiWorkspaceActionDefinition> {
    crate::workspace_operator_action_definitions(destination)
        .into_iter()
        .map(|action| CompatibilityUiWorkspaceActionDefinition {
            id: action.id.as_str(),
            label: action.label,
            shortcut: action.shortcut,
            requirement: action.requirement,
        })
        .collect()
}

pub fn compatibility_ui_workspace_action_presentations(
    compatibility: &WorkspaceCompatibilityState,
    destination: crate::WorkspaceDestination,
) -> Vec<CompatibilityUiWorkspaceActionPresentation> {
    crate::workspace_operator_action_definitions(destination)
        .into_iter()
        .map(|definition| {
            let availability = compatibility_ui_action_definition_availability(
                compatibility,
                &CompatibilityUiActionDefinition::gated(definition.requirement.clone()),
            );
            CompatibilityUiWorkspaceActionPresentation {
                id: definition.id.as_str(),
                label: definition.label,
                shortcut: definition.shortcut,
                description: definition.description,
                menu_path: definition.menu_path,
                safety: definition.safety,
                footer_priority: definition.footer_priority,
                help_group: definition.help_group,
                availability,
            }
        })
        .collect()
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
    fn compatibility_dynamic_model_filter_search_and_selection_reconcile_by_stable_id() {
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

    #[test]
    fn compatibility_ui_action_catalog_classifies_every_destination_and_command() {
        let screens = [
            Screen::Dashboard,
            Screen::Tasks,
            Screen::BuildHistory,
            Screen::Dependencies,
            Screen::Signatures,
            Screen::LayerRelationships,
            Screen::Recipes,
            Screen::Packages,
            Screen::Images,
            Screen::Sdk,
            Screen::Testing,
            Screen::Security,
            Screen::Qa,
            Screen::RawMode,
            Screen::Layers,
            Screen::Configuration,
            Screen::Bbmask,
            Screen::Maintenance,
            Screen::Logs,
            Screen::Errors,
            Screen::Help,
            Screen::BuildEnvironment,
            Screen::Compatibility,
            Screen::Settings,
        ];
        for screen in screens {
            assert_eq!(
                compatibility_ui_destination_action_definition(screen).activation,
                CompatibilityUiActionActivation::Inspectable
            );
        }

        let commands = [
            CommandId::BuildImage,
            CommandId::SelectImage,
            CommandId::BuildSelectedRecipe,
            CommandId::EditBbmask,
            CommandId::OpenDashboard,
            CommandId::OpenLayers,
            CommandId::OpenRecipes,
            CommandId::OpenImages,
            CommandId::OpenTasks,
            CommandId::OpenLogs,
            CommandId::OpenErrors,
            CommandId::OpenConfiguration,
            CommandId::OpenRawMode,
            CommandId::OpenCompatibility,
            CommandId::OpenSettings,
            CommandId::ChooseTheme,
            CommandId::OpenHelp,
        ];
        for command in commands {
            let _ = compatibility_ui_command_action_definition(command);
        }
        assert_eq!(
            compatibility_ui_command_action_definition(CommandId::BuildImage).activation,
            CompatibilityUiActionActivation::CapabilityGated
        );
        assert_eq!(
            compatibility_ui_command_action_definition(CommandId::OpenLayers).activation,
            CompatibilityUiActionActivation::Inspectable
        );
        assert_eq!(
            compatibility_ui_command_action_definition(CommandId::ChooseTheme).activation,
            CompatibilityUiActionActivation::ClientLocal
        );
    }

    #[test]
    fn compatibility_ui_action_catalog_preserves_inspection_gating_and_exact_fallback() {
        let absent = WorkspaceCompatibilityState::default();
        let layers = compatibility_ui_destination_action_availability(&absent, Screen::Layers);
        assert!(layers.enabled);
        assert_eq!(layers.state, WorkspaceAvailabilityState::Unknown);
        assert!(
            layers
                .exact_reason()
                .unwrap()
                .contains("current environment capability snapshot")
        );

        let build = compatibility_ui_command_action_availability(&absent, CommandId::BuildImage);
        assert!(!build.enabled);
        assert_eq!(build.state, WorkspaceAvailabilityState::Unknown);

        let local = compatibility_ui_command_action_availability(&absent, CommandId::ChooseTheme);
        assert!(local.enabled);
        assert_eq!(local.state, WorkspaceAvailabilityState::Available);
        assert!(local.exact_reason().is_none());

        let current = state_with(authority(7));
        let configuration =
            compatibility_ui_destination_action_availability(&current, Screen::Configuration);
        assert!(configuration.enabled);
        assert_eq!(
            configuration.state,
            WorkspaceAvailabilityState::AvailableWithLimitations
        );
        assert_eq!(
            configuration.implementations,
            [(
                CapabilityId::BitBakeGetVar,
                "bitbake.getvar.environment-fallback".into()
            )]
        );
        assert_eq!(
            configuration.exact_reason().as_deref(),
            Some("Native getvar is absent; the environment dump fallback is selected.")
        );
    }

    #[test]
    fn compatibility_ui_action_catalog_reuses_effect_and_dialog_authority() {
        let absent = WorkspaceCompatibilityState::default();
        let build = Effect::Start(crate::BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        });
        let build_action = compatibility_ui_effect_action_availability(&absent, &build);
        assert!(!build_action.enabled);
        assert_eq!(build_action.state, WorkspaceAvailabilityState::Unknown);

        let local_effect = Effect::CopyToClipboard("value".into());
        let local_action = compatibility_ui_effect_action_availability(&absent, &local_effect);
        assert!(local_action.enabled);
        assert_eq!(local_action.state, WorkspaceAvailabilityState::Available);

        let gated_dialog =
            compatibility_ui_dialog_action_availability(&absent, &Dialog::BuildOptions);
        assert!(!gated_dialog.enabled);
        assert_eq!(gated_dialog.state, WorkspaceAvailabilityState::Unknown);

        let local_dialog =
            compatibility_ui_dialog_action_availability(&absent, &Dialog::QuitConfirmation);
        assert!(local_dialog.enabled);
        assert_eq!(local_dialog.state, WorkspaceAvailabilityState::Available);
    }

    #[test]
    fn compatibility_ui_workspace_actions_catalog_is_unique_closed_and_probe_free() {
        let mut ids = std::collections::BTreeSet::new();
        for destination in crate::WorkspaceDestination::ALL {
            for action in compatibility_ui_workspace_action_definitions(destination) {
                assert!(ids.insert(action.id), "duplicate action ID {}", action.id);
                assert!(!action.label.is_empty());
                assert!(!action.shortcut.is_empty());
                assert!(!matches!(
                    action.requirement,
                    WorkspaceEffectRequirement::DaemonProbe { .. }
                ));
            }
        }
        for expected in [
            "dashboard.build",
            "recipes.devtool_modify",
            "layers.remove",
            "configuration.getvar",
            "tasks.cancel",
            "dependencies.refresh",
            "signatures.compare",
            "packages.detail",
            "images.qemu",
            "sdk.extensible",
            "testing.oe_selftest",
            "security.spdx",
            "qa.layer",
            "devtool.upgrade",
            "qemu_wic.wic",
            "maintenance.cleanup",
            "terminal.menuconfig",
        ] {
            assert!(
                ids.contains(expected),
                "missing contextual action {expected}"
            );
        }
    }

    #[test]
    fn compatibility_ui_workspace_actions_project_all_states_without_local_inference() {
        let compatibility = state_with(authority(9));
        let configuration = compatibility_ui_workspace_action_presentations(
            &compatibility,
            crate::WorkspaceDestination::Configuration,
        );
        let getvar = configuration
            .iter()
            .find(|action| action.id == "configuration.getvar")
            .unwrap();
        assert!(getvar.availability.enabled);
        assert_eq!(
            getvar.availability.state,
            WorkspaceAvailabilityState::AvailableWithLimitations
        );
        assert!(getvar.availability.reasons[0].contains("fallback"));
        let local = configuration
            .iter()
            .find(|action| action.id == "configuration.inspect")
            .unwrap();
        assert!(local.availability.enabled);
        assert_eq!(
            local.availability.state,
            WorkspaceAvailabilityState::Available
        );

        let devtool = compatibility_ui_workspace_action_presentations(
            &compatibility,
            crate::WorkspaceDestination::Devtool,
        );
        let upgrade = devtool
            .iter()
            .find(|action| action.id == "devtool.upgrade")
            .unwrap();
        assert!(!upgrade.availability.enabled);
        assert_eq!(
            upgrade.availability.state,
            WorkspaceAvailabilityState::Unavailable
        );
        assert_eq!(
            upgrade.availability.exact_reason().as_deref(),
            Some("Current Devtool does not expose the upgrade subcommand.")
        );
        let absent = compatibility_ui_workspace_action_presentations(
            &WorkspaceCompatibilityState::default(),
            crate::WorkspaceDestination::Images,
        );
        assert_eq!(
            absent
                .iter()
                .find(|action| action.id == "images.qemu")
                .unwrap()
                .availability
                .state,
            WorkspaceAvailabilityState::Unknown
        );
        assert!(
            absent
                .iter()
                .find(|action| action.id == "images.device_write")
                .unwrap()
                .availability
                .enabled
        );
    }
}
