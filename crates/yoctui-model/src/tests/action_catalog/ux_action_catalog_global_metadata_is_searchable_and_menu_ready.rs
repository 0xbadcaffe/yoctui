use super::*;

#[test]
fn ux_action_catalog_global_metadata_is_searchable_and_menu_ready() {
    for command in GLOBAL_COMMANDS {
        let action = global_operator_action_definition(command);
        assert!(matches!(action.target, OperatorActionTarget::Command(id) if id == command));
        assert!(action.menu_path.len() >= 2);
        assert!(!action.description.is_empty());
        assert!(!action.palette_keywords.is_empty());
        assert!(action.footer_priority <= 100);
    }
}
