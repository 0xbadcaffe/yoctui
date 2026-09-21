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

