use super::*;

#[test]
fn raw_catalog_model_rejects_partial_records() {
    let mut catalog = valid_catalog();
    catalog.commands[0].description.clear();
    assert_eq!(
        catalog.validate(),
        Err(RawCatalogError::InvalidCommand(id("task-control.run")))
    );

    let mut reference_only = valid_catalog();
    reference_only.commands[0].execution = RawExecutionPolicy::ReferenceOnly {
        kind: RawReferenceKind::Conceptual,
        reason: "Documented concept only.".into(),
    };
    assert_eq!(
        reference_only.validate(),
        Err(RawCatalogError::InvalidExecutionPolicy(id(
            "task-control.run"
        )))
    );
}
