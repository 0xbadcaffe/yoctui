use super::*;

#[test]
fn raw_catalog_model_rejects_missing_or_duplicate_capability_policy() {
    let mut catalog = valid_catalog();
    let RawExecutionPolicy::Executable { template } = &mut catalog.commands[0].execution else {
        unreachable!()
    };
    template.capabilities = RawCapabilityRequirement::All {
        capabilities: Vec::new(),
    };
    assert_eq!(
        catalog.validate(),
        Err(RawCatalogError::InvalidCapabilityRequirement(id(
            "task-control.run"
        )))
    );
}
