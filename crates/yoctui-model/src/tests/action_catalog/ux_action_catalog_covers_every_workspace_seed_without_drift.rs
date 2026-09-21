use super::*;

#[test]
fn ux_action_catalog_covers_every_workspace_seed_without_drift() {
    for destination in WorkspaceDestination::ALL {
        let seeds = compatibility_ui_workspace_action_seeds(destination);
        let definitions = workspace_operator_action_definitions(destination);
        assert_eq!(seeds.len(), definitions.len(), "{destination:?}");
        for (seed, definition) in seeds.iter().zip(&definitions) {
            assert_eq!(seed.id, definition.id.as_str());
            assert_eq!(seed.label, definition.label);
            assert_eq!(seed.shortcut, definition.shortcut);
            assert!(!definition.default_bindings.is_empty());
            assert_eq!(seed.requirement, definition.requirement);
        }
    }
}
