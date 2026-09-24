use super::*;

#[test]
fn ux_action_catalog_is_unique_complete_and_safe() {
    validate_operator_action_catalog().unwrap();
    let catalog = operator_action_catalog();
    assert_eq!(catalog.len(), 164, "37 global plus 127 workspace actions");
    assert!(
        catalog
            .iter()
            .all(|action| !action.palette_keywords.is_empty())
    );
    assert_eq!(
        catalog
            .iter()
            .find(|action| action.id.as_str() == "build.image")
            .unwrap()
            .safety,
        OperatorActionSafety::ConfirmationRequired
    );
    assert_eq!(
        catalog
            .iter()
            .find(|action| action.id.as_str() == "layers.remove")
            .unwrap()
            .safety,
        OperatorActionSafety::DestructiveConfirmation
    );
}
