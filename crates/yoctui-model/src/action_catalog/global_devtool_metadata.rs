fn devtool_utility_global_metadata(command: CommandId) -> Option<GlobalMetadata> {
    let CommandId::OpenDevtool(devtool) = command else {
        return None;
    };
    Some(GlobalMetadata {
        id: devtool.action_id(),
        scope: OperatorActionScope::Global,
        menu_path: vec!["Devtool", devtool.section(), devtool.label()],
        label: devtool.label(),
        description: devtool.description(),
        aliases: &[],
        keywords: &["devtool", "recipe", "workspace"],
        bindings: &[],
        local_requirement: OperatorActionLocalRequirement::WorkspaceLoaded,
        safety: devtool.safety(),
        footer_priority: 50,
        help_group: OperatorActionHelpGroup::Operate,
    })
}
