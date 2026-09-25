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
        | CommandId::OpenOnboarding
        | CommandId::OpenDashboard
        | CommandId::OpenLogs
        | CommandId::OpenErrors
        | CommandId::OpenCompatibility
        | CommandId::OpenSettings
        | CommandId::OpenHelp
        | CommandId::OpenAbout => CompatibilityUiActionDefinition::local(),
        CommandId::OpenLayers => compatibility_ui_destination_action_definition(Screen::Layers),
        CommandId::OpenRecipes => compatibility_ui_destination_action_definition(Screen::Recipes),
        CommandId::OpenPackages => compatibility_ui_destination_action_definition(Screen::Packages),
        CommandId::OpenImages => compatibility_ui_destination_action_definition(Screen::Images),
        CommandId::OpenSdk => compatibility_ui_destination_action_definition(Screen::Sdk),
        CommandId::OpenDependencies => {
            compatibility_ui_destination_action_definition(Screen::Dependencies)
        }
        CommandId::OpenTesting => compatibility_ui_destination_action_definition(Screen::Testing),
        CommandId::OpenSecurity => compatibility_ui_destination_action_definition(Screen::Security),
        CommandId::OpenQa => compatibility_ui_destination_action_definition(Screen::Qa),
        CommandId::OpenTasks => compatibility_ui_destination_action_definition(Screen::Tasks),
        CommandId::OpenConfiguration => {
            compatibility_ui_destination_action_definition(Screen::Configuration)
        }
        CommandId::OpenRawMode => compatibility_ui_destination_action_definition(Screen::RawMode),
        CommandId::OpenGitUi => CompatibilityUiActionDefinition::local(),
        CommandId::OpenDevtool(command) => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(command.capability()),
        ),
        CommandId::OpenBitBakeConfigBuild => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeConfigBuildListFragments),
        ),
        CommandId::OpenBitBakeLayersShowLayers => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersShowLayers),
        ),
        CommandId::OpenBitBakeLayersShowRecipes => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersShowRecipes),
        ),
        CommandId::OpenBitBakeLayersShowOverlayed => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersShowOverlayed),
        ),
        CommandId::OpenBitBakeLayersShowAppends => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersShowAppends),
        ),
        CommandId::OpenBitBakeLayersShowCrossDepends => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersShowCrossDepends),
        ),
        CommandId::OpenBitBakeLayersAddLayer => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersAddLayer),
        ),
        CommandId::OpenBitBakeLayersRemoveLayer => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersRemoveLayer),
        ),
        CommandId::OpenBitBakeLayersFlatten => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersFlatten),
        ),
        CommandId::OpenBitBakeLayersLayerIndexFetch => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersLayerIndexFetch),
        ),
        CommandId::OpenBitBakeLayersLayerIndexShowDepends => {
            CompatibilityUiActionDefinition::gated(WorkspaceEffectRequirement::one(
                Id::BitBakeLayersLayerIndexShowDepends,
            ))
        }
        CommandId::OpenBitBakeLayersCreateLayer => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersCreateLayer),
        ),
        CommandId::OpenBitBakeLayersShowMachines => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersShowMachines),
        ),
        CommandId::OpenBitBakeLayersSaveBuildConf => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersSaveBuildConf),
        ),
        CommandId::OpenBitBakeLayersCreateLayersSetup => CompatibilityUiActionDefinition::gated(
            WorkspaceEffectRequirement::one(Id::BitBakeLayersCreateLayersSetup),
        ),
        CommandId::OpenTerminalSessions => {
            compatibility_ui_destination_action_definition(Screen::TerminalSessions)
        }
        CommandId::OpenMaintenance => {
            compatibility_ui_destination_action_definition(Screen::Maintenance)
        }
        CommandId::OpenBuildEnvironment => {
            compatibility_ui_destination_action_definition(Screen::BuildEnvironment)
        }
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
