pub fn workspace_operator_action_definitions(
    destination: WorkspaceDestination,
) -> Vec<OperatorActionDefinition> {
    compatibility_ui_workspace_action_seeds(destination)
        .into_iter()
        .enumerate()
        .map(|(index, seed)| workspace_definition(destination, seed, index))
        .collect()
}

fn workspace_definition(
    destination: WorkspaceDestination,
    seed: CompatibilityUiWorkspaceActionDefinition,
    index: usize,
) -> OperatorActionDefinition {
    let mut aliases = Vec::new();
    if seed.shortcut.starts_with('/') {
        aliases.push("/");
    }
    aliases.extend(seed.shortcut.split('/').filter(|value| !value.is_empty()));
    let mut keywords = seed.id.split(['.', '_']).collect::<Vec<_>>();
    keywords.extend(
        seed.label
            .split_ascii_whitespace()
            .map(|value| value.trim_matches(|character: char| !character.is_ascii_alphanumeric())),
    );
    keywords.retain(|value| !value.is_empty());
    keywords.sort_unstable();
    keywords.dedup();
    OperatorActionDefinition {
        id: OperatorActionId::new(seed.id),
        scope: OperatorActionScope::Workspace(destination),
        menu_path: vec![destination.label(), seed.label],
        label: seed.label,
        description: format!("{} in the {} workspace", seed.label, destination.label()),
        aliases: aliases.clone(),
        palette_keywords: keywords,
        shortcut: seed.shortcut,
        default_bindings: aliases,
        requirement: seed.requirement,
        local_requirement: OperatorActionLocalRequirement::None,
        safety: workspace_safety(seed.id),
        footer_priority: 80_u8.saturating_sub(u8::try_from(index).unwrap_or(u8::MAX)),
        help_group: workspace_help_group(seed.id),
        target: OperatorActionTarget::Workspace {
            destination,
            legacy_id: seed.id,
        },
    }
}

fn workspace_safety(id: &str) -> OperatorActionSafety {
    if [
        ".remove",
        ".reset",
        ".cleanup",
        ".write",
        ".undeploy",
        ".cancel",
    ]
    .iter()
    .any(|fragment| id.contains(fragment))
    {
        OperatorActionSafety::DestructiveConfirmation
    } else if [
        ".build",
        ".force_task",
        ".create",
        ".deploy",
        ".finish",
        ".update",
        ".cve",
        ".spdx",
        ".qemu",
        ".wic",
        "sdk.",
        "testing.",
        "qa.recipe",
        "maintenance.archive",
        "maintenance.locked",
    ]
    .iter()
    .any(|fragment| id.contains(fragment))
    {
        OperatorActionSafety::ConfirmationRequired
    } else {
        OperatorActionSafety::ReadOnly
    }
}

fn workspace_help_group(id: &str) -> OperatorActionHelpGroup {
    if id.contains("build") || id.contains("force_task") {
        OperatorActionHelpGroup::Build
    } else if id.contains("inspect") || id.contains("open") || id.contains("evidence") {
        OperatorActionHelpGroup::Inspect
    } else {
        OperatorActionHelpGroup::Operate
    }
}
