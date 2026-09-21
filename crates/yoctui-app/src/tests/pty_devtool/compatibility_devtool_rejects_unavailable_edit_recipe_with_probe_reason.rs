use super::*;

#[test]
fn compatibility_devtool_rejects_unavailable_edit_recipe_with_probe_reason() {
    let (root, mut router, status) = fixture();
    let record = router
        .compatibility
        .snapshot
        .capabilities
        .iter_mut()
        .find(|record| record.id == CapabilityId::DevtoolEditRecipe)
        .unwrap();
    record.state = CapabilityState::Unavailable {
        reason: yoctui_model::CapabilityReason::new(
            "devtool.subcommand_missing",
            "Current Devtool does not expose the edit-recipe subcommand.",
            Some("Required capability: devtool.edit_recipe".into()),
        )
        .unwrap(),
    };
    router
        .compatibility
        .implementations
        .remove(&CapabilityId::DevtoolEditRecipe);
    assert!(matches!(
        router.preview(&status, PtyDevtoolAction::EditRecipe),
        Err(PtyDevtoolError::Compatibility(
            DevtoolCompatibilityError::Unavailable { reason, .. }
        )) if reason.contains("does not expose the edit-recipe")
    ));
    fs::remove_dir_all(root).unwrap();
}
