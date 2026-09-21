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
